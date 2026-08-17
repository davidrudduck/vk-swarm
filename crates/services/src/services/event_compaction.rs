//! Event journal compaction service.
//!
//! This module runs a periodic compaction task on the event journal, deleting
//! old events based on retention time and row count thresholds, while ensuring
//! active consumers (trigger cursors) retain enough history.
//!
//! # Design
//!
//! - Runs as a background task on a configurable interval
//! - Reads retention policy from environment variables with sensible defaults
//! - Calls the underlying compaction function with validated parameters
//! - Logs compaction results and any configuration warnings
//! - Sanitizes parameters to prevent silent data loss or refusal-to-start

use std::panic::AssertUnwindSafe;
use std::time::Duration;

use db::models::event_journal::EventJournalError;
use futures::FutureExt;
use sqlx::SqlitePool;
use tokio::sync::mpsc;

/// Default event retention window in hours (7 days).
const DEFAULT_RETENTION_HOURS: i64 = 168;

/// Default minimum rows to retain in the journal.
const DEFAULT_MIN_ROWS: i64 = 10000;

/// Default hard cap on journal size (triggers cleanup regardless of cursors).
const DEFAULT_MAX_ROWS: i64 = 100000;

/// Configuration for event journal compaction.
#[derive(Clone, Debug)]
pub struct EventCompactionConfig {
    /// Event retention window in hours (events older than this are eligible for deletion).
    pub retention_hours: i64,
    /// Minimum rows to retain (even if old, respects cursor floor).
    pub min_rows: i64,
    /// Hard cap on journal size (rows above this are deleted regardless).
    pub max_rows: i64,
    /// How often to run compaction (in seconds).
    pub compaction_interval_secs: u64,
}

impl Default for EventCompactionConfig {
    fn default() -> Self {
        parse_event_compaction_config(
            std::env::var("VK_EVENT_RETENTION_HOURS").ok(),
            std::env::var("VK_EVENT_MIN_ROWS").ok(),
            std::env::var("VK_EVENT_MAX_ROWS").ok(),
            std::env::var("VK_EVENT_COMPACTION_INTERVAL_SECS").ok(),
        )
    }
}

/// Sanitize the row thresholds so `compact` can never be handed a value that
/// would silently empty the journal.
///
/// `EventCompactionConfig`'s fields are public, so a caller-constructed config
/// can reach `compact` without ever passing through
/// [`parse_event_compaction_config`]. This function is therefore applied on
/// BOTH paths: at parse time, and again immediately before every `compact`
/// call.
///
/// # Sanitization Order
///
/// 1. Clamp `min_rows` to >= 1
/// 2. Clamp `max_rows` to >= 1
/// 3. Clamp `max_rows` up to >= `min_rows` if below
///
/// Each clamp emits a `tracing::warn!` with the variable name and both the
/// configured (operator-set, pre-clamp) and effective values.
fn sanitise_rows(mut min_rows: i64, mut max_rows: i64) -> (i64, i64) {
    // Captured before ANY clamp fires, so the warns always report the value the
    // operator actually configured rather than an intermediate clamped value.
    let min_rows_configured = min_rows;
    let max_rows_configured = max_rows;

    // Sanitization stage 1: clamp min_rows to >= 1
    if min_rows < 1 {
        tracing::warn!(
            variable = "VK_EVENT_MIN_ROWS",
            configured = min_rows_configured,
            effective = 1,
            "Clamping VK_EVENT_MIN_ROWS to minimum 1 (configured value below 1 would allow journal to empty)"
        );
        min_rows = 1;
    }

    // Sanitization stage 2: clamp max_rows to >= 1
    if max_rows < 1 {
        tracing::warn!(
            variable = "VK_EVENT_MAX_ROWS",
            configured = max_rows_configured,
            effective = 1,
            "Clamping VK_EVENT_MAX_ROWS to minimum 1 (configured value below 1 would allow journal to empty)"
        );
        max_rows = 1;
    }

    // Sanitization stage 3: clamp max_rows UP to min_rows if below
    if max_rows < min_rows {
        max_rows = min_rows;
        tracing::warn!(
            variable = "VK_EVENT_MAX_ROWS",
            configured = max_rows_configured,
            min_rows = min_rows,
            effective = max_rows,
            "Clamping VK_EVENT_MAX_ROWS up to VK_EVENT_MIN_ROWS (hard cap cannot be less than minimum)"
        );
    }

    (min_rows, max_rows)
}

