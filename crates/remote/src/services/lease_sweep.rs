//! Background service for sweeping expired Hive leases.
//!
//! Reclaims assignments whose `lease_expires_at` has lapsed: bumps each to a
//! strictly-higher fencing token (so the prior holder's late writes are stale
//! per ADR-0009 / SC3) and clears the lease so a subsequent `try_claim` (or a
//! dispatcher) can take it. The token bump alone is what bounces a partitioned
//! writer; the lease is freed but NOT reassigned here.

use sqlx::PgPool;
use std::time::Duration as StdDuration;
use tokio::time::{self, MissedTickBehavior};
use tracing::{debug, error, info};

use crate::db::task_assignments::{TaskAssignmentError, TaskAssignmentRepository};

/// Configuration for the lease sweep service.
#[derive(Debug, Clone)]
pub struct LeaseSweepConfig {
    /// How often to run the sweep (default: 10 seconds — shorter than the 60s
    /// lease TTL so reclaim is timely).
    pub sweep_interval: StdDuration,
}

impl Default for LeaseSweepConfig {
    fn default() -> Self {
        Self {
            sweep_interval: StdDuration::from_secs(10),
        }
    }
}

/// Spawn the lease sweep service as a background task.
///
/// Runs periodically and reclaims expired leases, bumping each fencing token
/// via `nextval('node_fencing_token_seq')` so the prior holder's late writes
/// are bounced.
pub fn spawn_lease_sweep_service(pool: PgPool, config: Option<LeaseSweepConfig>) {
    let config = config.unwrap_or_default();

    tokio::spawn(async move {
        let mut interval = time::interval(config.sweep_interval);
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

        loop {
            interval.tick().await;

            match sweep_expired(&pool).await {
                Ok(reclaimed) => {
                    if reclaimed > 0 {
                        info!(reclaimed = reclaimed, "Reclaimed expired Hive leases");
                    } else {
                        debug!("No expired Hive leases to reclaim");
                    }
                }
                Err(e) => {
                    error!(error = ?e, "Failed to reclaim expired Hive leases");
                }
            }
        }
    });
}

/// Reclaim expired leases from the database.
async fn sweep_expired(pool: &PgPool) -> Result<u64, sqlx::Error> {
    let reclaimed = TaskAssignmentRepository::new(pool)
        .reclaim_expired_leases()
        .await
        .map_err(|e| match e {
            TaskAssignmentError::Database(e) => e,
            // reclaim_expired_leases never returns NotFound / AlreadyAssigned
            _ => sqlx::Error::Protocol("unexpected TaskAssignmentError variant".into()),
        })?;

    Ok(reclaimed.len() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = LeaseSweepConfig::default();
        assert_eq!(config.sweep_interval.as_secs(), 10);
    }
}
