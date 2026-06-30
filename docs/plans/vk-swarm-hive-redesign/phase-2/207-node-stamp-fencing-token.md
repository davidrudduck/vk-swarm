---
id: "207"
phase: 2
title: Node — stamp OutboxOp.fencing_token from the lease at stream time for hive-assigned tasks
status: ready
depends_on: ["107", "206"]
parallel: false
conflicts_with: ["206", "208"]
files:
  - crates/services/src/services/hive_sync.rs
  - crates/services/src/services/node_runner.rs
irreversible: false
scope_test: "crates/services/src/services/hive_sync.rs"
allowed_change: edit
covers_criteria: [SC3]
covers_tests: []
---
## Failing test (write first)
**ARCHITECTURE NOTE (record in ledger — prompt divergence):** the prompt framed this as stamping at the
"outbox enqueue path." That is impossible-and-wrong-by-ADR: the enqueue lives in `crates/db`
(`task/queries.rs`, task 105), which has NO knowledge of hive leases, and ADR-0009 §3 says the node stamps
the token **it believes it holds at send time** — which lives in node-side lease state. The only place a
node op is materialized onto the wire is `sync_outbox` (107, `hive_sync.rs`, the db-row→WS `OutboxOp` map).
**That is the correct seam.** The `node_outbox.fencing_token` column (P1/101) stays NULL; the WS op's
`fencing_token` is stamped here from the live lease, NOT copied from the column.

Hermetic test (`db::test_utils::create_test_pool()` + an in-memory `NodeRunnerState`), in `hive_sync.rs`'s
`#[cfg(test)] mod tests`, beside 107's `sync_outbox_sends_unacked_ops_as_op_batch_in_seq_order`:
```rust
#[tokio::test]
async fn sync_outbox_stamps_fencing_token_for_hive_assigned_tasks_only() {
    let (pool, _tmp) = db::test_utils::create_test_pool().await;
    use db::models::node_outbox::{NewOutboxOp, OutboxRepository};

    let assigned_task = uuid::Uuid::new_v4();   // a hive-assigned local task
    let owned_task = uuid::Uuid::new_v4();       // node-owned, no assignment
    let mk = |tid: uuid::Uuid, k: &str| NewOutboxOp {
        op_type: "task.upsert".into(), entity_type: "task".into(), entity_id: tid,
        payload: serde_json::json!({}), idempotency_key: k.into(), fencing_token: None,
    };
    OutboxRepository::enqueue_op(&pool, mk(assigned_task, "task:a:1")).await.unwrap();
    OutboxRepository::enqueue_op(&pool, mk(owned_task, "task:b:1")).await.unwrap();

    // Lease state: assigned_task is in active_assignments with token 5; owned_task is not present.
    let state = std::sync::Arc::new(tokio::sync::RwLock::new(NodeRunnerState::default()));
    {
        let aid = uuid::Uuid::new_v4();
        state.write().await.active_assignments.insert(aid, ActiveAssignment {
            assignment_id: aid, task_id: uuid::Uuid::new_v4(), local_task_id: Some(assigned_task),
            local_attempt_id: None, status: TaskExecutionStatus::Pending,
            fencing_token: Some(5), lease_expires_at: Some(chrono::Utc::now() + chrono::Duration::seconds(60)),
        });
    }

    let (command_tx, mut command_rx) = tokio::sync::mpsc::channel(8);
    let service = HiveSyncService::new(pool.clone(), command_tx, HiveSyncConfig::default())
        .with_node_state(state.clone());   // the new Option<Arc<…>> setter (see Change)
    service.sync_outbox().await.unwrap();

    match command_rx.try_recv().expect("an OpBatch was sent") {
        NodeMessage::OpBatch { ops } => {
            let assigned = ops.iter().find(|o| o.entity_id == assigned_task).unwrap();
            let owned = ops.iter().find(|o| o.entity_id == owned_task).unwrap();
            assert_eq!(assigned.fencing_token, Some(5), "hive-assigned op carries the lease token");
            assert_eq!(owned.fencing_token, None, "node-owned op carries no token (CONTRACT §C)");
        }
        other => panic!("expected OpBatch, got {other:?}"),
    }
}

#[tokio::test]
async fn sync_outbox_without_node_state_passes_token_through_unchanged() {
    // Backwards-compat with 107: a service built WITHOUT with_node_state (None) stamps nothing — the WS
    // op's fencing_token equals the db-row column (NULL in the tracer). Keeps 107's test green.
    let (pool, _tmp) = db::test_utils::create_test_pool().await;
    use db::models::node_outbox::{NewOutboxOp, OutboxRepository};
    OutboxRepository::enqueue_op(&pool, NewOutboxOp {
        op_type: "task.upsert".into(), entity_type: "task".into(), entity_id: uuid::Uuid::new_v4(),
        payload: serde_json::json!({}), idempotency_key: "task:c:1".into(), fencing_token: None,
    }).await.unwrap();
    let (command_tx, mut command_rx) = tokio::sync::mpsc::channel(8);
    HiveSyncService::new(pool.clone(), command_tx, HiveSyncConfig::default())
        .sync_outbox().await.unwrap();
    match command_rx.try_recv().unwrap() {
        NodeMessage::OpBatch { ops } => assert_eq!(ops[0].fencing_token, None),
        other => panic!("expected OpBatch, got {other:?}"),
    }
}
```