/// Parse event compaction configuration from raw environment strings.
///
/// This is a pure function to avoid environment variable mutation in tests.
/// It sanitizes all values via [`sanitise_rows`] and logs warnings for any
/// clamping that occurs.
fn parse_event_compaction_config(
    retention_hours_str: Option<String>,
    min_rows_str: Option<String>,
    max_rows_str: Option<String>,
    interval_secs_str: Option<String>,
) -> EventCompactionConfig {
    let retention_hours = parse_i64_or_default(
        &retention_hours_str,
        DEFAULT_RETENTION_HOURS,
        "VK_EVENT_RETENTION_HOURS",
    );

    let min_rows_raw = parse_i64_or_default(&min_rows_str, DEFAULT_MIN_ROWS, "VK_EVENT_MIN_ROWS");

    let max_rows_raw = parse_i64_or_default(&max_rows_str, DEFAULT_MAX_ROWS, "VK_EVENT_MAX_ROWS");

    let interval_secs = parse_u64_or_default(
        &interval_secs_str,
        60, // Compact every 60 seconds by default
        "VK_EVENT_COMPACTION_INTERVAL_SECS",
    );

    let (min_rows, max_rows) = sanitise_rows(min_rows_raw, max_rows_raw);

    EventCompactionConfig {
        retention_hours,
        min_rows,
        max_rows,
        compaction_interval_secs: interval_secs,
    }
}

/// Parse an i64 from an Option<String>, defaulting if absent or unparseable.
fn parse_i64_or_default(val: &Option<String>, default: i64, var_name: &str) -> i64 {
    match val {
        Some(s) => s.parse::<i64>().unwrap_or_else(|_| {
            tracing::warn!(
                variable = var_name,
                configured = s,
                default = default,
                "Invalid numeric value for {var_name}; falling back to default"
            );
            default
        }),
        None => default,
    }
}

/// Parse a u64 from an Option<String>, defaulting if absent or unparseable.
fn parse_u64_or_default(val: &Option<String>, default: u64, var_name: &str) -> u64 {
    match val {
        Some(s) => s.parse::<u64>().unwrap_or_else(|_| {
            tracing::warn!(
                variable = var_name,
                configured = s,
                default = default,
                "Invalid numeric value for {var_name}; falling back to default"
            );
            default
        }),
        None => default,
    }
}

/// Handle for controlling the event compaction service.
#[derive(Clone)]
pub struct EventCompactionHandle {
    tx: mpsc::Sender<EventCompactionCommand>,
}

enum EventCompactionCommand {
    /// Request immediate compaction run.
    CompactNow,
    /// Shutdown the compaction service.
    Shutdown,
}

impl EventCompactionHandle {
    /// Request an immediate compaction run.
    pub async fn compact_now(&self) {
        let _ = self.tx.send(EventCompactionCommand::CompactNow).await;
    }

    /// Shutdown the event compaction service.
    pub async fn shutdown(&self) {
        let _ = self.tx.send(EventCompactionCommand::Shutdown).await;
    }
}

/// Event journal compaction service.
pub struct EventCompaction {
    pool: SqlitePool,
    config: EventCompactionConfig,
}

impl EventCompaction {
    /// Spawn a new event compaction service as a background task.
    ///
    /// Returns a handle that can be used to control the service.
    pub fn spawn(pool: SqlitePool, config: EventCompactionConfig) -> EventCompactionHandle {
        let (tx, rx) = mpsc::channel(16);
        let service = Self { pool, config };
        tokio::spawn(async move {
            let _ = supervised_run("event_compaction", service.run(rx)).await;
        });
        EventCompactionHandle { tx }
    }

    /// Spawn with default configuration.
    pub fn spawn_default(pool: SqlitePool) -> EventCompactionHandle {
        Self::spawn(pool, EventCompactionConfig::default())
    }

    async fn run(self, mut rx: mpsc::Receiver<EventCompactionCommand>) {
        let mut compaction_interval =
            tokio::time::interval(Duration::from_secs(self.config.compaction_interval_secs));

        tracing::info!(
            retention_hours = self.config.retention_hours,
            min_rows = self.config.min_rows,
            max_rows = self.config.max_rows,
            interval_secs = self.config.compaction_interval_secs,
            "Event compaction service started"
        );

        // Skip first immediate tick to let the server stabilize
        compaction_interval.tick().await;

        loop {
            tokio::select! {
                Some(cmd) = rx.recv() => {
                    match cmd {
                        EventCompactionCommand::CompactNow => {
                            // Errors are already logged inside run_compaction; the
                            // loop must never crash on a failed compaction pass.
                            let _ = self.run_compaction().await;
                        }
                        EventCompactionCommand::Shutdown => {
                            tracing::info!("Event compaction service shutting down");
                            break;
                        }
                    }
                }
                _ = compaction_interval.tick() => {
                    // Errors are already logged inside run_compaction; the loop
                    // must never crash on a failed compaction pass.
                    let _ = self.run_compaction().await;
                }
            }
        }
    }

