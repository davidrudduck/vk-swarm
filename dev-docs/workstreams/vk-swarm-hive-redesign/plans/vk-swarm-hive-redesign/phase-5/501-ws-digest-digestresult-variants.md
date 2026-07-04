---
id: "501"
phase: 5
title: Add Digest/DigestEntry/DigestResult WS variants to both crates + exhaustive stub arms
status: done
depends_on: ["202"]
parallel: false
conflicts_with: ["202", "503", "504"]
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
N/A — this task adds the anti-entropy wire variants + stub match arms only; behavior is filled by 503
(hive compare→reply) and 504 (node acts on DigestResult). Its sole obligation is that BOTH crates still
compile under `-D warnings` with the new enum variants exhaustively matched. Proven by `cargo check
--workspace` (the `## Done when` command) — a missing arm on either exhaustive match site fails to
compile (Trap 3). 503's and 504's tests exercise the runtime paths.

Verified exhaustive (no-`_`) match sites that this task MUST satisfy (the only two — same two 103 hit;
`parse_auth_response`, `session.rs:373` auth, and `relay.rs:347` all already carry a `_` wildcard):
- node: `handle_hive_message` `hive_client.rs` — explicit `DigestResult` arm BEFORE the `_ =>` wildcard
  (the wildcard is at `hive_client.rs:1062` per 103; re-locate if it drifted — STOP trigger below).
- hive: `handle_node_message` `session.rs:512` — exhaustive (no `_`), needs a named `Digest` arm.

**These shapes are FIXED by CONTRACT §A — do NOT widen them.** `DigestEntry` is `{ entity_type, entity_id,
version }` (NO hash field). `Digest` is `{ entries: Vec<DigestEntry> }` (NO outbox-high-water field — the
hive reads its own cursor from `node_op_log`). `DigestResult` is `{ resend_from_seq: Option<i64>,
pull_entities: Vec<Uuid> }`. Adding any field is a Trap-6 CONTRACT divergence → STOP and escalate first.

## Change

