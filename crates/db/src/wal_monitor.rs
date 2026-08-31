//! WAL (Write-Ahead Log) file monitoring service.
//!
//! This module monitors the SQLite WAL file size and triggers alerts when
//! it grows beyond acceptable thresholds. Large WAL files can indicate
//! checkpoint issues or sustained heavy write load.
//!
//! # Design
//!
//! - Runs as a background task checking WAL size periodically
//! - Updates metrics with current WAL size
//! - Logs warnings when WAL exceeds configurable threshold
//! - Optionally triggers passive checkpoint when WAL is large
//! - Runs periodic TRUNCATE checkpoints to minimize data loss on abrupt shutdown

use std::panic::AssertUnwindSafe;
use std::path::{Path, PathBuf};
use std::time::Duration;

use futures::FutureExt;
use sqlx::{SqliteConnection, SqlitePool};
use tokio::sync::mpsc;

#[cfg(target_os = "linux")]
use notify::Watcher;

use crate::DbMetrics;

/// Default check interval in seconds.
const DEFAULT_CHECK_INTERVAL_SECS: u64 = 60;

/// Default WAL size warning threshold in MB.
const DEFAULT_WARNING_THRESHOLD_MB: u64 = 50;

/// Default WAL size for triggering passive checkpoint in MB.
const DEFAULT_CHECKPOINT_THRESHOLD_MB: u64 = 100;

/// Default interval for forced TRUNCATE checkpoint in seconds (5 minutes).
/// This ensures max data loss of 5 minutes if the server is killed abruptly.
const DEFAULT_TRUNCATE_INTERVAL_SECS: u64 = 300;

/// Configuration for the WAL monitor.
#[derive(Clone, Debug)]
pub struct WalMonitorConfig {
    /// How often to check WAL size (in seconds).
    pub check_interval_secs: u64,
    /// WAL size in bytes that triggers a warning log.
    pub warning_threshold_bytes: u64,
    /// WAL size in bytes that triggers a passive checkpoint.
    pub checkpoint_threshold_bytes: u64,
    /// Whether to automatically trigger passive checkpoints.
    pub auto_checkpoint: bool,
    /// Interval in seconds for forced TRUNCATE checkpoint (flushes all WAL to main DB).
    /// This ensures data is regularly persisted to minimize loss on abrupt kill.
    /// Set to 0 to disable periodic TRUNCATE checkpoints.
    pub truncate_checkpoint_interval_secs: u64,
}

impl Default for WalMonitorConfig {
    fn default() -> Self {
        let warning_mb =
            get_env_or_default("VK_WAL_WARNING_THRESHOLD_MB", DEFAULT_WARNING_THRESHOLD_MB);
        let checkpoint_mb = get_env_or_default(
            "VK_WAL_CHECKPOINT_THRESHOLD_MB",
            DEFAULT_CHECKPOINT_THRESHOLD_MB,
        );

        Self {
            check_interval_secs: get_env_or_default(
                "VK_WAL_CHECK_INTERVAL_SECS",
                DEFAULT_CHECK_INTERVAL_SECS,
            ),
            warning_threshold_bytes: warning_mb * 1024 * 1024,
            checkpoint_threshold_bytes: checkpoint_mb * 1024 * 1024,
            auto_checkpoint: std::env::var("VK_WAL_AUTO_CHECKPOINT")
                .map(|v| v == "true" || v == "1")
                .unwrap_or(true),
            truncate_checkpoint_interval_secs: get_env_or_default(
                "VK_WAL_TRUNCATE_INTERVAL_SECS",
                DEFAULT_TRUNCATE_INTERVAL_SECS,
            ),
        }
    }
}

fn get_env_or_default(var: &str, default: u64) -> u64 {
    std::env::var(var)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

/// WAL file presence and identity state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WalState {
    Absent,
    Present(Option<u64>),
}

/// Cross-platform WAL inode extraction.
#[cfg(unix)]
fn wal_identity(md: &std::fs::Metadata) -> Option<u64> {
    use std::os::unix::fs::MetadataExt;
    Some(md.ino())
}

#[cfg(not(unix))]
fn wal_identity(_md: &std::fs::Metadata) -> Option<u64> {
    None
}

/// WAL state transition classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WalTransition {
    Unchanged,
    Appeared,
    Vanished,
    Replaced,
}

fn wal_transition(last: WalState, current: WalState) -> WalTransition {
    match (last, current) {
        (WalState::Absent, WalState::Absent) => WalTransition::Unchanged,
        (WalState::Absent, WalState::Present(_)) => WalTransition::Appeared,
        (WalState::Present(_), WalState::Absent) => WalTransition::Vanished,
        (WalState::Present(Some(a)), WalState::Present(Some(b))) if a != b => {
            WalTransition::Replaced
        }
        (WalState::Present(_), WalState::Present(_)) => WalTransition::Unchanged,
    }
}

/// Construct WAL path by appending `-wal` suffix to database path.
fn wal_path_for(db_path: &Path) -> PathBuf {
    PathBuf::from(format!("{}-wal", db_path.display()))
}

/// Event logged when WAL is unlinked externally.
#[allow(dead_code)]
struct UnlinkedEvent {
    event: &'static str,
    path: PathBuf,
    wal_path: PathBuf,
    last_inode: Option<u64>,
    remediation: &'static str,
}

/// Handle for controlling the WAL monitor.
#[derive(Clone)]
pub struct WalMonitorHandle {
    tx: mpsc::Sender<WalMonitorCommand>,
}

enum WalMonitorCommand {
    /// Request immediate WAL size check.
    CheckNow,
    /// Request immediate passive checkpoint.
    Checkpoint,
    /// Request immediate TRUNCATE checkpoint (blocks until all WAL is flushed).
    TruncateCheckpoint,
    /// Shutdown the monitor with acknowledgement.
    Shutdown(tokio::sync::oneshot::Sender<()>),
}

impl WalMonitorHandle {
    /// Request an immediate WAL size check.
    pub async fn check_now(&self) {
        let _ = self.tx.send(WalMonitorCommand::CheckNow).await;
    }

    /// Request an immediate passive checkpoint.
    pub async fn checkpoint(&self) {
        let _ = self.tx.send(WalMonitorCommand::Checkpoint).await;
    }

    /// Request an immediate TRUNCATE checkpoint (blocks until all WAL is flushed).
    pub async fn truncate_checkpoint(&self) {
        let _ = self.tx.send(WalMonitorCommand::TruncateCheckpoint).await;
    }

    /// Shutdown the WAL monitor.
    pub async fn shutdown(&self) {
        let (ack_tx, ack_rx) = tokio::sync::oneshot::channel();
        let _ = self.tx.send(WalMonitorCommand::Shutdown(ack_tx)).await;
        match tokio::time::timeout(std::time::Duration::from_secs(10), ack_rx).await {
            Ok(_) => {}
            Err(_) => tracing::error!("WAL monitor did not ack shutdown within 10s"),
        }
    }
}

