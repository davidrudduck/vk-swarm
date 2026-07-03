---
id: "209"
phase: 2
title: Hive lease-expiry sweep — reclaim expired leases with a bumped fencing token (background timer)
status: done
depends_on: ["201", "203"]
parallel: false
conflicts_with: ["203"]
files:
  - crates/remote/src/db/task_assignments.rs
  - crates/remote/src/services/lease_sweep.rs
  - crates/remote/src/services/mod.rs
  - crates/remote/src/app.rs
irreversible: false
scope_test: "crates/remote/src/db/task_assignments.rs"
allowed_change: mixed
covers_criteria: [SC3]
covers_tests: []
---
## Failing test (write first)
**PRECONDITION (Trap 2b — NON-NEGOTIABLE):** the reclaim-query test is a `#[cfg(test)] mod` INSIDE
`task_assignments.rs` (the repo method is what we test) and REQUIRES a live, migrated Postgres (the `201`
columns + `node_fencing_token_seq`). A run without `DATABASE_URL` returns early (skip) = HOLLOW pass. Stand
up Postgres, export `DATABASE_URL=postgres://…`, or RAISE. The timer service (`lease_sweep.rs`) is a thin
analog of `stale_cleanup.rs` — its only unit-testable part is the default config (mirror
`stale_cleanup.rs`'s `test_default_config`, hermetic).

**Sibling read (rubric #9):**
- `crates/remote/src/services/stale_cleanup.rs` (the WHOLE file) is the timer template: `*Config` struct +
  `Default`, `spawn_*_service(pool, Option<Config>)` with `tokio::spawn` + `time::interval` +
  `MissedTickBehavior::Skip`, a private `cleanup_*` fn, and a `#[cfg(test)] mod tests { test_default_config }`.
  Copy its structure verbatim, renamed for leases.
- `crates/remote/tests/backfill_e2e.rs` for the `database_url()`/`skip_without_db!`/`create_pool()` helpers
  (inline into the `task_assignments.rs` test module).

Add to `task_assignments.rs` (the reclaim-query test, beside 203's `lease_tests` — keep it in a
`sweep_tests` module to avoid colliding with 203's module name):
```rust
#[cfg(test)]
mod sweep_tests {
    use super::*;
    // inline database_url / skip_without_db! / create_pool + org/node fixtures (as 203).

    #[tokio::test]
    async fn reclaim_expired_leases_bumps_token_and_returns_reclaimed_assignments() {
        skip_without_db!();
        let pool = create_pool().await;
        let repo = TaskAssignmentRepository::new(&pool);
        // node_a claims a task with an ALREADY-PAST TTL (lease_expires_at < now).
        let a = repo.try_claim(task_id, node_a, np_id, chrono::Duration::seconds(-1))
            .await.unwrap().expect("a wins");
        // Sweep reclaims expired leases.
        let reclaimed = repo.reclaim_expired_leases().await.unwrap();
        assert!(reclaimed.iter().any(|r| r.assignment_id == a.assignment_id),
            "an expired lease is reclaimed by the sweep");
        let r = reclaimed.iter().find(|r| r.assignment_id == a.assignment_id).unwrap();
        assert!(r.fencing_token > a.fencing_token,
            "reclaim bumps the fencing token strictly higher (so the old holder's late ops are stale)");
    }

    #[tokio::test]
    async fn sweep_does_not_touch_live_leases() {
        skip_without_db!();
        let pool = create_pool().await;
        let repo = TaskAssignmentRepository::new(&pool);
        let _live = repo.try_claim(task_id, node_a, np_id, chrono::Duration::seconds(300))
            .await.unwrap().expect("a wins");
        let reclaimed = repo.reclaim_expired_leases().await.unwrap();
        assert!(reclaimed.iter().all(|r| r.task_id != task_id), "a live lease is not reclaimed");
    }
}
```

## Change
### 1. `crates/remote/src/db/task_assignments.rs` — the reclaim query
- **Anchor:** add `reclaim_expired_leases` inside `impl<'a> TaskAssignmentRepository<'a>`, beside
  `fail_node_assignments` (@329-349, the existing `UPDATE … WHERE completed_at IS NULL RETURNING` read with
  `.get(...)` — the exact pattern to mirror). Reuse the `LeaseClaim` struct from 203 (this file).
- **Sibling read (rubric #9):** `fail_node_assignments` (@329-349) — runtime `sqlx::query` + `.fetch_all` +
  `rows.iter().map(|r| r.get(...))`. Read it before authoring.
```rust
    /// Reclaim all leases whose expiry has lapsed: bump each to a strictly-higher fencing token so the
    /// prior holder's late ops are stale (ADR-0009 / CONTRACT §C). Returns the reclaimed assignments.
    /// Does NOT reassign to a new node here — it frees the lease (and advances the token) so the next
    /// try_claim (or a dispatcher) can take it; the token bump alone is what bounces the partitioned
    /// writer. Mirrors fail_node_assignments' runtime-query + RETURNING shape.
    pub async fn reclaim_expired_leases(&self) -> Result<Vec<LeaseClaim>, TaskAssignmentError> {
        // UPDATE node_task_assignments
        //   SET fencing_token = nextval('node_fencing_token_seq'), lease_expires_at = NULL
        //   WHERE completed_at IS NULL AND lease_expires_at IS NOT NULL AND lease_expires_at < now()
        //   RETURNING id, node_id, task_id, fencing_token, lease_expires_at
        // .fetch_all → map each row into LeaseClaim with .get(...). (lease_expires_at is NULL post-reclaim;
        //  LeaseClaim.lease_expires_at is non-Option — if so, either make it Option or drop it from the
        //  RETURNING map for this method. Decide and record; the test only asserts assignment_id/token.)
        todo!("executor finalizes the reclaim UPDATE per the contract above")
    }
```
  > **LeaseClaim shape note:** 203's `LeaseClaim.lease_expires_at: DateTime<Utc>` is non-Option. The reclaim
  > sets it NULL. Either widen `LeaseClaim.lease_expires_at` to `Option` in 203 (coordinate — 209
  > depends_on 203) OR have `reclaim_expired_leases` return a narrower struct. Prefer making 203's
  > `LeaseClaim.lease_expires_at` `Option<DateTime<Utc>>` from the start and adjust 203's asserts to
  > `.unwrap()` — record this cross-task coordination in the ledger. (This is why 209 conflicts_with 203.)

### 2. `crates/remote/src/services/lease_sweep.rs` (NEW) — the timer
- Copy `stale_cleanup.rs`'s structure verbatim, renamed: `LeaseSweepConfig { sweep_interval: StdDuration }`
  + `Default` (e.g. 10s — shorter than the lease TTL so reclaim is timely), `spawn_lease_sweep_service(pool:
  PgPool, config: Option<LeaseSweepConfig>)` with `tokio::spawn` + `time::interval` +
  `MissedTickBehavior::Skip`, a private `async fn sweep_expired(pool: &PgPool) -> Result<u64, sqlx::Error>`
  that calls `TaskAssignmentRepository::new(pool).reclaim_expired_leases()` and returns the count, logging
  reclaimed ids. Include the `#[cfg(test)] mod tests { test_default_config }`.

### 3. `crates/remote/src/services/mod.rs` — export
- **Anchor:** the existing `pub mod stale_cleanup;` (@4) + `pub use stale_cleanup::{…}` (@7).
- **After:** add `pub mod lease_sweep;` and `pub use lease_sweep::{LeaseSweepConfig, spawn_lease_sweep_service};`.

### 4. `crates/remote/src/app.rs` — spawn at startup
- **Anchor:** the `spawn_stale_cleanup_service(pool.clone(), None);` call (@41) and its `use` (@18).
- **After:** add `spawn_lease_sweep_service(pool.clone(), None);` beside it, and add the symbol to the
  `services::{…}` import (@18).

## Allowed moves
ONLY: add `reclaim_expired_leases` to `task_assignments.rs` (+ its `sweep_tests`), create `lease_sweep.rs`
(stale_cleanup analog), export it from `services/mod.rs`, and spawn it in `app.rs`. Reuse `LeaseClaim`
(203), `TaskAssignmentRepository`, the `stale_cleanup` timer pattern. Do NOT touch `try_claim`/`renew_lease`
bodies (203 owns them; 209 only adds a sibling method to the same file → conflicts_with 203), the WS
protocol (202/204/205), the node side, or the migration (201 owns it).

## STOP triggers
- A reclaimed lease does NOT bump the token (e.g. forgot `nextval`) → BUG: the token bump is the whole point;
  without it a partitioned writer's late ops are NOT bounced (205 compares against the assignment's current
  token). The first test catches it.
- The sweep reclaims a LIVE lease (forgot `lease_expires_at < now()`) → BUG: that yanks an actively-renewing
  node's lease. The second test catches it.
- `LeaseClaim.lease_expires_at` non-Option clashes with the NULL-on-reclaim → resolve the shape coordination
  with 203 (make it `Option`); record. (conflicts_with 203.)
- `app.rs` / `services/mod.rs` anchors drifted (stale_cleanup moved) → re-locate the spawn + export sites;
  the lease sweep mounts wherever stale_cleanup does.
- `query!`/macro forms tempting → use runtime `sqlx::query` like `fail_node_assignments`; no offline cache.

## Done when
`WAI_TYPECHECK_CMD="cargo check -p remote" WAI_TEST_CMD='test -n "$DATABASE_URL" && cargo test -p remote sweep_tests' bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-hive-redesign 209` exits 0
(run with `DATABASE_URL=postgres://…` against a migrated Postgres — Trap 2b; `test -n` prefix FAIL-CLOSED.
The `test_default_config` in `lease_sweep.rs` is hermetic and runs regardless; the reclaim test gates on PG.)
