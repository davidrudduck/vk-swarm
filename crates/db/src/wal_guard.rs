use sqlx::ConnectOptions;
use sqlx::sqlite::{SqliteConnectOptions, SqliteConnection, SqliteJournalMode, SqliteSynchronous};
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;

/// Guard connection mode.
///
/// NOTE: production wiring uses `MapOnly` only (local-deployment `from_parts`).
/// `HoldRead`'s read-mark machinery (release/reacquire around checkpoints) is
/// currently dormant outside tests — it exists to pin a read transaction so an
/// external unlink cannot drop the WAL inode from under live readers, and is
/// retained for deployments that enable it. Unsupported on Windows (no
/// PowerShell environment bootstrap / WAL lock semantics untested there).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    MapOnly,
    HoldRead,
}

pub struct WalGuard {
    conn: SqliteConnection,
    options: SqliteConnectOptions,
    mode: Mode,
    holding_read_mark: bool,
}

pub(crate) fn options_for(db_path: &Path) -> Result<SqliteConnectOptions, sqlx::Error> {
    let opts = SqliteConnectOptions::from_str(&format!("sqlite://{}", db_path.to_string_lossy()))?
        .create_if_missing(false)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .busy_timeout(Duration::from_secs(5));
    Ok(opts)
}

impl WalGuard {
    /// The mode this guard was connected with.
    pub fn mode(&self) -> Mode {
        self.mode
    }

    pub async fn connect(db_path: &Path, mode: Mode) -> Result<Self, sqlx::Error> {
        let options = options_for(db_path)?;
        let mut conn = options.clone().connect().await?;

        crate::apply_performance_pragmas(&mut conn).await?;

        sqlx::query("SELECT count(*) FROM sqlite_master")
            .fetch_one(&mut conn)
            .await?;

        let mut holding_read_mark = false;
        if mode == Mode::HoldRead {
            sqlx::query("BEGIN DEFERRED").execute(&mut conn).await?;
            sqlx::query("SELECT name FROM sqlite_schema LIMIT 1")
                .fetch_optional(&mut conn)
                .await?;
            holding_read_mark = true;
        }

        Ok(WalGuard {
            conn,
            options,
            mode,
            holding_read_mark,
        })
    }

    pub async fn is_alive(&mut self) -> bool {
        sqlx::query("SELECT 1")
            .execute(&mut self.conn)
            .await
            .is_ok()
    }

    pub async fn reconnect(&mut self) -> Result<(), sqlx::Error> {
        // The fresh connection inherits no transaction state, so the read-mark
        // flag must be re-derived rather than preserved.
        self.holding_read_mark = false;
        self.conn = self.options.clone().connect().await?;
        crate::apply_performance_pragmas(&mut self.conn).await?;
        sqlx::query("SELECT count(*) FROM sqlite_master")
            .fetch_one(&mut self.conn)
            .await?;

        // HoldRead: the read-mark is the guard's prevention mechanism — always
        // reacquire it on reconnect (matches the monitor's expectation that a
        // live HoldRead guard holds its mark outside truncate windows).
        if self.mode == Mode::HoldRead {
            sqlx::query("BEGIN DEFERRED")
                .execute(&mut self.conn)
                .await?;
            sqlx::query("SELECT name FROM sqlite_schema LIMIT 1")
                .fetch_optional(&mut self.conn)
                .await?;
            self.holding_read_mark = true;
        }

        Ok(())
    }

    pub async fn release_read_mark(&mut self) {
        if self.holding_read_mark {
            if let Err(e) = sqlx::query("COMMIT").execute(&mut self.conn).await {
                tracing::warn!(error = ?e, "wal guard: releasing read-mark failed (COMMIT)");
            }
            self.holding_read_mark = false;
        }
    }

    pub async fn reacquire_read_mark(&mut self) -> Result<(), sqlx::Error> {
        if self.mode != Mode::HoldRead {
            return Ok(());
        }

        sqlx::query("BEGIN DEFERRED")
            .execute(&mut self.conn)
            .await?;
        sqlx::query("SELECT name FROM sqlite_schema LIMIT 1")
            .fetch_optional(&mut self.conn)
            .await?;
        self.holding_read_mark = true;
        Ok(())
    }
}

