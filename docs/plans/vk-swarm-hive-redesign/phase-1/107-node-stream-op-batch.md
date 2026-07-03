---
id: "107"
phase: 1
title: Node streamer — drain node_outbox into NodeMessage::OpBatch in sync_once
status: done
depends_on: ["103", "104"]
parallel: false
conflicts_with: []
files:
  - crates/services/src/services/hive_sync.rs
irreversible: false
scope_test: "crates/services/src/services/hive_sync.rs"
allowed_change: edit
covers_criteria: [SC2]
covers_tests: [TS1]
---
## Failing test (write first)
In `crates/services/src/services/hive_sync.rs` `#[cfg(test)] mod tests`, add a test that drives
`sync_once` (or the extracted `sync_outbox` helper) over a `create_test_pool()` with enqueued
`node_outbox` ops and asserts a `NodeMessage::OpBatch` is pushed to the command channel carrying those
ops in seq order — and that NO ack/clear happens here (108 owns the cursor advance).

```rust
#[tokio::test]
async fn sync_outbox_sends_unacked_ops_as_op_batch_in_seq_order() {
    let (pool, _tmp) = db::test_utils::create_test_pool().await;
    use db::models::node_outbox::{NewOutboxOp, OutboxRepository};
    let mk = |k: &str| NewOutboxOp {
        op_type: "task.upsert".into(), entity_type: "task".into(),
        entity_id: uuid::Uuid::new_v4(), payload: serde_json::json!({}),
        idempotency_key: k.into(), fencing_token: None,
    };
    OutboxRepository::enqueue_op(&pool, mk("task:a:1")).await.unwrap();
    OutboxRepository::enqueue_op(&pool, mk("task:b:1")).await.unwrap();

    let (command_tx, mut command_rx) = tokio::sync::mpsc::channel(8);
    let service = HiveSyncService::new(pool.clone(), command_tx, HiveSyncConfig::default());
    service.sync_outbox().await.unwrap(); // the extracted helper (see Change)

    let msg = command_rx.try_recv().expect("an OpBatch was sent");
    match msg {
        NodeMessage::OpBatch { ops } => {
            assert_eq!(ops.len(), 2);
            assert!(ops[1].seq > ops[0].seq, "seq order preserved");
            assert!(ops.iter().all(|o| o.op_type == "task.upsert"));
        }
        other => panic!("expected OpBatch, got {other:?}"),
    }

    // Cursor NOT advanced here (108 owns it): the ops are still unacked.
    assert_eq!(OutboxRepository::peek_unacked(&pool, 10).await.unwrap().len(), 2);
}
```

## Change
- **File:** `crates/services/src/services/hive_sync.rs`
- **Anchor:** `sync_once` (@151-188) and the `use super::hive_client::{…}` import block (@38-41). The
  `HiveSyncService` already holds `pool` (@106) and `command_tx: mpsc::Sender<NodeMessage>` (@107) —
  exactly what the streamer needs (no new wiring).
- **Sibling read (rubric #9):** the existing `sync_*` helpers (e.g. `sync_tasks` @216) are the pattern —
  read from `self.pool`, build a `NodeMessage`, push via `self.command_tx.send(...)`. Mirror that;
  `OutboxOp` is the WS `crate::services::hive_client::OutboxOp` from 103 (NOT the db-row `OutboxOp` from
  104 — map db-row → WS struct).
- **Before (import block, @38-41):**
```rust
use super::hive_client::{
    AttemptSyncMessage, ExecutionSyncMessage, LocalProjectSyncInfo, LogsBatchMessage, NodeMessage,
    ProjectsSyncMessage, SyncLogEntry, TaskOutputType, TaskSyncMessage,
};
```
- **After:** add `OutboxOp` to the import list:
```rust
use super::hive_client::{
    AttemptSyncMessage, ExecutionSyncMessage, LocalProjectSyncInfo, LogsBatchMessage, NodeMessage,
    OutboxOp, ProjectsSyncMessage, SyncLogEntry, TaskOutputType, TaskSyncMessage,
};
```
- **Before (`sync_once` tail, @184-188):**
```rust
        // Labels are NOT synced from nodes to hive - they flow hive->nodes only

        Ok(())
    }
```
- **After:**
```rust
        // Labels are NOT synced from nodes to hive - they flow hive->nodes only

        // Drain the node_outbox op-log (SC2 tracer): send unacked ops in seq order as a single
        // OpBatch. Does NOT mark them acked — the cursor advances only on the hive's durable OpAck
        // (task 108). Runs ALONGSIDE the legacy sync above (additive; hive apply is idempotent).
        if let Err(e) = self.sync_outbox().await {
            warn!(error = ?e, "Failed to drain node_outbox op-log");
        }

        Ok(())
    }

    /// Drain unacked node_outbox ops and push them to the hive as one ordered `OpBatch`.
    /// Best-effort: an empty outbox sends nothing. Does NOT advance the ack cursor (108 owns that).
    async fn sync_outbox(&self) -> Result<(), HiveSyncError> {
        use db::models::node_outbox::OutboxRepository;
        let rows = OutboxRepository::peek_unacked(&self.pool, self.config.max_tasks_per_batch).await?;
        if rows.is_empty() {
            return Ok(());
        }
        let ops: Vec<OutboxOp> = rows
            .into_iter()
            .map(|r| OutboxOp {
                seq: r.seq,
                op_type: r.op_type,
                entity_type: r.entity_type,
                entity_id: r.entity_id,
                payload: r.payload,
                idempotency_key: r.idempotency_key,
                fencing_token: r.fencing_token,
            })
            .collect();
        self.command_tx
            .send(NodeMessage::OpBatch { ops })
            .await
            .map_err(|e| HiveSyncError::Send(e.to_string()))?;
        Ok(())
    }
```

## Allowed moves
ONLY: add `OutboxOp` to the import, call `self.sync_outbox()` at the end of `sync_once`, and add the
private `sync_outbox` helper that peeks unacked rows and sends ONE `OpBatch`. Do NOT mark ops acked here
(108 owns the cursor advance on `OpAck`), do NOT remove/alter the existing legacy `sync_*` calls, do NOT
change `sync_once`'s signature.

## STOP triggers
- `db::models::node_outbox::OutboxRepository`/the db-row `OutboxOp` fields differ from 104 → 104 must be
  `passed`; align field names. (depends_on: 104.)
- `NodeMessage::OpBatch`/the WS `OutboxOp` are absent → 103 must be `passed` (depends_on: 103); without
  it `cargo check -p services` will not compile.
- `self.config` has no batch-size field reusable for the outbox → use `max_tasks_per_batch` (already
  present, @49); do NOT add a new config field in this task.
- The test cannot construct `HiveSyncService` because `new` is private/signature differs → it is `pub fn
  new(pool, command_tx, config)` (@113); use it as-is.

## Done when
`WAI_TYPECHECK_CMD="cargo check -p services" WAI_TEST_CMD="cargo test -p services sync_outbox" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-hive-redesign 107` exits 0