    /// Run one compaction pass, returning the number of rows deleted.
    ///
    /// The row thresholds are re-sanitised here rather than trusted from
    /// `self.config`: `EventCompactionConfig`'s fields are public, so a
    /// caller-constructed config could otherwise hand `compact` a raw `0` and
    /// let stage 2 empty the journal while flagging nobody.
    async fn run_compaction(&self) -> Result<u64, EventJournalError> {
        let start = std::time::Instant::now();

        let (min_rows, max_rows) = sanitise_rows(self.config.min_rows, self.config.max_rows);

        match db::models::event_journal::compact(
            &self.pool,
            self.config.retention_hours,
            min_rows,
            max_rows,
        )
        .await
        {
            Ok(deleted_count) => {
                let duration = start.elapsed();
                if deleted_count > 0 {
                    tracing::info!(
                        deleted_rows = deleted_count,
                        duration_ms = duration.as_millis() as u64,
                        retention_hours = self.config.retention_hours,
                        min_rows = min_rows,
                        max_rows = max_rows,
                        "Event journal compaction completed"
                    );
                } else {
                    tracing::debug!(
                        duration_ms = duration.as_millis() as u64,
                        "Event journal compaction completed (no rows deleted)"
                    );
                }
                Ok(deleted_count)
            }
            Err(e) => {
                let duration = start.elapsed();
                tracing::error!(
                    error = ?e,
                    duration_ms = duration.as_millis() as u64,
                    "Event journal compaction failed"
                );
                Err(e)
            }
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
    use tracing_test::traced_test;

    // Test 1: reads_retention_defaults_when_env_absent
    //
    // The D6 defaults are pinned LITERALLY, not against the constants: a
    // mutated constant must fail this test rather than move the goalposts.
    #[test]
    fn reads_retention_defaults_when_env_absent() {
        let config = parse_event_compaction_config(None, None, None, None);
        assert_eq!(config.retention_hours, 168);
        assert_eq!(config.min_rows, 10000);
        assert_eq!(config.max_rows, 100000);
        assert_eq!(config.compaction_interval_secs, 60);
    }

    // Test 2: env_overrides_are_respected
    #[test]
    fn env_overrides_are_respected() {
        let config = parse_event_compaction_config(
            Some("72".to_string()),
            Some("5000".to_string()),
            Some("50000".to_string()),
            Some("30".to_string()),
        );
        assert_eq!(config.retention_hours, 72);
        assert_eq!(config.min_rows, 5000);
        assert_eq!(config.max_rows, 50000);
        assert_eq!(config.compaction_interval_secs, 30);
    }

    // Test 3: invalid_env_falls_back_to_default_and_warns
    //
    // Defaults pinned literally (see test 1). The warn is asserted, not merely
    // assumed: a silent fallback gives the operator no signal at startup.
    #[test]
    #[traced_test]
    fn invalid_env_falls_back_to_default_and_warns() {
        let config = parse_event_compaction_config(
            Some("not_a_number".to_string()),
            Some("also_invalid".to_string()),
            Some("definitely_invalid".to_string()),
            Some("nope".to_string()),
        );
        assert_eq!(config.retention_hours, 168);
        assert_eq!(config.min_rows, 10000);
        assert_eq!(config.max_rows, 100000);
        assert_eq!(config.compaction_interval_secs, 60);

        assert!(
            logs_contain("VK_EVENT_RETENTION_HOURS"),
            "the fallback warn must name the offending variable"
        );
        assert!(
            logs_contain("VK_EVENT_MIN_ROWS"),
            "the fallback warn must name the offending variable"
        );
        assert!(
            logs_contain("VK_EVENT_MAX_ROWS"),
            "the fallback warn must name the offending variable"
        );
        assert!(
            logs_contain("VK_EVENT_COMPACTION_INTERVAL_SECS"),
            "the fallback warn must name the offending variable"
        );
    }

    // Test 4: compaction_run_is_a_no_op_on_an_empty_journal
    #[tokio::test]
    async fn compaction_run_is_a_no_op_on_an_empty_journal() {
        let (pool, _temp_dir) = db::test_utils::create_test_pool_with_migrations().await;
        let config = EventCompactionConfig {
            retention_hours: 168,
            min_rows: 10000,
            max_rows: 100000,
            compaction_interval_secs: 60,
        };
        let service = EventCompaction {
            pool: pool.clone(),
            config,
        };

        let deleted = service
            .run_compaction()
            .await
            .expect("compaction on an empty journal must succeed");
        assert_eq!(deleted, 0, "an empty journal has nothing to delete");

        let count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM event_journal")
            .fetch_one(&pool)
            .await
            .expect("failed to count events");
        assert_eq!(
            count, 0,
            "the journal must still be empty after a no-op run"
        );
    }

    // Test 5: max_rows_below_min_rows_is_clamped_with_a_warning
    #[test]
    #[traced_test]
    fn max_rows_below_min_rows_is_clamped_with_a_warning() {
        let config = parse_event_compaction_config(
            Some("168".to_string()),
            Some("50000".to_string()),
            Some("5000".to_string()), // max < min
            None,
        );
        // max_rows should be clamped UP to min_rows
        assert_eq!(config.min_rows, 50000);
        assert_eq!(config.max_rows, 50000);

        assert!(
            logs_contain("VK_EVENT_MAX_ROWS"),
            "the clamp warn must name the offending variable"
        );
        assert!(
            logs_contain("configured=5000"),
            "the clamp warn must report the operator-configured value"
        );
        assert!(
            logs_contain("effective=50000"),
            "the clamp warn must report the effective value"
        );
    }

    // Test 6: max_rows_of_zero_is_clamped_to_at_least_one
    #[tokio::test]
    #[traced_test]
    async fn max_rows_of_zero_is_clamped_to_at_least_one() {
        let config = parse_event_compaction_config(
            Some("168".to_string()),
            Some("10000".to_string()),
            Some("0".to_string()), // max = 0 should be clamped
            None,
        );
        // max_rows should be clamped to min_rows (which is 10000)
        assert!(config.max_rows >= 1);
        assert_eq!(config.max_rows, config.min_rows);

        // Both stage 2 and stage 3 fire here, so this is the only case that can
        // observe stage 3 reporting the OPERATOR-set value rather than stage 2's
        // already-clamped intermediate. A regression reports `configured=1`.
        assert!(
            logs_contain("configured=0"),
            "the clamp warns must report the operator-configured 0"
        );
        // NOTE: substring match — this would also fire on `configured=10000`. It
        // is sound here only because stage 1 does not warn in this test (min_rows
        // is 10000). Changing this test's min_rows input to trip stage 1 would
        // make the failure look like a regression when it is not.
        assert!(
            !logs_contain("configured=1"),
            "no warn may report stage 2's intermediate clamp as the configured value"
        );

        // Verify that a compaction run doesn't actually delete everything
        let (pool, _temp_dir) = db::test_utils::create_test_pool_with_migrations().await;
        let service = EventCompaction {
            pool: pool.clone(),
            config,
        };

        // Insert a test event manually (runtime API)
        sqlx::query("INSERT INTO event_journal (event_type, payload) VALUES (?, ?)")
            .bind("TestEvent")
            .bind("{}")
            .execute(&pool)
            .await
            .expect("failed to insert test event");

        // Run compaction
        service
            .run_compaction()
            .await
            .expect("compaction must succeed");

        // Verify the event is still there (journal is not empty)
        let count: i64 = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM event_journal")
            .fetch_one(&pool)
            .await
            .expect("failed to count events");
        assert!(
            count > 0,
            "journal should not be empty after compaction with max_rows=0"
        );
    }

    // Test 7: min_rows_of_zero_is_clamped_to_at_least_one
    //
    // Covers sanitisation stage 1, which the max_rows tests never reach.
    #[test]
    #[traced_test]
    fn min_rows_of_zero_is_clamped_to_at_least_one() {
        let config = parse_event_compaction_config(None, Some("0".to_string()), None, None);
        assert_eq!(config.min_rows, 1, "min_rows must be floored at 1");

        assert!(
            logs_contain("VK_EVENT_MIN_ROWS"),
            "the clamp warn must name the offending variable"
        );
    }

    // Test 8: the env-reading seam itself.
    //
    // Every other config test drives the pure parse function, so a typo in an
    // env var NAME would pass green. Mirrors wal_monitor.rs's
    // `test_default_config`, but pins the D6 values literally.
    #[test]
    fn default_config_uses_d6_defaults_when_env_absent() {
        let config = EventCompactionConfig::default();
        assert_eq!(config.retention_hours, 168);
        assert_eq!(config.min_rows, 10000);
        assert_eq!(config.max_rows, 100000);
    }
}