### 1. Node enums — `crates/services/src/services/hive_client.rs`
- **Anchor:** `enum NodeMessage` (tail `BackfillResponse(BackfillResponseMessage)` @116-117), `enum
  HiveMessage` (tail `BackfillRequest(BackfillRequestMessage)` @149-150), the new `DigestEntry` struct
  beside the other message structs (after `BackfillResponseMessage` @645 is fine — anywhere a `pub
  struct` lives in this module), and the node→hive dispatch wildcard in `handle_hive_message` (@1062).
  > **NOTE (103 shifted these tails):** P1's 103 appended `OpBatch` to `NodeMessage` and `OpAck` to
  > `HiveMessage` in this same tail region. Re-anchor on the LITERAL last arm before each enum's closing
  > `}` (which is now 103's `OpBatch`/`OpAck`, not `BackfillResponse`/`BackfillRequest`). Append after
  > whichever arm is last. The Before/After below quote the pre-103 tail; adjust to the post-103 tail.
- **Before (NodeMessage tail — the last arm before the closing `}`; pre-103 it is `BackfillResponse`):**
```rust
    #[serde(rename = "backfill_response")]
    BackfillResponse(BackfillResponseMessage),
}
```
- **After (insert the new variant immediately before the enum's closing `}`):**
```rust
    #[serde(rename = "backfill_response")]
    BackfillResponse(BackfillResponseMessage),
    /// Anti-entropy digest: per-entity version snapshot the hive compares against its own state to
    /// detect silent divergence the ack cursor misses (SC5, CONTRACT §A). Tracer scope: entity_type "task".
    #[serde(rename = "digest")]
    Digest { entries: Vec<DigestEntry> },
}
```
- **Before (HiveMessage tail — the last arm before the closing `}`; pre-103 it is `BackfillRequest`):**
```rust
    #[serde(rename = "backfill_request")]
    BackfillRequest(BackfillRequestMessage),
}
```
- **After:**
```rust
    #[serde(rename = "backfill_request")]
    BackfillRequest(BackfillRequestMessage),
    /// Reply to a node Digest (SC5, CONTRACT §A): `resend_from_seq` asks the node to re-stream its
    /// op-log from that seq (node-has/hive-lacks heal); `pull_entities` lists shared-task ids the hive
    /// has that the node lacks (hive-has/node-lacks heal via the bulk-snapshot reconcile leg).
    #[serde(rename = "digest_result")]
    DigestResult { resend_from_seq: Option<i64>, pull_entities: Vec<Uuid> },
}
```
- **Add the `DigestEntry` struct** (place after `BackfillResponseMessage`, ~@658). `Uuid` is in scope:
```rust
/// One entry in a node→hive anti-entropy Digest (SC5, CONTRACT §A). `entity_id` is the node's LOCAL
/// task id (= the hive's `shared_tasks.source_task_id` for this node — the id bridge); `version` is the
/// node's `Task::remote_version`. Mirrored byte-for-byte in the hive crate's `message.rs`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DigestEntry {
    pub entity_type: String,
    pub entity_id: Uuid,
    pub version: i64,
}
```
- **Node `handle_hive_message` — add an explicit `DigestResult` STUB arm BEFORE the `_ =>` wildcard @1062:**
  - **Before (@1062-1064):**
```rust
            _ => {
                tracing::debug!(?hive_msg, "ignoring unhandled hive message");
            }
```
  - **After:**
```rust
            HiveMessage::DigestResult { resend_from_seq, pull_entities } => {
                // STUB — filled by task 504 (re-stream from resend_from_seq + pull listed entities).
                // For now log only so the arm is EXPLICIT (not swallowed by the `_ =>` wildcard below).
                tracing::debug!(
                    ?resend_from_seq,
                    pull_count = pull_entities.len(),
                    "received digest_result (heal TODO: task 504)"
                );
            }
            _ => {
                tracing::debug!(?hive_msg, "ignoring unhandled hive message");
            }
```
  > The `_ =>` wildcard at @1062 silently drops unhandled hive→node variants — the explicit
  > `DigestResult` arm MUST precede it (decisions-ledger; the #1 easy-to-miss bug here, same as 103's
  > `OpAck`). Keep the wildcard for the others. (Note: 103 added an `OpAck` arm here too; place
  > `DigestResult` beside it, both before the `_ =>`.)

### 2. Hive enums — `crates/remote/src/nodes/ws/message.rs`
- **Anchor:** `enum NodeMessage` (tail `BackfillResponse` region per 103), `enum HiveMessage` (tail
  `BackfillRequest` region per 103), and the `DigestEntry` struct near the other message structs. 103
  added `OpBatch`/`OpAck`/`OutboxOp` in this same tail region; re-anchor on the LITERAL last arm before
  each enum's closing `}` (post-103) and append after it.
- **Before (NodeMessage tail — the `BackfillResponse` arm, before whatever is now the last arm):**
```rust
    /// Response to a backfill request from hive
    #[serde(rename = "backfill_response")]
    BackfillResponse(BackfillResponseMessage),
```
- **After (insert the new variant immediately before the enum's closing `}` — after 103's `OpBatch` if
  present):**
```rust
    /// Response to a backfill request from hive
    #[serde(rename = "backfill_response")]
    BackfillResponse(BackfillResponseMessage),

    /// Anti-entropy digest (SC5, CONTRACT §A). Tracer scope: entity_type "task".
    #[serde(rename = "digest")]
    Digest { entries: Vec<DigestEntry> },
```
- **Before (HiveMessage tail — the `BackfillRequest` arm):**
```rust
    /// Request data backfill from node
    #[serde(rename = "backfill_request")]
    BackfillRequest(BackfillRequestMessage),
```
- **After:**
```rust
    /// Request data backfill from node
    #[serde(rename = "backfill_request")]
    BackfillRequest(BackfillRequestMessage),

    /// Reply to a node Digest (SC5, CONTRACT §A).
    #[serde(rename = "digest_result")]
    DigestResult { resend_from_seq: Option<i64>, pull_entities: Vec<Uuid> },
```
- **Add the IDENTICAL `DigestEntry` struct** (same field names/order/types as the node copy — the wire
  contract must match byte-for-byte). `Uuid` is in scope (@8):
```rust
/// One entry in a node→hive anti-entropy Digest (SC5, CONTRACT §A). `entity_id` is the node's local
/// task id (= `shared_tasks.source_task_id` for this node — the id bridge); `version` mirrors the node's
/// `Task::remote_version`. Mirrors the node's `hive_client.rs` copy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DigestEntry {
    pub entity_type: String,
    pub entity_id: Uuid,
    pub version: i64,
}
```

### 3. Hive dispatch STUB arm — `crates/remote/src/nodes/ws/session.rs`
- **Anchor:** `handle_node_message` match (@512-579) — EXHAUSTIVE (no `_`), so the new
  `NodeMessage::Digest` variant forces a new arm or the workspace won't compile (Trap 3). 103 added an
  `OpBatch` arm just before the closing `}` here; place the `Digest` arm beside it.
- **Before (the match's final arm region — the `BackfillResponse` arm before the closing `}`):**
```rust
        NodeMessage::BackfillResponse(response) => {
            handle_backfill_response(node_id, response, pool, tracker).await
        }
```
- **After (append the new arm; if 103's `OpBatch` arm already follows `BackfillResponse`, append AFTER
  it — the key is the named `Digest` arm exists inside the exhaustive match):**
```rust
        NodeMessage::BackfillResponse(response) => {
            handle_backfill_response(node_id, response, pool, tracker).await
        }
        NodeMessage::Digest { entries } => {
            // STUB — filled by task 503 (compare against shared_tasks + node_op_log, reply DigestResult).
            // Logs so the exhaustive match compiles now; 503 replaces the body with handle_digest(...).
            tracing::debug!(node_id = %node_id, entry_count = entries.len(), "received digest (compare TODO: task 503)");
            Ok(())
        }
```

## Allowed moves
ONLY: add `Digest` to `NodeMessage` and `DigestResult` to `HiveMessage` in BOTH crates, add the
identical `DigestEntry` struct to both, add the explicit `DigestResult` stub arm in the node before its
`_ =>` wildcard, and add the named `Digest` stub arm in the hive's exhaustive match. Do NOT write
compare/heal logic (503/504 own that), do NOT thread `ws_sender` anywhere, do NOT touch the
node_outbox/node_op_log tables, the `OutboxRepository`, or any migration. The variant shapes are FIXED by
CONTRACT §A — do not add a hash or high-water field.

## STOP triggers
- The `_ =>` wildcard is NOT at `hive_client.rs:1062` (file drifted, e.g. 103's `OpAck` arm shifted it)
  → re-locate the wildcard; the explicit `DigestResult` arm MUST precede it or 504's heal is silently dead.
- `handle_node_message` (`session.rs:512`) has gained a `_` wildcard since authoring → STILL add the
  named `Digest` arm (503 needs a named arm to replace; do not let it fall into a wildcard).
- A THIRD exhaustive (no-`_`) match on either enum exists beyond the two listed above (re-verify:
  `grep -rn "NodeMessage::Auth\|HiveMessage::TaskAssign" crates/services/src crates/remote/src`) → STOP:
  `files:` is incomplete; add the site and record in the ledger.
- The two `DigestEntry` definitions diverge in field name/order/type, OR a `hash`/high-water field is
  tempting → STOP; the wire contract must be identical across crates AND match CONTRACT §A exactly
  (dual-definition convention + frozen interface, decisions-ledger Trap 3/Trap 6).
- 103 has NOT landed and the enum tails do not match the anchors quoted here → P5 rides P1's op-log; 103
  (the P1 protocol task) must be `passed` first (this workstream is phase-by-phase). Re-anchor on the
  literal `BackfillResponse`/`BackfillRequest` tail lines.

## Manual verification (record in decisions-ledger)
This task adds wire variants + stub arms only (no runtime behavior to unit-test); verification is the
workspace compile under `-D warnings`, which proves BOTH exhaustive match sites are satisfied (Trap 3):
- `cargo check --workspace` → exits 0. Record the exit status. A missing arm on either end (node
  `handle_hive_message`, hive `handle_node_message`) fails this compile.
- `grep -n "Digest\|DigestResult\|struct DigestEntry" crates/services/src/services/hive_client.rs
  crates/remote/src/nodes/ws/message.rs` → shows the variants + struct present in BOTH crates with
  identical `DigestEntry` fields. Record the diff of the two struct blocks (must match, and must match
  CONTRACT §A — `{ entity_type, entity_id, version }`).
- `grep -n "HiveMessage::DigestResult" crates/services/src/services/hive_client.rs` → the explicit arm
  appears BEFORE the `_ =>` line (compare line numbers). Record both line numbers.

## Done when
`WAI_TYPECHECK_CMD="cargo check --workspace" WAI_TEST_CMD="cargo check --workspace" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-hive-redesign 501` exits 0
(workspace check because the arms span the `services` and `remote` crates — Trap 1/Trap 3.)
