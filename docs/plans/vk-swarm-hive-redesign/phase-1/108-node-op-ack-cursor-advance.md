---
id: "108"
phase: 1
title: Node OpAck — advance node_outbox ack cursor on durable hive ack
status: ready
depends_on: ["103", "104"]
parallel: false
conflicts_with: ["103"]
files:
  - crates/services/src/services/hive_client.rs
  - crates/services/src/services/node_runner.rs
irreversible: false
scope_test: "crates/services/src/services/node_runner.rs"
allowed_change: edit
covers_criteria: [SC2]
covers_tests: [TS1]
---
## Failing test (write first)
The cursor advance is the close of the silent-loss window: ops are cleared from unacked ONLY when the
hive durably acks. Test it at the seam where the pool lives — the `OpAck` event handler advances the
node_outbox cursor. Hermetic (`create_test_pool()`); extract a small `apply_op_ack(pool, seq)` helper
so the test can call it directly without standing up a full WS loop.

```rust
#[tokio::test]
async fn op_ack_advances_outbox_cursor() {
    let (pool, _tmp) = db::test_utils::create_test_pool().await;
    use db::models::node_outbox::{NewOutboxOp, OutboxRepository};
    let mk = |k: &str| NewOutboxOp {
        op_type: "task.upsert".into(), entity_type: "task".into(),
        entity_id: uuid::Uuid::new_v4(), payload: serde_json::json!({}),
        idempotency_key: k.into(), fencing_token: None,
    };
    let a = OutboxRepository::enqueue_op(&pool, mk("task:a:1")).await.unwrap();
    let _b = OutboxRepository::enqueue_op(&pool, mk("task:b:1")).await.unwrap();

    // Durable ack through the first op's seq → only b remains unacked.
    crate::services::node_runner::apply_op_ack(&pool, a.seq).await;

    let remaining = OutboxRepository::peek_unacked(&pool, 10).await.unwrap();
    assert_eq!(remaining.len(), 1, "ops at/under acked seq are cleared from unacked");
    assert!(remaining[0].seq > a.seq);
}
```
> If the module path for `apply_op_ack` differs (e.g. it is `pub(crate)` in `node_runner`), adjust the
> call path; keep the helper test-reachable.

## Change

### 1. `crates/services/src/services/hive_client.rs` — emit a HiveEvent on OpAck
- **Anchor:** `enum HiveEvent` (the variant list, ends with `Error { message: String }`), and the
  `HiveMessage::OpAck { applied_through_seq }` STUB arm that 103 added in `handle_hive_message`
  (before the `_ =>` wildcard @1062).
