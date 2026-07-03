---
id: "103"
phase: 1
title: Add OpBatch/OutboxOp/OpAck WS variants to both crates + exhaustive stub arms
status: ready
depends_on: []
parallel: false
conflicts_with: ["106", "108"]
files:
  - crates/services/src/services/hive_client.rs
  - crates/remote/src/nodes/ws/message.rs
  - crates/remote/src/nodes/ws/session.rs
irreversible: false
scope_test: "N/A"
allowed_change: mixed
covers_criteria: []
---
## Failing test (write first)
N/A — this task adds wire variants + stub match arms only; behavior is filled by 106 (hive apply) and
108 (node ack). Its sole obligation is that BOTH crates still compile under `-D warnings` with the new
enum variants exhaustively matched. Proven by `cargo check --workspace` (the `## Done when` command) —
a missing arm on either exhaustive match site fails to compile (Trap 3). 106's and 108's tests exercise
the runtime paths.

Verified exhaustive (no-`_`) match sites that this task MUST satisfy (the only two; `parse_auth_response`
@958, `session.rs:373` auth, and `relay.rs:347` all already carry a `_` wildcard):
- node: `handle_hive_message` `hive_client.rs:972` — explicit `OpAck` arm before the `_ =>` @1062.
- hive: `handle_node_message` `session.rs:512` — exhaustive, needs a named `OpBatch` arm.

## Change

### 1. Node enums — `crates/services/src/services/hive_client.rs`
- **Anchor:** `enum NodeMessage` (@82, tail @116-118), `enum HiveMessage` (@123, tail @149-151), the new
  `OutboxOp` struct beside the other message structs (after `NodeRemovedMessage` @262-268), and the
  node→hive dispatch wildcard in `handle_hive_message` (@1062).
- **Before (NodeMessage tail, @116-118):**
```rust
    #[serde(rename = "backfill_response")]
    BackfillResponse(BackfillResponseMessage),
}
```
- **After:**
```rust
    #[serde(rename = "backfill_response")]
    BackfillResponse(BackfillResponseMessage),
    /// Ordered batch of outbox ops (node→hive op-log, SC2). Tracer scope: op_type = "task.upsert".
    #[serde(rename = "op_batch")]
    OpBatch { ops: Vec<OutboxOp> },
}
```
- **Before (HiveMessage tail, @149-151):**
```rust
    #[serde(rename = "backfill_request")]
    BackfillRequest(BackfillRequestMessage),
}
```
- **After:**
```rust
    #[serde(rename = "backfill_request")]
    BackfillRequest(BackfillRequestMessage),
    /// Durable ack of the node op-log: all ops with seq <= applied_through_seq are persisted (SC2c).
    #[serde(rename = "op_ack")]
    OpAck { applied_through_seq: i64 },
}
```
- **Add the `OutboxOp` struct** (place after `NodeRemovedMessage`, ~@268):
```rust
/// A single node→hive op-log operation (SC2). Mirrors the `node_outbox` row shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboxOp {
    pub seq: i64,
    pub op_type: String,
    pub entity_type: String,
    pub entity_id: Uuid,
    pub payload: serde_json::Value,
    pub idempotency_key: String,
    pub fencing_token: Option<i64>,
}
```
- **Node `handle_hive_message` — add an explicit `OpAck` STUB arm BEFORE the `_ =>` wildcard @1062:**
  - **Before (@1062-1064):**
```rust
            _ => {
                tracing::debug!(?hive_msg, "ignoring unhandled hive message");
            }
```
  - **After:**