## Change
### 1. `crates/services/src/services/hive_sync.rs`
- **Anchor:** `struct HiveSyncService` (@~105: `pool`, `command_tx`, `config`), its `new` (@~113), and the
  `sync_outbox` helper (added by 107, the db-row→WS `OutboxOp` map @~107-118).
- **Sibling read (rubric #9):** 107's `sync_outbox` is the exact code being edited — read its map closure
  (`fencing_token: r.fencing_token`) before changing it. The state type is
  `Arc<RwLock<NodeRunnerState>>` from `node_runner.rs` (the same one cloned at node_runner.rs:648/663).
- **Add an OPTIONAL node-state field** (defaulting `None` so existing callers + 107's test are untouched —
  this is what keeps 207 from rippling into P1):
  - Add field `node_state: Option<std::sync::Arc<tokio::sync::RwLock<crate::services::node_runner::NodeRunnerState>>>`
    to `HiveSyncService` (confirm the `NodeRunnerState` path; it is `pub` at node_runner.rs:249).
  - In `new`, initialize `node_state: None`.
  - Add a builder setter:
```rust
    /// Attach the node runner state so outbox ops against hive-assigned tasks can be stamped with the
    /// current fencing token (SC3). Without it, ops pass through unstamped (tracer/back-compat).
    pub fn with_node_state(
        mut self,
        state: std::sync::Arc<tokio::sync::RwLock<crate::services::node_runner::NodeRunnerState>>,
    ) -> Self {
        self.node_state = Some(state);
        self
    }
```
- **Stamp in `sync_outbox`'s map step.** Before building `ops`, if `self.node_state` is `Some`, take a read
  lock once and build a `local_task_id -> fencing_token` lookup from `active_assignments`
  (`.values().filter_map(|a| Some((a.local_task_id?, a.fencing_token?)))` into a `HashMap<Uuid, i64>`).
  Then in the per-row map, set `fencing_token: token_by_task.get(&r.entity_id).copied()` (Some for a
  hive-assigned task whose lease has a token; None otherwise — node-owned work, CONTRACT §C). When
  `node_state` is `None`, keep `fencing_token: r.fencing_token` (the 107 behavior).
  > Only `entity_type == "task"` ops map by task id; the tracer is task-only (107), so `r.entity_id` is the
  > local task id. If later op types add non-task entities, gate the stamp on `r.entity_type == "task"`.

### 2. `crates/services/src/services/node_runner.rs`
- **Anchor:** the `spawn_hive_sync_service` call site (@658) where `HiveSyncService` is constructed for the
  running node. Thread the shared `state`/`handle.state.clone()` into it via `.with_node_state(state.clone())`
  so the live node stamps tokens. (This is the ONLY node_runner edit — a one-line builder call at the spawn
  site. `spawn_hive_sync_service` may need a small passthrough of the state arg; if it constructs the
  `HiveSyncService` internally, add the `state` param to it and call `.with_node_state`. Keep the change to
  the construction/threading only.)
  > If `spawn_hive_sync_service` lives in `hive_sync.rs` (it is `use super::hive_sync::spawn_hive_sync_service`
  > at node_runner.rs:626), the `with_node_state` call may belong there instead — place it wherever the
  > `HiveSyncService` is actually built for the node, and keep `node_runner.rs`'s edit to passing the state in.

## Allowed moves
ONLY: add the optional `node_state` field + `with_node_state` setter to `HiveSyncService`, stamp the WS op's
`fencing_token` from the lease lookup in `sync_outbox`'s map, and thread the node state into the
`HiveSyncService` construction for the running node. Do NOT change `sync_outbox`'s send/seq-order behavior
(107 owns it), do NOT touch the db enqueue (105), the `node_outbox` schema/column (101), the WS enums (202),
`ActiveAssignment`'s field definitions (206 owns them — 207 only READS them), the hive side, or any migration.

## STOP triggers
- Stamping a token for a task NOT in `active_assignments` → BUG: node-owned work must carry `None`
  (CONTRACT §C / ADR-0009 §3). Only `local_task_id`s present with a `Some(fencing_token)` get stamped.
- A REQUIRED (non-Option) `node_state` param on `new` → BUG: breaks 107's test + every existing caller.
  Use the `Option` field + `with_node_state` builder; `None` = passthrough.
- Copying `r.fencing_token` (the always-NULL db column) when `node_state` is `Some` → the stamp never
  happens. When state is attached, the token comes from the lease lookup, not the column.
- `NodeRunnerState`/`ActiveAssignment.fencing_token` absent → 206 must be `passed` first (depends_on: 206).
  `sync_outbox` absent → 107 must be `passed` (depends_on: 107).
- Reaching into `ActiveAssignment` to MUTATE the token here → STOP: 207 only reads the lease; 206 owns the
  field + its writes.

## Done when
`WAI_TYPECHECK_CMD="cargo check -p services" WAI_TEST_CMD="cargo test -p services sync_outbox" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-hive-redesign 207` exits 0
(node side — hermetic `create_test_pool()`; no Postgres precondition. `cargo check -p services` is the
Trap-1 typecheck override. The filter `sync_outbox` runs both 107's and 207's tests in that module.)
