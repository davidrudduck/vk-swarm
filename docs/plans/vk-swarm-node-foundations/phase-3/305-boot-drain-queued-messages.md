---
id: "305"
phase: 3
title: Boot-drain the persisted message queue for non-resumed attempts
status: ready
depends_on: ["102", "304"]
parallel: false
conflicts_with: []
files:
  - crates/local-deployment/src/container.rs
  - crates/server/src/main.rs
irreversible: false
scope_test: "crates/local-deployment/src/container.rs"
allowed_change: mixed
covers_criteria: [SC2]
---
## Failing test (write first)
The drain-on-resume half of SC2 (breakdown-review R1): task 102 makes the queue PERSIST, but nothing
drains it at boot — `try_consume_queued_message` fires only on a live process exit (`container.rs:738`).
A crashed-and-restarted attempt that has queued messages but is NOT being resumed would otherwise sit
forever. Test that a boot-drain starts the next queued message for such an attempt:
```rust
#[tokio::test]
async fn test_boot_drain_starts_queued_message_for_idle_attempt() {
    let (pool, _tmp) = db::test_utils::create_test_pool().await;
    let svc = test_local_container_service(pool.clone()).await; // existing/standard harness
    // Seed an attempt with a persisted queued message and NO running execution_process.
    let attempt_id = seed_attempt_with_queued_message(&pool, "do the next thing").await;

    svc.drain_queued_messages_on_boot().await.unwrap();

    // The queued message was consumed (a new execution start was triggered) and removed from the queue.
    assert!(svc.message_queue().peek_next(attempt_id).await.is_none());
}
```
(Match the real test harness for building a `LocalContainerService`; if starting a real execution is too
heavy, assert the drain SELECTED the attempt + called the start path — stub/observe the start. Record
the chosen seam in the ledger.)

## Change
- **File:** `crates/local-deployment/src/container.rs` — add an async method
  `drain_queued_messages_on_boot(&self) -> Result<(), ContainerError>` that:
  1. SELECTs distinct `task_attempt_id`s having rows in `queued_messages` (the table from 101).
  2. For EACH, skips it if that attempt currently has a running/just-resumed execution process
     (`has_running_processes_for_attempt`, the trait method at `services/container.rs:144`) — those are
     handled by task 304's resume + the live `:738` drain; starting a queued message under a live writer
     would create a second writer (the ADR-0001 hazard).
  3. For attempts with NO active execution, start the next queued message via the SAME path the live
     drain uses (`try_consume_queued_message` / the start-execution machinery) so the queued follow-up
     becomes a real execution.
- **File:** `crates/server/src/main.rs` — call `drain_queued_messages_on_boot()` at startup **AFTER**
  `cleanup_orphan_executions()` (the recovery call at `main.rs:133`), so step 2's "is it being resumed?"
  check sees 304's results. One added call, guarded/logged like the existing recovery call.

## Allowed moves
Add the boot-drain method + its one startup call site. Reuse existing primitives
(`has_running_processes_for_attempt`, `try_consume_queued_message`, the message-queue accessors). Do NOT
modify recovery (304) or the live-exit drain at `:738`. Do NOT change `MessageQueueStore`'s API (102
owns it).

## STOP triggers
- `try_consume_queued_message` cannot be invoked at boot without a live `ExecutionContext` it constructs
  from an exit event → build the minimal context the start path needs from the attempt's latest
  execution_process, OR call the lower-level start path directly; record the chosen approach. Do NOT
  fabricate a fake exit event.
- The "is this attempt being resumed by 304?" signal is ambiguous (304 may mark `resume_state='resumed'`
  but not yet have a running process at the instant the drain runs) → key the skip on BOTH
  `has_running_processes_for_attempt` AND `resume_state IN ('pending','resumed')` to avoid the
  double-writer race; record the predicate.
- Ordering: if `main.rs` calls the drain BEFORE `cleanup_orphan_executions`, the skip check is wrong —
  it MUST run after recovery.

## Done when
Adds no new schema (reads the 101 table) but its `query!`/`query_as!` reference `queued_messages` (101)
— ensure the schema is materialized (Trap 2): apply migrations and/or `cargo sqlx prepare --workspace`.

`WAI_TYPECHECK_CMD="cargo sqlx prepare --workspace --check || cargo sqlx prepare --workspace; cargo check -p local-deployment && cargo check -p server" WAI_TEST_CMD="cargo test -p local-deployment boot_drain" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-node-foundations 305` exits 0