struct RefusalLatch {
    _conn: sqlx::sqlite::SqliteConnection,
}

impl RefusalLatch {
    async fn arm(mut conn: sqlx::sqlite::SqliteConnection) -> Result<Self, sqlx::Error> {
        sqlx::query("BEGIN IMMEDIATE").execute(&mut conn).await?;
        Ok(Self { _conn: conn })
    }

    /// Clear the process-wide refusal fence after explicitly dismantling a test latch.
    #[cfg(test)]
    pub(crate) fn disarm(self) {
        crate::WAL_WRITE_REFUSAL_ACTIVE.store(false, std::sync::atomic::Ordering::SeqCst);
    }
}

/// Fail-fast latched writers: production pools use a 30s SQLite busy_timeout, which
/// outlives the 10s client deadline. Flip the process flag (after_release + new
/// connects) and PRAGMA-zero idle connections. Never `timeout(acquire)` — sqlx
/// drops the popped connection on cancel, emptying the pool.
async fn zero_pooled_busy_timeout(pool: &SqlitePool) {
    crate::WAL_WRITE_REFUSAL_ACTIVE.store(true, std::sync::atomic::Ordering::SeqCst);
    let mut held = Vec::new();
    while let Some(conn) = pool.try_acquire() {
        held.push(conn);
    }
    for mut conn in held {
        if let Err(e) = sqlx::query("PRAGMA busy_timeout = 0")
            .execute(&mut *conn)
            .await
        {
            tracing::warn!(
                error = ?e,
                "wal monitor: failed to zero busy_timeout on pooled connection"
            );
        }
    }
}

/// Fence idle pooled connections before releasing the old-domain refusal latch.
async fn fence_pooled_read_only(pool: &SqlitePool) {
    let mut held = Vec::new();
    while let Some(conn) = pool.try_acquire() {
        held.push(conn);
    }
    for mut conn in held {
        if let Err(e) = sqlx::query("PRAGMA query_only = ON")
            .execute(&mut *conn)
            .await
        {
            tracing::warn!(
                error = ?e,
                "wal monitor: failed to fence pooled connection read-only"
            );
        }
    }
}

/// WAL monitoring service.
pub struct WalMonitor {
    db_path: PathBuf,
    pool: SqlitePool,
    metrics: DbMetrics,
    config: WalMonitorConfig,
    last_wal_state: WalState,
    wal_ever_present: bool,
    tripped: bool,
    trip_events: u32,
    guard: Option<crate::wal_guard::WalGuard>,
    salvage_conn: Option<SqliteConnection>,
    last_salvage: Option<Result<(i32, i32, i32), String>>,
    refusal: Option<RefusalLatch>,
}

impl WalMonitor {
    /// Spawn a new WAL monitor as a background task.
    ///
    /// Returns a handle that can be used to control the monitor.
    pub fn spawn(
        db_path: impl AsRef<Path>,
        pool: SqlitePool,
        metrics: DbMetrics,
        config: WalMonitorConfig,
        guard: Option<crate::wal_guard::WalGuard>,
        salvage_conn: Option<SqliteConnection>,
    ) -> WalMonitorHandle {
        let (tx, rx) = mpsc::channel(16);
        let db_path_buf = db_path.as_ref().to_path_buf();
        let wal_path = wal_path_for(&db_path_buf);

        let last_wal_state = match std::fs::metadata(&wal_path) {
            Ok(md) => WalState::Present(wal_identity(&md)),
            Err(_) => WalState::Absent,
        };
        let wal_ever_present = matches!(last_wal_state, WalState::Present(_));

        let monitor = Self {
            db_path: db_path_buf,
            pool,
            metrics,
            config,
            last_wal_state,
            wal_ever_present,
            tripped: false,
            trip_events: 0,
            guard,
            salvage_conn,
            last_salvage: None,
            refusal: None,
        };
        tokio::spawn(async move {
            let _ = supervised_run("wal_monitor", monitor.run(rx)).await;
        });
        WalMonitorHandle { tx }
    }

    /// Spawn with default configuration.
    pub fn spawn_default(
        db_path: impl AsRef<Path>,
        pool: SqlitePool,
        metrics: DbMetrics,
        guard: Option<crate::wal_guard::WalGuard>,
        salvage_conn: Option<SqliteConnection>,
    ) -> WalMonitorHandle {
        Self::spawn(
            db_path,
            pool,
            metrics,
            WalMonitorConfig::default(),
            guard,
            salvage_conn,
        )
    }

