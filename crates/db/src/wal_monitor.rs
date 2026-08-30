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
use sqlx::SqlitePool;
use tokio::sync::mpsc;

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
        (WalState::Present(Some(a)), WalState::Present(Some(b))) if a != b => WalTransition::Replaced,
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
    ) -> WalMonitorHandle {
        Self::spawn(db_path, pool, metrics, WalMonitorConfig::default(), guard)
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
                    if let Some(guard) = &mut self.guard {
                        if let Err(e) = guard.reacquire_read_mark().await {
                            tracing::error!(error = ?e, "failed to reacquire WAL guard read mark");
                        }
                    }
                }
            }
            
            if let Some(guard) = &mut self.guard {
                if !guard.is_alive().await {
                    match guard.reconnect().await {
                        Ok(_) => tracing::warn!(event = "wal_guard_reconnected", "WAL guard reconnected"),
                        Err(e) => {
                            tracing::error!(error = ?e, "WAL guard reconnect failed");
                            if !self.tripped {
                                self.tripped = true;
                                self.trip_events += 1;
                                tracing::error!(event = "wal_guard_unavailable", "WAL guard unavailable; treating as durability trip");
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
        
        if self.tripped {
            let current = match std::fs::metadata(&wal_path) {
                Ok(md) => WalState::Present(wal_identity(&md)),
                Err(_) => WalState::Absent,
            };
            self.last_wal_state = current;
            return;
        }
        
        let current = match std::fs::metadata(&wal_path) {
            Ok(md) => WalState::Present(wal_identity(&md)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => WalState::Absent,
            Err(e) => {
                tracing::warn!(
                    error = ?e,
                    path = ?wal_path,
                    "Failed to read WAL file metadata"
                );
                return;
            }
        };
        
        let transition = wal_transition(self.last_wal_state, current);
        
        match transition {
            WalTransition::Appeared => {
                self.wal_ever_present = true;
                self.last_wal_state = current;
                let wal_size = match std::fs::metadata(&wal_path) {
                    Ok(meta) => meta.len(),
                    Err(_) => 0,
                };
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
                }
            }
            WalTransition::Unchanged | WalTransition::Replaced if matches!(current, WalState::Present(_)) => {
                self.wal_ever_present = true;
                let wal_size = match std::fs::metadata(&wal_path) {
                    Ok(meta) => meta.len(),
                    Err(_) => 0,
                };
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
            WalTransition::Vanished | WalTransition::Replaced | _ if matches!(current, WalState::Absent) && self.wal_ever_present => {
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
            WalTransition::Unchanged if matches!(current, WalState::Absent) && !self.wal_ever_present => {
                tracing::debug!("WAL not yet present (benign)");
                self.last_wal_state = current;
            }
            _ => {
                self.last_wal_state = current;
            }
        }
    }

    async fn handle_trip(&mut self) {
        // In this task, only emit event + set tripped.
        // Tasks 030/031 extend with salvage and refusal latch.
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
        assert_eq!(wal_path_for(Path::new("/tmp/test.db")), PathBuf::from("/tmp/test.db-wal"));
        assert_eq!(wal_path_for(Path::new("/data/db.sqlite")), PathBuf::from("/data/db.sqlite-wal"));
    }

    #[test]
    fn wal_transition_classifies_all_cases() {
        assert_eq!(wal_transition(WalState::Absent, WalState::Absent), WalTransition::Unchanged);
        assert_eq!(wal_transition(WalState::Absent, WalState::Present(None)), WalTransition::Appeared);
        assert_eq!(wal_transition(WalState::Present(None), WalState::Absent), WalTransition::Vanished);
        assert_eq!(wal_transition(WalState::Present(Some(1)), WalState::Present(Some(2))), WalTransition::Replaced);
        assert_eq!(wal_transition(WalState::Present(Some(2)), WalState::Present(Some(2))), WalTransition::Unchanged);
        assert_eq!(wal_transition(WalState::Present(None), WalState::Present(None)), WalTransition::Unchanged);
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
        };

        // Now remove the WAL and check
        let _removed = std::fs::remove_file(&wal_path);
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
}