```rust
            HiveMessage::OpAck { applied_through_seq } => {
                // STUB — filled by task 108 (advance the node_outbox ack cursor). For now log only so
                // the arm is EXPLICIT (not swallowed by the `_ =>` wildcard below) and compiles.
                tracing::debug!(applied_through_seq, "received op_ack (cursor advance TODO: task 108)");
            }
            _ => {
                tracing::debug!(?hive_msg, "ignoring unhandled hive message");
            }
```
  > The `_ =>` wildcard at @1062 silently drops unhandled hive→node variants — the explicit `OpAck` arm
  > MUST precede it (decisions-ledger; the #1 easy-to-miss bug here). Keep the wildcard for the others.

### 2. Hive enums — `crates/remote/src/nodes/ws/message.rs`
- **Anchor:** `enum NodeMessage` (@15, tail @83-86), `enum HiveMessage` (@91, tail @139-142), and the
  `OutboxOp` struct near the other message structs (after the enums, before `AuthMessage` @144-159 is
  fine — anywhere a `pub struct` lives in this module).
- **Before (NodeMessage tail, @83-86):**
```rust
    /// Response to a backfill request from hive
    #[serde(rename = "backfill_response")]
    BackfillResponse(BackfillResponseMessage),
}
```
- **After:**
```rust
    /// Response to a backfill request from hive
    #[serde(rename = "backfill_response")]
    BackfillResponse(BackfillResponseMessage),

    /// Ordered batch of outbox ops (node→hive op-log, SC2). Tracer scope: op_type = "task.upsert".
    #[serde(rename = "op_batch")]
    OpBatch { ops: Vec<OutboxOp> },
}
```
- **Before (HiveMessage tail, @139-142):**
```rust
    /// Request data backfill from node
    #[serde(rename = "backfill_request")]
    BackfillRequest(BackfillRequestMessage),
}
```
- **After:**
```rust
    /// Request data backfill from node
    #[serde(rename = "backfill_request")]
    BackfillRequest(BackfillRequestMessage),

    /// Durable ack of the node op-log: all ops with seq <= applied_through_seq are persisted (SC2c).
    #[serde(rename = "op_ack")]
    OpAck { applied_through_seq: i64 },
}
```
- **Add the IDENTICAL `OutboxOp` struct** (same field names/order/types as the node copy — the wire
  contract must match byte-for-byte). `serde_json` IS a remote dep (`crates/remote/Cargo.toml:23`) and
  `Uuid` is in scope (@8):
```rust
/// A single node→hive op-log operation (SC2). Mirrors the node's `node_outbox` row shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboxOp {
    pub seq: i64,
    pub op_type: String,
    pub entity_type: String,
    pub entity_id: Uuid,
    pub payload: serde_json::Value,
    pub idempotency_key: String,
    pub fencing_token: Option<i64>,
}
```

### 3. Hive dispatch STUB arm — `crates/remote/src/nodes/ws/session.rs`
- **Anchor:** `handle_node_message` match (@512-579) — EXHAUSTIVE (no `_`), so the new
  `NodeMessage::OpBatch` variant forces a new arm or the workspace won't compile (Trap 3).
- **Before (@575-579):**
```rust
        NodeMessage::BackfillResponse(response) => {
            handle_backfill_response(node_id, response, pool, tracker).await
        }
    }
}
```
- **After:**
```rust
        NodeMessage::BackfillResponse(response) => {
            handle_backfill_response(node_id, response, pool, tracker).await
        }
        NodeMessage::OpBatch { ops } => {
            // STUB — filled by task 106 (idempotent apply + durable OpAck). Logs so the exhaustive
            // match compiles now; 106 replaces the body with handle_op_batch(...).
            tracing::debug!(node_id = %node_id, op_count = ops.len(), "received op_batch (apply TODO: task 106)");
            Ok(())
        }
    }
}
```

## Allowed moves
ONLY: add `OpBatch` to `NodeMessage` and `OpAck` to `HiveMessage` in BOTH crates, add the identical
`OutboxOp` struct to both, add the explicit `OpAck` stub arm in the node before its `_ =>` wildcard, and
add the named `OpBatch` stub arm in the hive's exhaustive match. Do NOT write apply/ack logic (106/108
own that), do NOT thread `ws_sender` anywhere, do NOT touch the node_outbox/node_op_log tables or any
migration.

## STOP triggers
- The `_ =>` wildcard is NOT at `hive_client.rs:1062` (file drifted) → re-locate it; the explicit `OpAck`
  arm MUST precede the wildcard or 108's cursor advance is silently dead.
- `handle_node_message` (`session.rs:512`) has gained a `_` wildcard since authoring → STILL add the
  named `OpBatch` arm (106 needs a named arm to replace; do not let it fall into a wildcard).
- A THIRD exhaustive (no-`_`) match on either enum exists beyond the two listed above (re-verify:
  `grep -rn "NodeMessage::Auth\|HiveMessage::TaskAssign" crates/services/src crates/remote/src`) → STOP:
  `files:` is incomplete; add the site and record in the ledger.
- The two `OutboxOp` definitions diverge in field name/order/type → STOP; the wire contract must be
  identical across crates (dual-definition convention, decisions-ledger).

## Manual verification (record in decisions-ledger)
This task adds wire variants + stub arms only (no runtime behavior to unit-test); verification is the
workspace compile under `-D warnings`, which proves BOTH exhaustive match sites are satisfied (Trap 3):
- `cargo check --workspace` → exits 0. Record the exit status. A missing arm on either end (node
  `handle_hive_message`, hive `handle_node_message`) fails this compile.
- `grep -n "OpBatch\|OpAck\|struct OutboxOp" crates/services/src/services/hive_client.rs
  crates/remote/src/nodes/ws/message.rs` → shows the variants + struct present in BOTH crates with
  identical `OutboxOp` fields. Record the diff of the two struct blocks (must match).
- `grep -n "HiveMessage::OpAck" crates/services/src/services/hive_client.rs` → the explicit arm appears
  BEFORE the `_ =>` line (compare line numbers). Record both line numbers.

## Done when
`WAI_TYPECHECK_CMD="cargo check --workspace" WAI_TEST_CMD="cargo check --workspace" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-hive-redesign 103` exits 0
(workspace check because the arms span the `services` and `remote` crates — Trap 1/Trap 3.)
