# Adversarial Review - vk-swarm-hive-redesign Phase 4-7

This document will contain the findings of the adversarial review.

## Verdict: DEVIATES

## SC conformance
- SC1: MET. The data-plane half ("no fan-out") is addressed by Phase 6.
  - **Criterion:** "no node↔node or node↔hive↔node fan-out"
  - **Evidence:**
    - The new test `crates/remote/tests/no_fanout_invariant.rs` implements a compile-time guard. Its `classify` function exhaustively matches all `HiveMessage` variants and the test `no_hive_message_variant_is_task_state_fanout` asserts that none are classified as `Delivery::TaskStatePush`.
    - The new comment fence in `crates/remote/src/nodes/ws/connection.rs` documents the invariant at the send-sites.
  - **Justification:** The implementation correctly adds a regression guard to enforce the "no fan-out" rule, as described in the plan and decision ledger. This was a "verify and guard" task, which has been completed.
- SC4: OUT OF SCOPE. Covered by Phase 3, which was shipped in a previous PR (#450) and is not part of this review.
- SC5: MET. Phase 5 implements the anti-entropy reconciliation sweep.
  - **Criterion:** "Node↔hive divergence self-heals via an anti-entropy reconciliation sweep — no manual `reset_*` migration is ever required."
  - **Evidence:**
    - The digest exchange mechanism is implemented across tasks 501-504.
    - `crates/remote/src/nodes/ws/session.rs` (task 503) implements `handle_digest_compare` which computes the set difference between the node's digest and the hive's state.
    - `crates/services/src/services/node_runner.rs` (task 504) handles the `DigestResult` from the hive, triggering a re-stream from the outbox (using the new `peek_from_seq` which can read acked ops) or a bulk pull.
    - The decision ledger records two critical bug fixes (P5-review F1, F2) that prevent infinite loops for soft-deleted tasks and large numbers of ops, making the healing process more robust.
  - **Justification:** The implementation correctly establishes a self-healing mechanism for both node-has/hive-lacks and hive-has/node-lacks divergences, satisfying the requirement to eliminate manual repair migrations.
- SC6: MET. Phase 7 implements the hive-only-state cutover.
  - **Criterion:** "`MUST-MIGRATE` tables (...) are fully populated from the node and hive, `REGENERABLE` tables are cleared, and `REGENERABLE` data is fully re-ingested from the node-side truth tables via the normal sync path."
  - **Evidence:**
    - Task 701 adds a migration (`..._hive_cutover_clear_regenerable_discardable.sql`) that `TRUNCATE`s regenerable/discardable tables and `DELETE`s completed assignments. Its accompanying test (`hive_cutover_migration.rs`) verifies that this clears the right data while preserving must-migrate data.
    - Task 702's test (`hive_cutover_must_migrate.rs`) verifies that the crucial ID-bridge and status information in `MUST-MIGRATE` tables is correct after the cutover.
    - Task 703's test (`hive_cutover_reingest.rs`) proves that the cleared `REGENERABLE` tables (e.g., `node_task_attempts`) can be repopulated by the existing node data ingest path.
  - **Justification:** The implementation correctly performs the one-time data cutover and includes comprehensive tests to ensure that must-migrate data is preserved and that the system can be repopulated from nodes afterward, directly satisfying all clauses of SC6.
- SC7: UNMET.
  - **Criterion:** "A task deleted on the hive is soft-deleted on the node; a task deleted on the node is soft-deleted on the hive. Each leg is idempotent and race-free."
  - **Assessment:** The hive->node deletion path is correctly implemented and guarded (MET). However, the node->hive deletion path is not implemented. `task.delete` operations from the node are explicitly skipped in the hive's `handle_op_batch_apply` function. See finding F1.

## Findings (cited)
### F1 — Node-to-Hive `task.delete` is not handled.
**Severity:** Blocker
**Cited evidence:**
```rust
// file: crates/remote/src/nodes/ws/session.rs

// inside handle_op_batch_apply
        // (a) Tracer scope guard: only task.upsert is handled in this phase.
        if op.op_type != "task.upsert" {
            applied_through_seq = op.seq;
            continue;
        }
```
**Why it deviates:**
SC7 requires that a task deleted on the node is soft-deleted on the hive. The `handle_op_batch_apply` function, which processes operations from the node, explicitly skips any operation that is not `task.upsert` due to a "Tracer scope guard". This means `task.delete` operations from the node are ignored by the hive, the task is never soft-deleted on the hive side, and the node and hive states diverge permanently.

**Exact fix:**
The tracer scope guard must be removed and replaced with proper handling for the `task.delete` op type within `handle_op_batch_apply`.

```rust
// In `handle_op_batch_apply` in `crates/remote/src/nodes/ws/session.rs`:

// Replace the guard with a match statement to handle different op types.
match op.op_type.as_str() {
    "task.upsert" => {
        // ... existing upsert logic from the function ...
    },
    "task.delete" => {
        // Logic to handle delete operation from the node
        let local_task_id: Uuid = op.entity_id;

        // Idempotency: only apply if not seen before
        let seen: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM node_op_log WHERE node_id = $1 AND idempotency_key = $2)",
        )
        .bind(node_id)
        .bind(&op.idempotency_key)
        .fetch_one(pool)
        .await
        .map_err(|e| HandleError::Database(e.to_string()))?;

        if !seen {
            let repo = crate::db::tasks::SharedTaskRepository::new(pool);
            if let Some(task_to_delete) = repo.find_by_source_task_id(node_id, local_task_id).await.map_err(|e| HandleError::Database(e.to_string()))? {
                 // Use the existing `delete` method which performs a soft-delete.
                 // The user_id for deletion is not available in the op, so it's passed as None.
                 let delete_data = crate::db::tasks::DeleteTaskData {
                     version: task_to_delete.version, // Or use a version from op payload if available
                     acting_user_id: None,
                 };
                 if let Err(e) = repo.delete(task_to_delete.id, &delete_data).await {
                     tracing::error!("Failed to soft-delete task from node op: {}", e);
                 }
            }
        }
        
        // Record in node_op_log and advance seq, even if task was already gone.
        sqlx::query(
            r#"
            INSERT INTO node_op_log (node_id, idempotency_key, seq, op_type, entity_id)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (node_id, idempotency_key) DO NOTHING
            "#,
        )
        .bind(node_id)
        .bind(&op.idempotency_key)
        .bind(op.seq)
        .bind(&op.op_type)
        .bind(op.entity_id)
        .execute(pool)
        .await
        .map_err(|e| HandleError::Database(e.to_string()))?;
        
        applied_through_seq = op.seq;
    },
    _ => {
        // Default behavior for unhandled ops: skip and advance
        applied_through_seq = op.seq;
    }
}
// Ensure the loop continues after the match.
continue;
```

## Hollow-test audit
The new tests added in this diff appear to be robust and not hollow. They consistently drive production or near-production entry points, seed realistic data, and assert specific business outcomes as defined in the success criteria.

- **`no_fanout_invariant.rs` (SC1/P6):** Not hollow. A structural test that provides a compile-time guarantee. Adding a new `TaskStatePush` variant would cause a compile failure, which is the intended guard.
- **`ts5_delete_equivalence.rs` (SC7/P4):** Not hollow. Drives both the reconcile and WebSocket inbound legs and asserts the specific "soft-unlink" outcome in the database. If the `unlink_by_shared_task_id` implementation was faulty, the test would fail.
- **Phase 5 Anti-Entropy tests (SC5/P5):** Not hollow. The main acceptance test `ts4_self_heal_via_anti_entropy` (in `crates/remote/tests/anti_entropy.rs`) seeds divergent state between a mock node and the hive, runs the digest exchange, and asserts that the correct heal directives (`pull_entities`, `resend_from_seq`) are generated.
- **Phase 7 Cutover tests (SC6/P7):** Not hollow. The tests for the cutover (`hive_cutover_migration.rs`, `hive_cutover_must_migrate.rs`, `hive_cutover_reingest.rs`) are thorough. They run the actual migration SQL against seeded data and verify that `REGENERABLE` data is cleared, `MUST-MIGRATE` data is preserved, and the system can re-ingest data into the cleared tables post-migration.

The testing strategy appears sound and provides good coverage for the implemented changes.

## Reachability traces
- **SC1 (No Fan-out):** MET. The guard is a compile-time structural test, not on a production call path, but it correctly protects the build. The associated comment fence is on the production send path in `connection.rs`.
- **SC5 (Anti-Entropy):** MET. The digest exchange is correctly wired into the main production loops on both the node (`NodeRunner::run` -> `sync_once` -> `sync_digest`) and the hive (`Connection::run` -> `handle_message` -> `handle_digest`). The node's response to the `DigestResult` is also on the main event processing path.
- **SC6 (Cutover):** MET. The code is a one-time database migration, which is part of the deployment path. The guard tests correctly verify its outcome.
- **SC7 (Inbound Collapse):** MET. All new logic (soft-unlinking, dirty-guard, reassigned handling) is correctly wired into the two inbound production paths: the periodic reconcile (`sync_remote_projects`) and the real-time WebSocket event processor (`process_event`).