    async fn run(mut self, mut rx: mpsc::Receiver<WalMonitorCommand>) {
        let mut check_interval =
            tokio::time::interval(Duration::from_secs(self.config.check_interval_secs));

        // Periodic TRUNCATE checkpoint timer - ensures data is persisted regularly
        // to minimize data loss if the server is killed abruptly (e.g., by pkill from child processes)
        let truncate_enabled = self.config.truncate_checkpoint_interval_secs > 0;
        let mut truncate_interval =
            tokio::time::interval(Duration::from_secs(if truncate_enabled {
                self.config.truncate_checkpoint_interval_secs
            } else {
                u64::MAX // Effectively disabled
            }));

        tracing::info!(
            check_interval_secs = self.config.check_interval_secs,
            warning_threshold_mb = self.config.warning_threshold_bytes / (1024 * 1024),
            checkpoint_threshold_mb = self.config.checkpoint_threshold_bytes / (1024 * 1024),
            auto_checkpoint = self.config.auto_checkpoint,
            truncate_interval_secs = self.config.truncate_checkpoint_interval_secs,
            "WAL monitor started"
        );

        // Skip first immediate tick for truncate interval
        if truncate_enabled {
            truncate_interval.tick().await;
        }

        #[cfg(target_os = "linux")]
        {
            let mut watch = start_watch(&self.db_path);
            let wal_basename = wal_path_for(&self.db_path)
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();

            loop {
                tokio::select! {
                     Some(cmd) = rx.recv() => {
                         match cmd {
                             WalMonitorCommand::CheckNow => {
                                 self.check_wal_size().await;
                             }
                             WalMonitorCommand::Checkpoint => {
                                 self.run_checkpoint().await;
                             }
                             WalMonitorCommand::TruncateCheckpoint => {
                                 self.run_truncate_checkpoint().await;
                             }
                              WalMonitorCommand::Shutdown(ack) => {
                                  if let Some(ref mut g) = self.guard {
                                      g.release_read_mark().await;
                                  }
                                  if self.refusal.is_some() {
                                      fence_pooled_read_only(&self.pool).await;
                                      self.refusal = None;
                                  }
                                 tracing::info!("WAL monitor shutting down");
                                 let _ = ack.send(());
                                 break;
                             }
                        }
                    }
                    _ = check_interval.tick() => {
                        self.check_wal_size().await;
                        if watch.is_none() {
                            watch = start_watch(&self.db_path);
                            if watch.is_some() {
                                tracing::info!("WAL inotify watch re-armed");
                            }
                        }
                    }
                    _ = truncate_interval.tick(), if truncate_enabled => {
                        if let Some(guard) = &mut self.guard {
                            guard.release_read_mark().await;
                        }
                        self.run_truncate_checkpoint().await;
                        if let Some(guard) = &mut self.guard
                            && let Err(e) = guard.reacquire_read_mark().await {
                            tracing::error!(error = ?e, "failed to reacquire WAL guard read mark");
                        }
                    }
                    ev = async {
                        match watch.as_mut() {
                            Some((_, rx)) => rx.recv().await,
                            None => std::future::pending().await,
                        }
                    } => {
                        match ev {
                            Some(Ok(event)) => {
                                if is_wal_removal(&event.kind, &event.paths, &wal_basename) {
                                    self.check_wal_size().await;
                                }
                            }
                            Some(Err(e)) => {
                                tracing::warn!(error = ?e, "wal inotify watch dropped; falling back to 60s poll");
                                watch = None;
                            }
                            None => {
                                tracing::warn!("wal inotify watch dropped; falling back to 60s poll");
                                watch = None;
                            }
                        }
                    }
                }

                if let Some(guard) = &mut self.guard
                    && !guard.is_alive().await
                {
                    match guard.reconnect().await {
                        Ok(_) => {
                            tracing::warn!(event = "wal_guard_reconnected", "WAL guard reconnected")
                        }
                        Err(e) => {
                            tracing::error!(error = ?e, "WAL guard reconnect failed");
                            if !self.tripped {
                                self.tripped = true;
                                self.trip_events += 1;
                                tracing::error!(
                                    event = "wal_guard_unavailable",
                                    "WAL guard unavailable; treating as durability trip"
                                );
                                self.handle_trip().await;
                            }
                        }
                    }
                }
            }
        }

        #[cfg(not(target_os = "linux"))]
        {
            tracing::debug!(
                "WAL inotify monitoring unavailable on non-Linux; using 60s poll fallback"
            );

            loop {
                tokio::select! {
                     Some(cmd) = rx.recv() => {
                         match cmd {
                             WalMonitorCommand::CheckNow => {
                                 self.check_wal_size().await;
                             }
                             WalMonitorCommand::Checkpoint => {
                                 self.run_checkpoint().await;
                             }
                             WalMonitorCommand::TruncateCheckpoint => {
                                 self.run_truncate_checkpoint().await;
                             }
                              WalMonitorCommand::Shutdown(ack) => {
                                  if let Some(ref mut g) = self.guard {
                                      g.release_read_mark().await;
                                  }
                                  if self.refusal.is_some() {
                                      fence_pooled_read_only(&self.pool).await;
                                      self.refusal = None;
                                  }
                                 tracing::info!("WAL monitor shutting down");
                                 let _ = ack.send(());
                                 break;
                             }
                        }
                    }
                    _ = check_interval.tick() => {
                        self.check_wal_size().await;
                    }
                    _ = truncate_interval.tick(), if truncate_enabled => {
                        if let Some(guard) = &mut self.guard {
                            guard.release_read_mark().await;
                        }
                        self.run_truncate_checkpoint().await;
                        if let Some(guard) = &mut self.guard
                            && let Err(e) = guard.reacquire_read_mark().await {
                            tracing::error!(error = ?e, "failed to reacquire WAL guard read mark");
                        }
                    }
                }

                if let Some(guard) = &mut self.guard
                    && !guard.is_alive().await
                {
                    match guard.reconnect().await {
                        Ok(_) => {
                            tracing::warn!(event = "wal_guard_reconnected", "WAL guard reconnected")
                        }
                        Err(e) => {
                            tracing::error!(error = ?e, "WAL guard reconnect failed");
                            if !self.tripped {
                                self.tripped = true;
                                self.trip_events += 1;
                                tracing::error!(
                                    event = "wal_guard_unavailable",
                                    "WAL guard unavailable; treating as durability trip"
                                );
                                self.handle_trip().await;
                            }
                        }
                    }
                }
            }
        }
    }

