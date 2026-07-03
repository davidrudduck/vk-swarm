---
id: "210"
phase: 2
title: SC3 acceptance test — partition cannot double-execute (stale-token reject + self-fence)
status: ready
depends_on: ["203", "205", "208", "209"]
parallel: false
conflicts_with: []
files:
  - crates/remote/tests/lease_partition_e2e.rs
irreversible: false
scope_test: "crates/remote/tests/lease_partition_e2e.rs"
allowed_change: create
covers_criteria: [SC3]
covers_tests: [TS2]
---
## Failing test (write first)
**This task CLAIMS TS2** — the only task that does; `wai-plan-lint` hard-fails if TS2 is unclaimed. TS2
(spec `## Test strategy`): *"Lease a task to node A; simulate A partitioned but alive; expire + reassign to
B (higher fencing token); assert A's late commit is **rejected by stale-token** and A **self-fences** (agent
halted) within TTL — at-most-once effect, bounded overlap."*

**PRECONDITION (Trap 2b — NON-NEGOTIABLE):** the stale-token-reject half exercises the hive op-apply +
fencing against a live, migrated Postgres. A run without `DATABASE_URL` returns early (skip) = HOLLOW pass.
Stand up Postgres, `sqlx::migrate!("./migrations")`, export `DATABASE_URL=postgres://…`, or RAISE.