- **Before (HiveEvent tail — the `Error` variant + closing brace):**
```rust
    /// Error from hive
    Error { message: String },
}
```
- **After:**
```rust
    /// Error from hive
    Error { message: String },
    /// Durable op-log ack: all node_outbox ops with seq <= applied_through_seq are persisted (SC2c).
    OpAck { applied_through_seq: i64 },
}
```
- **Before (the 103 stub arm in `handle_hive_message`):**
```rust
            HiveMessage::OpAck { applied_through_seq } => {
                // STUB — filled by task 108 (advance the node_outbox ack cursor). For now log only so
                // the arm is EXPLICIT (not swallowed by the `_ =>` wildcard below) and compiles.
                tracing::debug!(applied_through_seq, "received op_ack (cursor advance TODO: task 108)");
            }
```
- **After:**
```rust
            HiveMessage::OpAck { applied_through_seq } => {
                tracing::trace!(applied_through_seq, "received op_ack");
                let _ = self
                    .event_tx
                    .send(HiveEvent::OpAck { applied_through_seq })
                    .await;
            }
```
  > `handle_hive_message` is `&self` and `HiveClient` holds NO pool — so the cursor advance CANNOT run
  > here. It emits a `HiveEvent::OpAck` (mirroring every other arm's `self.event_tx.send(HiveEvent::…)`),
  > and the pool-bearing consumer (`run_node_runner`, which has `db.pool`) does the DB write. This is the
  > established node-side seam (see `TaskSyncResponse`/`BackfillRequest`: "DB update happens in
  > run_node_runner where we have access to the pool", node_runner.rs:461,481).

### 2. `crates/services/src/services/node_runner.rs` — handle the event with the pool
- **Anchor:** the EXHAUSTIVE `process_event` match (@341-484; no `_`, so the new variant forces an arm)
  and the `run_node_runner` event loop (@666-921, which has `db.pool` and a `Some(_) => {}` catch-all
  @921).
- **Add to `process_event`** (state-only side; just acknowledge — the DB write is in the loop). Before
  the closing `}` of the match (after the `BackfillRequest` arm @474-483):
```rust
            HiveEvent::OpAck { applied_through_seq } => {
                tracing::trace!(applied_through_seq = *applied_through_seq, "op_ack received");
                // DB cursor advance happens in run_node_runner where the pool is available.
            }
```
- **Add a pool-bearing arm in the `run_node_runner` loop** — BEFORE the `Some(_) => {}` catch-all @921:
```rust
                Some(HiveEvent::OpAck { applied_through_seq }) => {
                    apply_op_ack(&db.pool, applied_through_seq).await;
                }
```
- **Add the `apply_op_ack` helper** (free fn in `node_runner`, `pub(crate)` so the test can reach it),
  placed near the other module helpers (e.g. beside `handle_backfill_attempt`):
```rust
/// Advance the node_outbox ack cursor on a durable hive OpAck (SC2c). Clears all unacked ops with
/// seq <= applied_through_seq. Best-effort: a failure is logged (the op stays unacked and is re-sent
/// on the next OpBatch — at-least-once, which is safe because the hive apply is idempotent).
pub(crate) async fn apply_op_ack(pool: &sqlx::SqlitePool, applied_through_seq: i64) {
    use db::models::node_outbox::OutboxRepository;
    if let Err(e) = OutboxRepository::mark_acked_through(pool, applied_through_seq).await {
        tracing::warn!(error = %e, applied_through_seq, "failed to advance node_outbox ack cursor");
    }
}
```

## Allowed moves
ONLY: add the `HiveEvent::OpAck` variant, replace 103's `OpAck` stub body with the event emission, add
the `process_event` arm, add the `run_node_runner` arm, and add the `apply_op_ack` helper. Do NOT mark
ops acked anywhere else, do NOT advance the cursor on send (107 explicitly does NOT ack — the cursor
advances ONLY here, on durable ack), do NOT change the WS enum (103 owns it).

## STOP triggers
- `process_event`'s match has gained a `_` wildcard → still add the explicit `OpAck` arm (keep it named
  for clarity; the loop arm is what does the work).
- The `run_node_runner` loop's `Some(_) => {}` catch-all is gone/changed → place the `OpAck` arm before
  whatever catch-all exists; it MUST run with `db.pool` in scope.
- `db.pool` is not a `SqlitePool` in `run_node_runner` → it is (`spawn_hive_sync_service(db.pool.clone()…)`
  @658 passes a `SqlitePool`); use `&db.pool`.
- 103's `OpAck` stub arm is not present (103 not `passed`) → STOP; depends_on: 103.
- `OutboxRepository::mark_acked_through` absent → 104 must be `passed`; depends_on: 104.
- Advancing the cursor on SEND instead of on ACK → BUG: that reopens the silent-loss window this task
  closes. The advance lives ONLY in `apply_op_ack`, reached ONLY from a `HiveMessage::OpAck`.

## Done when
`WAI_TYPECHECK_CMD="cargo check -p services" WAI_TEST_CMD="cargo test -p services op_ack" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-hive-redesign 108` exits 0
(export `DATABASE_URL=sqlite://<repo>/dev_assets/db.sqlite` migrated through 101 before running — Trap 2.)