    async fn check_wal_size(&mut self) {
        let wal_path = wal_path_for(&self.db_path);

        let (current, wal_size) = match std::fs::metadata(&wal_path) {
            Ok(md) => (WalState::Present(wal_identity(&md)), md.len()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (WalState::Absent, 0),
            Err(e) => {
                tracing::warn!(
                    error = ?e,
                    path = ?wal_path,
                    "Failed to read WAL file metadata"
                );
                return;
            }
        };

        if self.tripped {
            self.last_wal_state = current;
            return;
        }

        let transition = wal_transition(self.last_wal_state, current);

        match transition {
            WalTransition::Appeared => {
                self.wal_ever_present = true;
                self.last_wal_state = current;
                self.metrics.update_wal_size(wal_size);
                let wal_size_mb = wal_size as f64 / (1024.0 * 1024.0);
                if wal_size >= self.config.checkpoint_threshold_bytes {
                    tracing::warn!(
                        wal_size_mb = format!("{:.2}", wal_size_mb),
                        threshold_mb = self.config.checkpoint_threshold_bytes / (1024 * 1024),
                        "WAL file exceeds checkpoint threshold"
                    );
                    if self.config.auto_checkpoint {
                        self.run_checkpoint().await;
                    }
                } else if wal_size >= self.config.warning_threshold_bytes {
                    tracing::warn!(
                        wal_size_mb = format!("{:.2}", wal_size_mb),
                        threshold_mb = self.config.warning_threshold_bytes / (1024 * 1024),
                        "WAL file size exceeds warning threshold"
                    );
                } else {
                    tracing::debug!(
                        wal_size_mb = format!("{:.2}", wal_size_mb),
                        "WAL file size check completed"
                    );
                }
            }
            WalTransition::Unchanged if matches!(current, WalState::Present(_)) => {
                self.wal_ever_present = true;
                self.metrics.update_wal_size(wal_size);
                let wal_size_mb = wal_size as f64 / (1024.0 * 1024.0);
                if wal_size >= self.config.checkpoint_threshold_bytes {
                    tracing::warn!(
                        wal_size_mb = format!("{:.2}", wal_size_mb),
                        threshold_mb = self.config.checkpoint_threshold_bytes / (1024 * 1024),
                        "WAL file exceeds checkpoint threshold"
                    );
                    if self.config.auto_checkpoint {
                        self.run_checkpoint().await;
                    }
                } else if wal_size >= self.config.warning_threshold_bytes {
                    tracing::warn!(
                        wal_size_mb = format!("{:.2}", wal_size_mb),
                        threshold_mb = self.config.warning_threshold_bytes / (1024 * 1024),
                        "WAL file size exceeds warning threshold"
                    );
                } else {
                    tracing::debug!(
                        wal_size_mb = format!("{:.2}", wal_size_mb),
                        "WAL file size check completed"
                    );
                }
                self.last_wal_state = current;
            }
            WalTransition::Replaced | WalTransition::Vanished => {
                let last_inode = match self.last_wal_state {
                    WalState::Present(Some(inode)) => Some(inode),
                    _ => None,
                };
                let event = UnlinkedEvent {
                    event: "wal_unlinked_externally",
                    path: self.db_path.clone(),
                    wal_path: wal_path.clone(),
                    last_inode,
                    remediation: "node will refuse writes; restart the node after investigating",
                };
                tracing::warn!(
                    event = "wal_unlinked_externally",
                    path = %event.path.display(),
                    wal_path = %event.wal_path.display(),
                    last_inode = ?event.last_inode,
                    remediation = event.remediation,
                    "WAL unlinked externally"
                );
                self.tripped = true;
                self.trip_events += 1;
                self.last_wal_state = current;
                self.handle_trip().await;
            }
            WalTransition::Unchanged
                if matches!(current, WalState::Absent) && self.wal_ever_present =>
            {
                let last_inode = match self.last_wal_state {
                    WalState::Present(Some(inode)) => Some(inode),
                    _ => None,
                };
                let event = UnlinkedEvent {
                    event: "wal_unlinked_externally",
                    path: self.db_path.clone(),
                    wal_path: wal_path.clone(),
                    last_inode,
                    remediation: "node will refuse writes; restart the node after investigating",
                };
                tracing::warn!(
                    event = "wal_unlinked_externally",
                    path = %event.path.display(),
                    wal_path = %event.wal_path.display(),
                    last_inode = ?event.last_inode,
                    remediation = event.remediation,
                    "WAL unlinked externally"
                );
                self.tripped = true;
                self.trip_events += 1;
                self.last_wal_state = current;
                self.handle_trip().await;
            }
            WalTransition::Unchanged
                if matches!(current, WalState::Absent) && !self.wal_ever_present =>
            {
                tracing::debug!("WAL not yet present (benign)");
                self.last_wal_state = current;
            }
            _ => {
                self.last_wal_state = current;
            }
        }
    }

    async fn run_salvage_checkpoint(&mut self) -> Result<(i32, i32, i32), sqlx::Error> {
        use sqlx::Row;
        let conn = self.salvage_conn.as_mut().ok_or_else(|| {
            sqlx::Error::Io(std::io::Error::new(
                std::io::ErrorKind::NotConnected,
                "salvage connection unavailable",
            ))
        })?;
        let row = sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
            .fetch_one(conn)
            .await?;
        let (busy, log_frames, checkpointed): (i32, i32, i32) =
            (row.try_get(0)?, row.try_get(1)?, row.try_get(2)?);
        if busy != 0 {
            return Err(sqlx::Error::Protocol(format!(
                "salvage checkpoint blocked (busy={busy}, log_frames={log_frames}, checkpointed={checkpointed})"
            )));
        }
        Ok((busy, log_frames, checkpointed))
    }

    async fn handle_trip(&mut self) {
        match self.run_salvage_checkpoint().await {
            Ok((busy, log_frames, checkpointed)) => {
                tracing::info!(
                    event = "wal_salvage_checkpoint_succeeded",
                    busy,
                    log_frames,
                    checkpointed_frames = checkpointed,
                    "WAL salvage checkpoint succeeded"
                );
                self.last_salvage = Some(Ok((busy, log_frames, checkpointed)));
            }
            Err(e) => {
                tracing::error!(event = "wal_salvage_checkpoint_failed", error = ?e, "WAL salvage checkpoint failed");
                self.last_salvage = Some(Err(e.to_string()));
            }
        }

        match self.salvage_conn.take() {
            Some(conn) => match RefusalLatch::arm(conn).await {
                Ok(l) => {
                    self.refusal = Some(l);
                    // Zero busy_timeout BEFORE the refusal log: the live harness POSTs
                    // as soon as it sees wal_write_refusal_active, and production pools
                    // wait 30s on SQLITE_BUSY (curl --max-time 10 → HTTP 000).
                    zero_pooled_busy_timeout(&self.pool).await;
                    tracing::error!(event = "wal_write_refusal_active", path = %self.db_path.display(), remediation = "writes are refused until the node is restarted", "WAL write refusal active");
                }
                Err(e) => {
                    tracing::error!(event = "wal_write_refusal_active", armed = false, error = ?e, "write-refusal latch could not be armed; closing the pool (D6 deviation: refuse-everything)");
                    self.pool.close().await;
                }
            },
            None => {
                tracing::error!(
                    event = "wal_write_refusal_active",
                    armed = false,
                    error = "salvage connection unavailable",
                    "write-refusal latch could not be armed; closing the pool (D6 deviation: refuse-everything)"
                );
                self.pool.close().await;
            }
        }
    }

    /// Run a PASSIVE checkpoint.
    ///
    /// PASSIVE checkpoint does not block readers or writers.
    /// It checkpoints as many frames as possible without waiting.
    async fn run_checkpoint(&self) {
        tracing::info!("Running passive WAL checkpoint");

        let start = std::time::Instant::now();

        // Use PRAGMA wal_checkpoint(PASSIVE) which doesn't block
        let result: Result<(i32, i32, i32), sqlx::Error> =
            sqlx::query_as("PRAGMA wal_checkpoint(PASSIVE)")
                .fetch_one(&self.pool)
                .await;

        let duration = start.elapsed();
        self.metrics.update_checkpoint_duration(duration);

        match result {
            Ok((blocked, log_pages, checkpointed)) => {
                tracing::info!(
                    duration_ms = duration.as_millis() as u64,
                    blocked = blocked,
                    log_pages = log_pages,
                    checkpointed = checkpointed,
                    "WAL checkpoint completed"
                );
            }
            Err(e) => {
                tracing::error!(
                    error = ?e,
                    duration_ms = duration.as_millis() as u64,
                    "WAL checkpoint failed"
                );
            }
        }
    }

    /// Run a TRUNCATE checkpoint.
    ///
    /// TRUNCATE checkpoint blocks until ALL WAL content is written to the main database file,
    /// then truncates the WAL file to zero bytes. This ensures all data is persisted to the
    /// main database file, minimizing data loss if the server is killed abruptly.
    ///
    /// This is more aggressive than PASSIVE checkpoint but provides stronger data durability.
    async fn run_truncate_checkpoint(&mut self) {
        tracing::info!("Running TRUNCATE checkpoint (periodic data safety)");

        let start = std::time::Instant::now();

        // Use PRAGMA wal_checkpoint(TRUNCATE) which blocks until complete
        let result: Result<(i32, i32, i32), sqlx::Error> =
            sqlx::query_as("PRAGMA wal_checkpoint(TRUNCATE)")
                .fetch_one(&self.pool)
                .await;

        let duration = start.elapsed();
        self.metrics.update_checkpoint_duration(duration);

        match result {
            Ok((blocked, log_pages, checkpointed)) => {
                if blocked == 0 {
                    tracing::info!(
                        duration_ms = duration.as_millis() as u64,
                        log_pages = log_pages,
                        checkpointed = checkpointed,
                        "TRUNCATE checkpoint completed - all WAL flushed to main database"
                    );
                } else {
                    // blocked != 0 means checkpoint was blocked (busy database)
                    tracing::warn!(
                        duration_ms = duration.as_millis() as u64,
                        blocked = blocked,
                        log_pages = log_pages,
                        checkpointed = checkpointed,
                        "TRUNCATE checkpoint was blocked - some WAL may not be flushed"
                    );
                }
            }
            Err(e) => {
                tracing::warn!(
                    error = ?e,
                    duration_ms = duration.as_millis() as u64,
                    "TRUNCATE checkpoint failed"
                );
            }
        }
    }
}

/// Get the current WAL file size for a database.
///
/// Returns 0 if the WAL file doesn't exist.
pub fn get_wal_size(db_path: impl AsRef<Path>) -> u64 {
    let wal_path = wal_path_for(db_path.as_ref());
    std::fs::metadata(&wal_path).map(|m| m.len()).unwrap_or(0)
}

/// Check if an inotify event indicates WAL removal.
#[cfg(target_os = "linux")]
fn is_wal_removal(
    kind: &notify::event::EventKind,
    paths: &[std::path::PathBuf],
    wal_basename: &str,
) -> bool {
    use notify::event::{ModifyKind, RenameMode};

    // Match Remove(_) or Modify(Name(RenameMode::From))
    let is_removal_kind = matches!(kind, notify::event::EventKind::Remove(_))
        || matches!(
            kind,
            notify::event::EventKind::Modify(ModifyKind::Name(RenameMode::From))
        );

    if !is_removal_kind {
        return false;
    }

    // Check if any path's file_name equals wal_basename
    paths.iter().any(|path| {
        path.file_name()
            .and_then(|name| name.to_str())
            .map(|name_str| name_str == wal_basename)
            .unwrap_or(false)
    })
}

/// Set up inotify watch for WAL file in the database directory.
#[cfg(target_os = "linux")]
fn start_watch(
    db_path: &Path,
) -> Option<(
    notify::RecommendedWatcher,
    tokio::sync::mpsc::UnboundedReceiver<Result<notify::Event, notify::Error>>,
)> {
    use notify::{RecommendedWatcher, RecursiveMode};

    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

    let mut watcher = match RecommendedWatcher::new(
        move |res| {
            let _ = tx.send(res);
        },
        notify::Config::default(),
    ) {
        Ok(w) => w,
        Err(e) => {
            tracing::warn!(error = ?e, "Failed to create WAL inotify watcher; falling back to 60s poll");
            return None;
        }
    };

    let db_dir = db_path.parent().unwrap_or(Path::new("."));
    match watcher.watch(db_dir, RecursiveMode::NonRecursive) {
        Ok(_) => Some((watcher, rx)),
        Err(e) => {
            tracing::warn!(error = ?e, "Failed to watch WAL directory; falling back to 60s poll");
            None
        }
    }
}

/// Run `fut` to completion, catching any panic and logging it at error level.
///
/// Returns `Ok(())` on normal completion, `Err(panic_message)` on panic. This
/// lets long-running background tasks fail noisily instead of being silently
/// swallowed by a dropped `JoinHandle`.
async fn supervised_run<F>(name: &'static str, fut: F) -> Result<(), String>
where
    F: std::future::Future<Output = ()>,
{
    match AssertUnwindSafe(fut).catch_unwind().await {
        Ok(()) => Ok(()),
        Err(panic) => {
            let msg = panic
                .downcast_ref::<&'static str>()
                .map(|s| (*s).to_string())
                .or_else(|| panic.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "<non-string panic>".to_string());
            tracing::error!(task = name, panic = %msg, "background task panicked");
            Err(msg)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = WalMonitorConfig::default();
        assert_eq!(config.check_interval_secs, DEFAULT_CHECK_INTERVAL_SECS);
        assert_eq!(
            config.warning_threshold_bytes,
            DEFAULT_WARNING_THRESHOLD_MB * 1024 * 1024
        );
        assert_eq!(
            config.checkpoint_threshold_bytes,
            DEFAULT_CHECKPOINT_THRESHOLD_MB * 1024 * 1024
        );
        assert!(config.auto_checkpoint);
        assert_eq!(
            config.truncate_checkpoint_interval_secs,
            DEFAULT_TRUNCATE_INTERVAL_SECS
        );
    }

    #[test]
    fn test_get_wal_size_nonexistent() {
        let size = get_wal_size("/nonexistent/path/db.sqlite");
        assert_eq!(size, 0);
    }

    #[tokio::test]
    async fn supervised_run_passes_through_normal_completion() {
        let result = supervised_run("test", async {
            // no-op
        })
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn supervised_run_catches_panic_and_reports_message() {
        let result = supervised_run("test", async {
            panic!("synthetic boom for test");
        })
        .await;
        assert!(matches!(result, Err(ref msg) if msg.contains("synthetic boom for test")));
    }

    #[tokio::test]
    async fn supervised_run_catches_non_string_panic_with_fallback_marker() {
        let result = supervised_run("test", async {
            std::panic::panic_any(123_u32);
        })
        .await;
        assert!(
            matches!(result, Err(ref msg) if msg.contains("<non-string panic>")),
            "non-string panic payload should yield the fallback marker, got {result:?}"
        );
    }

    #[test]
    fn wal_path_for_appends_dash_wal() {
        assert_eq!(
            wal_path_for(Path::new("/tmp/test.db")),
            PathBuf::from("/tmp/test.db-wal")
        );
        assert_eq!(
            wal_path_for(Path::new("/data/db.sqlite")),
            PathBuf::from("/data/db.sqlite-wal")
        );
    }

    #[test]
    fn wal_transition_classifies_all_cases() {
        assert_eq!(
            wal_transition(WalState::Absent, WalState::Absent),
            WalTransition::Unchanged
        );
        assert_eq!(
            wal_transition(WalState::Absent, WalState::Present(None)),
            WalTransition::Appeared
        );
        assert_eq!(
            wal_transition(WalState::Present(None), WalState::Absent),
            WalTransition::Vanished
        );
        assert_eq!(
            wal_transition(WalState::Present(Some(1)), WalState::Present(Some(2))),
            WalTransition::Replaced
        );
        assert_eq!(
            wal_transition(WalState::Present(Some(2)), WalState::Present(Some(2))),
            WalTransition::Unchanged
        );
        assert_eq!(
            wal_transition(WalState::Present(None), WalState::Present(None)),
            WalTransition::Unchanged
        );
    }

    #[tokio::test]
    async fn vanished_trips() {
        let (pool, temp_dir) = crate::test_utils::create_test_pool().await;
        let db_path = temp_dir.path().join("test.db");

        sqlx::query("INSERT INTO projects (id, name, git_repo_path) VALUES (randomblob(16), 'test', '/tmp/test')")
            .execute(&pool)
            .await
            .expect("failed to insert projects row");

        let wal_path = wal_path_for(&db_path);

        // Seed state BEFORE removing WAL
        let config = WalMonitorConfig::default();
        let metrics = crate::metrics::DbMetrics::new();
        let last_wal_state = match std::fs::metadata(&wal_path) {
            Ok(md) => WalState::Present(wal_identity(&md)),
            Err(_) => WalState::Absent,
        };
        let wal_ever_present = matches!(last_wal_state, WalState::Present(_));

        let mut mon = WalMonitor {
            db_path: db_path.clone(),
            pool,
            metrics,
            config,
            last_wal_state,
            wal_ever_present,
            tripped: false,
            trip_events: 0,
            guard: None,
            salvage_conn: None,
            last_salvage: None,
            refusal: None,
        };

        // Now remove the WAL and check
        let _removed = std::fs::remove_file(&wal_path);
        mon.check_wal_size().await;
        assert!(mon.tripped);
        assert_eq!(mon.trip_events, 1);
    }

    #[tokio::test]
    async fn replaced_trips() {
        let (pool, temp_dir) = crate::test_utils::create_test_pool().await;
        let db_path = temp_dir.path().join("test.db");

        sqlx::query("INSERT INTO projects (id, name, git_repo_path) VALUES (randomblob(16), 'test', '/tmp/test')")
            .execute(&pool)
            .await
            .expect("failed to insert projects row");

        let wal_path = wal_path_for(&db_path);
        let mut last_wal_state = match std::fs::metadata(&wal_path) {
            Ok(md) => WalState::Present(wal_identity(&md)),
            Err(_) => WalState::Absent,
        };
        assert!(matches!(last_wal_state, WalState::Present(_)));
        last_wal_state = WalState::Present(Some(u64::MAX));

        let mut mon = WalMonitor {
            db_path,
            pool,
            metrics: crate::metrics::DbMetrics::new(),
            config: WalMonitorConfig::default(),
            last_wal_state,
            wal_ever_present: true,
            tripped: false,
            trip_events: 0,
            guard: None,
            salvage_conn: None,
            last_salvage: None,
            refusal: None,
        };

        mon.check_wal_size().await;
        assert!(mon.tripped);
        assert_eq!(mon.trip_events, 1);
    }

    #[tokio::test]
    async fn no_wal_yet_does_not_trip() {
        let (pool, temp_dir) = crate::test_utils::create_test_pool().await;
        let db_path = temp_dir.path().join("test.db");

        let config = WalMonitorConfig::default();
        let metrics = crate::metrics::DbMetrics::new();
        let last_wal_state = WalState::Absent;
        let wal_ever_present = false;

        let mut mon = WalMonitor {
            db_path,
            pool,
            metrics,
            config,
            last_wal_state,
            wal_ever_present,
            tripped: false,
            trip_events: 0,
            guard: None,
            salvage_conn: None,
            last_salvage: None,
            refusal: None,
        };

        mon.check_wal_size().await;
        assert!(!mon.tripped);
    }

    #[tokio::test]
    async fn trip_is_idempotent() {
        let (pool, temp_dir) = crate::test_utils::create_test_pool().await;
        let db_path = temp_dir.path().join("test.db");

        sqlx::query("INSERT INTO projects (id, name, git_repo_path) VALUES (randomblob(16), 'test', '/tmp/test')")
            .execute(&pool)
            .await
            .expect("failed to insert projects row");

        let wal_path = wal_path_for(&db_path);

        // Seed state BEFORE removing WAL
        let config = WalMonitorConfig::default();
        let metrics = crate::metrics::DbMetrics::new();
        let last_wal_state = match std::fs::metadata(&wal_path) {
            Ok(md) => WalState::Present(wal_identity(&md)),
            Err(_) => WalState::Absent,
        };
        let wal_ever_present = matches!(last_wal_state, WalState::Present(_));

        let mut mon = WalMonitor {
            db_path: db_path.clone(),
            pool,
            metrics,
            config,
            last_wal_state,
            wal_ever_present,
            tripped: false,
            trip_events: 0,
            guard: None,
            salvage_conn: None,
            last_salvage: None,
            refusal: None,
        };

        // Remove WAL and trigger trip
        let _removed = std::fs::remove_file(&wal_path);
        mon.check_wal_size().await;
        assert_eq!(mon.trip_events, 1);

        // Remove it again and check idempotence
        let _removed = std::fs::remove_file(&wal_path);
        mon.check_wal_size().await;
        mon.check_wal_size().await;
        assert_eq!(mon.trip_events, 1);
    }

    // Serial: uses options_for connections (apply_performance_pragmas honours the
    // process-global refusal flag set by the refusal tests).
    #[tokio::test]
    #[serial_test::serial]
    async fn trip_runs_salvage_checkpoint() {
        use sqlx::ConnectOptions;
        let (pool, tmp) = crate::test_utils::create_test_pool().await;
        let db_path = tmp.path().join("test.db");
        sqlx::query("INSERT INTO projects (id, name, git_repo_path) VALUES (randomblob(16), 'salvage-probe', '/tmp/salvage-probe-uniq')").execute(&pool).await.unwrap();
        let wal = wal_path_for(&db_path);
        assert!(wal.exists());
        let md = std::fs::metadata(&wal).unwrap();
        let mut salvage_conn = crate::wal_guard::options_for(&db_path)
            .unwrap()
            .connect()
            .await
            .unwrap();
        sqlx::query("SELECT count(*) FROM sqlite_master")
            .fetch_one(&mut salvage_conn)
            .await
            .unwrap();
        let mut mon = WalMonitor {
            db_path: db_path.clone(),
            pool: pool.clone(),
            metrics: crate::metrics::DbMetrics::new(),
            config: WalMonitorConfig::default(),
            last_wal_state: WalState::Present(wal_identity(&md)),
            wal_ever_present: true,
            tripped: false,
            trip_events: 0,
            guard: None,
            salvage_conn: Some(salvage_conn),
            last_salvage: None,
            refusal: None,
        };
        std::fs::remove_file(&wal).unwrap();
        let _ = std::fs::remove_file(format!("{}-shm", db_path.display()));
        async fn main_file_probe(db_path: &std::path::Path) -> i64 {
            let p = sqlx::sqlite::SqlitePoolOptions::new()
                .max_connections(1)
                .connect_with(
                    crate::wal_guard::options_for(db_path)
                        .unwrap()
                        .immutable(true),
                )
                .await
                .unwrap();
            let has: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='projects'",
            )
            .fetch_one(&p)
            .await
            .unwrap();
            let n: i64 = if has == 0 {
                0
            } else {
                sqlx::query_scalar("SELECT count(*) FROM projects WHERE name='salvage-probe'")
                    .fetch_one(&p)
                    .await
                    .unwrap()
            };
            p.close().await;
            n
        }
        let before = main_file_probe(&db_path).await;
        assert_eq!(
            before, 0,
            "pre-trip main file already holds the row — no differential to measure (an earlier checkpoint flushed it); the A6 assertion would be hollow"
        );
        mon.check_wal_size().await;
        assert!(mon.tripped, "trip was not detected after WAL removal");
        assert!(
            mon.last_salvage.as_ref().is_some_and(|r| r.is_ok()),
            "salvage did not run through the dedicated connection: {:?}",
            mon.last_salvage
        );
        assert!(
            matches!(mon.last_salvage.as_ref(), Some(Ok((0, _, _)))),
            "salvage checkpoint reported busy: {:?}",
            mon.last_salvage
        );
        let after = main_file_probe(&db_path).await;
        assert_eq!(
            after, 1,
            "salvage checkpoint did not flush pre-trip frames into the main file (A6 pre-stop differential; before={before})"
        );
        pool.close().await;
        if let Some(latch) = mon.refusal.take() {
            latch.disarm();
        }
        drop(mon);
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn refusal_fence_survives_monitor_shutdown() {
        use sqlx::ConnectOptions;

        struct FlagGuard;
        impl Drop for FlagGuard {
            fn drop(&mut self) {
                crate::WAL_WRITE_REFUSAL_ACTIVE.store(false, std::sync::atomic::Ordering::SeqCst);
            }
        }

        let _flag_guard = FlagGuard;
        let (pool, tmp) = crate::test_utils::create_test_pool().await;
        let db_path = tmp.path().join("test.db");
        sqlx::query("INSERT INTO projects (id, name, git_repo_path) VALUES (randomblob(16), 'shutdown-probe', '/tmp/shutdown-probe-uniq')")
            .execute(&pool)
            .await
            .unwrap();
        let wal = wal_path_for(&db_path);
        let salvage_conn = crate::wal_guard::options_for(&db_path)
            .unwrap()
            .connect()
            .await
            .unwrap();
        let handle = WalMonitor::spawn(
            &db_path,
            pool.clone(),
            crate::metrics::DbMetrics::new(),
            WalMonitorConfig {
                truncate_checkpoint_interval_secs: 0,
                ..WalMonitorConfig::default()
            },
            None,
            Some(salvage_conn),
        );

        std::fs::remove_file(&wal).unwrap();
        let _ = std::fs::remove_file(tmp.path().join("test.db-shm"));
        handle.check_now().await;
        tokio::time::timeout(Duration::from_secs(5), async {
            while !crate::WAL_WRITE_REFUSAL_ACTIVE.load(std::sync::atomic::Ordering::SeqCst) {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("monitor did not arm the refusal fence");

        handle.shutdown().await;
        assert!(
            crate::WAL_WRITE_REFUSAL_ACTIVE.load(std::sync::atomic::Ordering::SeqCst),
            "monitor shutdown must preserve the write-refusal fence until process exit"
        );
        pool.close().await;
    }

    // Refusal tests touch the process-global WAL_WRITE_REFUSAL_ACTIVE flag. Serialize
    // them against each other and against any future flag user.
    #[tokio::test]
    #[serial_test::serial]
    async fn refusal_latch_blocks_writes_allows_reads() {
        use sqlx::ConnectOptions;
        let (pool, tmp) = crate::test_utils::create_test_pool().await;
        let db_path = tmp.path().join("test.db");
        // Hold one pool connection across unlink. sqlx retires a connection after a
        // locked write; the next `&pool` acquire is a FRESH post-unlink conn that
        // fails with SQLITE_IOERR (code 522), which is NOT a latch block. D6's
        // reads-continue is about OLD-domain pooled connections (execute-time
        // amendment 2026-08-30).
        let mut pooled = pool.acquire().await.unwrap();
        sqlx::query("INSERT INTO projects (id, name, git_repo_path) VALUES (randomblob(16), 'pre-latch', '/tmp/pre-latch-uniq')").execute(&mut *pooled).await.unwrap(); // forces the WAL into existence
        // Dedicated connection opened PRE-unlink (old shm/inode domain) — a fresh post-unlink conn would fence nobody.
        let mut conn = crate::wal_guard::options_for(&db_path)
            .unwrap()
            .connect()
            .await
            .unwrap();
        // Dummy read maps the wal-index — connect alone does not put the connection in the old shm domain.
        sqlx::query("SELECT count(*) FROM sqlite_master")
            .fetch_one(&mut conn)
            .await
            .unwrap();
        // integrated-review amendment 2026-08-30: fault injection removes BOTH files, matching the live harness.
        std::fs::remove_file(tmp.path().join("test.db-wal")).unwrap(); // REAL external unlink
        let _ = std::fs::remove_file(tmp.path().join("test.db-shm"));
        let latch = RefusalLatch::arm(conn)
            .await
            .expect("latch must arm on the old-domain connection");
        let write = sqlx::query("INSERT INTO projects (id, name, git_repo_path) VALUES (randomblob(16), 'refusal-probe', '/tmp/refusal-probe-uniq')").execute(&mut *pooled).await;
        let write_code = write
            .as_ref()
            .err()
            .and_then(|e| e.as_database_error())
            .and_then(|e| e.code())
            .map(|c| c.into_owned());
        assert_eq!(
            write_code.as_deref(),
            Some("5"),
            "write must be refused by the latch with SQLITE_BUSY (code 5), got {write:?}"
        );
        let read: Result<i64, sqlx::Error> = sqlx::query_scalar("SELECT count(*) FROM projects")
            .fetch_one(&mut *pooled)
            .await;
        assert!(
            read.is_ok(),
            "read blocked on the held old-domain connection: {read:?}"
        );
        drop(latch);
    }

    /// With the refusal flag set, a FRESH pooled connection (the new post-unlink
    /// WAL domain the BEGIN IMMEDIATE latch cannot fence) must be read-only:
    /// writes fail SQLITE_READONLY (code 8), reads continue (D6).
    #[tokio::test]
    #[serial_test::serial]
    async fn refusal_flag_fences_fresh_connections_read_only() {
        use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
        use std::str::FromStr;
        use std::time::Duration;
        let (seed, tmp) = crate::test_utils::create_test_pool().await;
        let db_path = tmp.path().join("test.db");
        seed.close().await;
        // Reset the global flag on every exit path — it is process-wide.
        struct FlagGuard;
        impl Drop for FlagGuard {
            fn drop(&mut self) {
                crate::WAL_WRITE_REFUSAL_ACTIVE.store(false, std::sync::atomic::Ordering::SeqCst);
            }
        }
        let _guard = FlagGuard;
        crate::WAL_WRITE_REFUSAL_ACTIVE.store(true, std::sync::atomic::Ordering::SeqCst);
        let options = SqliteConnectOptions::from_str(&format!("sqlite://{}", db_path.display()))
            .unwrap()
            .journal_mode(SqliteJournalMode::Wal)
            .busy_timeout(Duration::from_secs(30));
        let pool = crate::with_refusal_after_release(
            SqlitePoolOptions::new()
                .min_connections(1)
                .max_connections(2)
                .acquire_timeout(Duration::from_secs(5))
                .after_connect(|conn, _meta| {
                    Box::pin(async move { crate::apply_performance_pragmas(conn).await })
                }),
        )
        .connect_with(options)
        .await
        .unwrap();
        let write = sqlx::query("INSERT INTO projects (id, name, git_repo_path) VALUES (randomblob(16), 'refused-fresh', '/tmp/refused-fresh-uniq')")
            .execute(&pool)
            .await;
        let code = write
            .as_ref()
            .err()
            .and_then(|e| e.as_database_error())
            .and_then(|e| e.code())
            .map(|c| c.into_owned());
        assert_eq!(
            code.as_deref(),
            Some("8"),
            "fresh post-unlink pooled conn write must fail SQLITE_READONLY (code 8), got {write:?}"
        );
        let read: Result<i64, sqlx::Error> = sqlx::query_scalar("SELECT count(*) FROM projects")
            .fetch_one(&pool)
            .await;
        assert!(
            read.is_ok(),
            "read blocked on a fresh post-latch pooled conn: {read:?}"
        );
        pool.close().await;
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn refusal_latch_fail_fast_under_production_busy_timeout() {
        use sqlx::ConnectOptions;
        use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
        use std::str::FromStr;
        use std::time::Duration;
        // Start from a migrated temp DB, then reopen with the production 30s
        // busy_timeout + the same after_release hook the live pool uses. This pool
        // intentionally omits after_connect(apply_performance_pragmas) so the
        // old-domain SQLITE_BUSY path is isolated from the new-domain query_only
        // fence, covered by refusal_flag_fences_fresh_connections_read_only.
        let (seed, tmp) = crate::test_utils::create_test_pool().await;
        let db_path = tmp.path().join("test.db");
        seed.close().await;
        let options = SqliteConnectOptions::from_str(&format!("sqlite://{}", db_path.display()))
            .unwrap()
            .journal_mode(SqliteJournalMode::Wal)
            .busy_timeout(Duration::from_secs(30));
        let pool = crate::with_refusal_after_release(
            SqlitePoolOptions::new()
                .min_connections(1)
                .max_connections(5)
                .acquire_timeout(Duration::from_secs(5)),
        )
        .connect_with(options)
        .await
        .unwrap();
        sqlx::query("INSERT INTO projects (id, name, git_repo_path) VALUES (randomblob(16), 'pre-latch', '/tmp/pre-latch-uniq')")
            .execute(&pool)
            .await
            .unwrap();
        // Hold the old-domain pooled conn across unlink so ping/checkout cannot
        // replace it with a fresh 30s-timeout connection.
        let mut pooled = pool.acquire().await.unwrap();
        let mut conn = crate::wal_guard::options_for(&db_path)
            .unwrap()
            .connect()
            .await
            .unwrap();
        sqlx::query("SELECT count(*) FROM sqlite_master")
            .fetch_one(&mut conn)
            .await
            .unwrap();
        std::fs::remove_file(tmp.path().join("test.db-wal")).unwrap();
        let _ = std::fs::remove_file(tmp.path().join("test.db-shm"));
        let latch = RefusalLatch::arm(conn)
            .await
            .expect("latch must arm on the old-domain connection");
        zero_pooled_busy_timeout(&pool).await;
        sqlx::query("PRAGMA busy_timeout = 0")
            .execute(&mut *pooled)
            .await
            .unwrap();
        let write = tokio::time::timeout(
            Duration::from_secs(3),
            sqlx::query("INSERT INTO projects (id, name, git_repo_path) VALUES (randomblob(16), 'refusal-probe', '/tmp/refusal-probe-uniq')")
                .execute(&mut *pooled),
        )
        .await
        .expect("latched write waited past 3s — busy_timeout still the production 30s window");
        let write_code = write
            .as_ref()
            .err()
            .and_then(|e| e.as_database_error())
            .and_then(|e| e.code())
            .map(|c| c.into_owned());
        assert_eq!(
            write_code.as_deref(),
            Some("5"),
            "write must be SQLITE_BUSY (code 5) inside the 10s client deadline, got {write:?}"
        );
        drop(pooled);
        let pool_write = tokio::time::timeout(
            Duration::from_secs(3),
            sqlx::query("INSERT INTO projects (id, name, git_repo_path) VALUES (randomblob(16), 'refusal-pool', '/tmp/refusal-pool-uniq')")
                .execute(&pool),
        )
        .await
        .expect("pooled write after release waited past 3s — after_release busy_timeout not zero");
        let pool_code = pool_write
            .as_ref()
            .err()
            .and_then(|e| e.as_database_error())
            .and_then(|e| e.code())
            .map(|c| c.into_owned());
        assert_eq!(
            pool_code.as_deref(),
            Some("5"),
            "pool checkout after latch must be SQLITE_BUSY (code 5), got {pool_write:?}"
        );
        latch.disarm();
        pool.close().await;
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn is_wal_removal_matches_delete_and_rename_from() {
        use notify::event::{EventKind, ModifyKind, RemoveKind, RenameMode};
        let wal = std::path::PathBuf::from("/x/db.sqlite-wal");
        let other = std::path::PathBuf::from("/x/db.sqlite");
        assert!(is_wal_removal(
            &EventKind::Remove(RemoveKind::File),
            std::slice::from_ref(&wal),
            "db.sqlite-wal"
        ));
        assert!(is_wal_removal(
            &EventKind::Modify(ModifyKind::Name(RenameMode::From)),
            std::slice::from_ref(&wal),
            "db.sqlite-wal"
        ));
        assert!(!is_wal_removal(
            &EventKind::Remove(RemoveKind::File),
            std::slice::from_ref(&other),
            "db.sqlite-wal"
        ));
        assert!(!is_wal_removal(
            &EventKind::Create(notify::event::CreateKind::File),
            std::slice::from_ref(&wal),
            "db.sqlite-wal"
        ));
    }
}