pub async fn open_salvage_connection(db_path: &Path) -> Result<SqliteConnection, sqlx::Error> {
    let mut conn = options_for(db_path)?.connect().await?;
    crate::apply_performance_pragmas(&mut conn).await?;
    sqlx::query("SELECT count(*) FROM sqlite_master")
        .fetch_one(&mut conn)
        .await?;
    Ok(conn)
}

pub fn guard_disabled() -> bool {
    std::env::var("VK_WAL_GUARD")
        .map(|v| matches!(v.to_ascii_lowercase().as_str(), "off" | "0" | "false"))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Serial against the refusal-flag tests: guard connections run
    // apply_performance_pragmas, which honours the process-global
    // WAL_WRITE_REFUSAL_ACTIVE flag (busy_timeout=0 + query_only) — a concurrent
    // refusal test would turn this test's writes READONLY.
    #[tokio::test]
    #[serial_test::serial]
    async fn guard_blocks_external_unlink_hold_read() {
        if std::process::Command::new("sqlite3")
            .arg("--version")
            .output()
            .is_err()
        {
            eprintln!("Skipping test: sqlite3 CLI not available");
            return;
        }
        let (pool, tmp) = crate::test_utils::create_test_pool().await;
        let db_path = tmp.path().join("test.db");
        sqlx::query("INSERT INTO projects (id, name, git_repo_path) VALUES (randomblob(16), 'guard-probe', '/tmp/guard-probe-uniq')").execute(&pool).await.unwrap();
        let mode = Mode::MapOnly;
        let guard = WalGuard::connect(&db_path, mode).await.unwrap();
        pool.close().await;
        let out = std::process::Command::new("sqlite3")
            .arg(&db_path)
            .arg("PRAGMA user_version=1;")
            .output()
            .unwrap();
        assert!(out.status.success());
        let wal = std::path::PathBuf::from(format!("{}-wal", db_path.display()));
        assert!(
            wal.exists(),
            "external write-session close unlinked the WAL despite the guard (pool already closed)"
        );
        drop(guard);
        let offline = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options_for(&db_path).unwrap())
            .await
            .unwrap();
        let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM projects WHERE name='guard-probe'")
            .fetch_one(&offline)
            .await
            .unwrap();
        assert_eq!(
            n, 1,
            "row not durable after the external write session despite the guard"
        );
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn reconnect_restores_read_mark() {
        let (pool, tmp) = crate::test_utils::create_test_pool().await;
        let db_path = tmp.path().join("test.db");
        let mut guard = WalGuard::connect(&db_path, Mode::HoldRead).await.unwrap();
        sqlx::query("CREATE TEMP TABLE reconnect_probe(x)")
            .execute(&mut guard.conn)
            .await
            .unwrap();
        guard.reconnect().await.unwrap();
        let (probe,): (i64,) =
            sqlx::query_as("SELECT count(*) FROM sqlite_temp_master WHERE name='reconnect_probe'")
                .fetch_one(&mut guard.conn)
                .await
                .unwrap();
        assert_eq!(probe, 0, "reconnect() did not replace the connection");
        assert!(guard.is_alive().await);
        assert!(guard.holding_read_mark);
        assert!(
            sqlx::query("BEGIN DEFERRED")
                .execute(&mut guard.conn)
                .await
                .is_err(),
            "no open read transaction after reconnect — read-mark not re-materialised"
        );
        pool.close().await;
    }

    #[test]
    #[serial_test::serial]
    fn test_guard_disabled() {
        // Unset
        unsafe {
            std::env::remove_var("VK_WAL_GUARD");
        }
        assert!(!super::guard_disabled());

        // Set to "off"
        unsafe {
            std::env::set_var("VK_WAL_GUARD", "off");
        }
        assert!(super::guard_disabled());

        // Set to "0"
        unsafe {
            std::env::set_var("VK_WAL_GUARD", "0");
        }
        assert!(super::guard_disabled());

        // Set to "false"
        unsafe {
            std::env::set_var("VK_WAL_GUARD", "false");
        }
        assert!(super::guard_disabled());

        // Set to "OFF"
        unsafe {
            std::env::set_var("VK_WAL_GUARD", "OFF");
        }
        assert!(super::guard_disabled());

        // Set to anything else
        unsafe {
            std::env::set_var("VK_WAL_GUARD", "on");
        }
        assert!(!super::guard_disabled());

        unsafe {
            std::env::remove_var("VK_WAL_GUARD");
        }
    }
}