**Scope decision (record in ledger):** a full cross-process WS round-trip (real node binary ↔ real hive)
is not hermetically testable in a single `cargo test`. TS2 is proven as TWO coordinated assertions over the
real units, NOT a mocked end-to-end:
1. **Hive stale-token rejection (the at-most-once commit EFFECT)** — proven by **205's in-module
   `#[cfg(test)] mod` test in `session.rs`**, which CAN call the private `handle_op_batch` and asserts that
   a stale-token op is rejected (no apply, no advance, `LeaseRevoked` surfaced). This acceptance task
   ASSERTS that coverage exists BY REFERENCE (the ledger records 205's reject-test name), exactly as it does
   for the self-fence leg below — it does NOT re-assert the reject at the fencing-free repository layer,
   which would BYPASS the mechanism and be HOLLOW (tournament R2/F8: `upsert_from_node` has no fencing).
2. **Reclaim → reassign → strictly-higher token (the partition-safety BASIS)** — an integration test here
   in `crates/remote/tests/` drives the real `try_claim` (203) → `reclaim_expired_leases` (209) → second
   `try_claim` and asserts **B's fencing_token is strictly higher than A's**. This is the public-repo chain
   that makes 205's stale-token compare meaningful; it is genuinely exercisable from `tests/`.
3. **Node self-fence (bounded overlap)** — covered by 208's hermetic `self_fence_tests`
   (`assignments_to_self_fence` selects an EXPIRED Running assignment for halt) + 206's lease-state test.
   ASSERTED by reference (this crate cannot import the node `services` test module); the ledger records the
   two test names.

> **Seam (R2/F8):** the fencing REJECT needs the private `session.rs::handle_op_batch` and therefore lives
> in 205's own in-module `#[cfg(test)]` test — NOT here. This `tests/` integration file proves only the
> public-repo reclaim/token-bump chain (leg 2). Do NOT make `handle_op_batch` `pub` and do NOT assert the
> reject at the repo layer (hollow). SC3/TS2 is the conjunction of the three legs, recorded in the ledger.

Create `crates/remote/tests/lease_partition_e2e.rs`. **Sibling read (rubric #9):** `backfill_e2e.rs` for
`database_url()`/`skip_without_db!`/`create_pool()`/`create_test_organization`/`create_test_node`; inline
verbatim (no `common` module).
```rust
//! TS2 (SC3): a partition cannot cause double execution — stale-token commit is rejected.
use sqlx::PgPool;
use remote::db::task_assignments::TaskAssignmentRepository;

fn database_url() -> Option<String> { std::env::var("DATABASE_URL").ok() }
macro_rules! skip_without_db { () => {
    if database_url().is_none() { eprintln!("Skipping: DATABASE_URL not set"); return; }
}; }
async fn create_pool() -> PgPool { PgPool::connect(&database_url().unwrap()).await.expect("connect") }
// … inline create_test_organization / create_test_node + seed (task_id, node_project_id).

#[tokio::test]
async fn partitioned_node_late_commit_is_rejected_after_reassignment() {
    skip_without_db!();
    let pool = create_pool().await;
    let repo = TaskAssignmentRepository::new(&pool);
    // 1. Lease task T to node A (token T1). Use an already-past TTL to simulate the partition window.
    let a = repo.try_claim(task_id, node_a, np_id, chrono::Duration::seconds(-1))
        .await.unwrap().expect("A wins");
    // 2. A is partitioned-but-alive. The lease-expiry SWEEP (209) reclaims the expired lease, bumping the
    //    token (exercises reclaim_expired_leases — why 210 depends_on 209). Then reassign to node B.
    let reclaimed = repo.reclaim_expired_leases().await.unwrap();
    assert!(reclaimed.iter().any(|r| r.task_id == task_id), "the sweep reclaimed A's expired lease");
    let b = repo.try_claim(task_id, node_b, np_id, chrono::Duration::seconds(300))
        .await.unwrap().expect("B claims the reclaimed task");
    assert!(b.fencing_token > a.fencing_token, "B's token is strictly higher (partition-safety BASIS)");
    // That strictly-higher token is exactly what 205's fencing compare uses to REJECT node A's late op
    // (stamped with the OLD token T1 < T2). The reject itself is proven by 205's in-module #[cfg(test)]
    // test against the private `handle_op_batch` (R2/F8 — do NOT re-assert it here at the repo layer).
    // SC3/TS2 = this token-bump chain + 205's reject test + 208's self-fence test (recorded in the ledger).
    let _ = (a, b);
}
```

## Allowed moves
ONLY create the one `tests/lease_partition_e2e.rs` integration test (the reclaim/token-bump chain, leg 2).
Do NOT add production code, do NOT make `handle_op_batch`/the fencing check `pub`, and do NOT re-assert the
fencing reject at the repo layer (hollow — R2/F8; the reject is 205's in-module test). Do NOT touch any
other file.

## STOP triggers
- The test asserts the reject by making a private hive fn `pub` → STOP: that changes a contract for a test.
  Reuse 205's existing test seam, or assert the observable end-state via the public `TaskAssignmentRepository`
  + `SharedTaskRepository` surface. Record the seam.
- `remote::db::task_assignments::TaskAssignmentRepository` is not importable from `tests/` (not `pub`) →
  check the module's visibility; `backfill_e2e.rs` imports `remote::…` repos, so the path exists — match its
  import style. If `try_claim`/`reclaim` are not `pub`, STOP: 203/209 must expose them (they are `pub async
  fn` per those tasks) — re-verify.
- A run reports the test SKIPPED (no `DATABASE_URL`) and the gate goes green → HOLLOW (Trap 2b). The
  `test -n` prefix in Done-when makes it FAIL-CLOSED; do not remove it.
- Trying to spin a real node process for a true cross-process round-trip → out of scope; TS2 is proven by
  the hive reject (here) + the node self-fence unit tests (208/206). Record this framing.

## Done when
`WAI_TYPECHECK_CMD="cargo check -p remote" WAI_TEST_CMD='test -n "$DATABASE_URL" && cargo test -p remote --test lease_partition_e2e' bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-hive-redesign 210` exits 0
(run with `DATABASE_URL=postgres://…` against a migrated Postgres — Trap 2b; `test -n` prefix FAIL-CLOSED.
The node self-fence half (TS2's "bounded overlap") is verified by 208's `self_fence_tests` + 206's
`lease_state_tests` on the node `services` crate — recorded in the ledger as the second TS2 leg.)
