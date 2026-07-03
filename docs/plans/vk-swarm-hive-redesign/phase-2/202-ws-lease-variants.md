---
id: "202"
phase: 2
title: Add LeaseHeartbeat/LeaseGrant/LeaseRevoked WS variants to both crates + exhaustive stub arms
status: done
depends_on: []
parallel: false
conflicts_with: ["204", "205", "206", "501"]
files:
  - crates/services/src/services/hive_client.rs
  - crates/remote/src/nodes/ws/message.rs
  - crates/remote/src/nodes/ws/session.rs
irreversible: false
scope_test: "N/A"
allowed_change: mixed
covers_criteria: []
covers_tests: []
---
## Failing test (write first)
N/A — this task adds wire variants + stub match arms only; behavior is filled by 204 (hive renew), 205
(fencing), and 206 (node lease state). Its sole obligation is that BOTH crates still compile under
`-D warnings` with the new enum variants exhaustively matched (Trap 3). Proven by `cargo check
--workspace` (the `## Done when` command) — a missing arm on either exhaustive match site fails to compile.
The runtime paths are exercised by 204/205/206/210.

Mirrors Phase-1 task **103** exactly (same dual-crate enum-edit + stub-arm shape). The only two exhaustive
(no-`_`) match sites this task must satisfy (same two as 103):
- node: `handle_hive_message` `hive_client.rs:972` — explicit `LeaseGrant` + `LeaseRevoked` arms BEFORE
  the `_ =>` wildcard @1062 (the wildcard silently drops unhandled hive→node variants — the #1 bug here).
- hive: `handle_node_message` `session.rs:512` — exhaustive; needs a named `LeaseHeartbeat` arm.

**This task does NOT touch `HiveEvent`** (`hive_client.rs:661`). Mapping `LeaseGrant`/`LeaseRevoked` into
`HiveEvent` variants consumed by `node_runner.process_event` is task 206's job — exactly as 103 left
`HiveEvent::OpAck` to the consumer task 108. Here the node arm only logs a stub.

## Change

### 1. Node enums — `crates/services/src/services/hive_client.rs`
- **Anchor:** `enum NodeMessage` (@82, tail `BackfillResponse` @116-118), `enum HiveMessage` (@123, tail
  `BackfillRequest` @149-151), and the node→hive dispatch wildcard in `handle_hive_message` (@1062).
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
    /// Periodic lease renewal: the node's in-flight hive-assignment ids to keep alive (SC3, CONTRACT §A).
    #[serde(rename = "lease_heartbeat")]
    LeaseHeartbeat { assignment_ids: Vec<Uuid> },
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
    /// Lease granted/renewed: the assignment's current fencing token + lease expiry (SC3, CONTRACT §A).
    #[serde(rename = "lease_grant")]
    LeaseGrant {
        assignment_id: Uuid,
        fencing_token: i64,
        lease_expires_at: chrono::DateTime<Utc>,
    },
    /// Lease revoked: the hive reclaimed/expired this assignment; the node must self-fence (SC3).
    #[serde(rename = "lease_revoked")]
    LeaseRevoked { assignment_id: Uuid, reason: String },
}
```
- **Node `handle_hive_message` — add explicit `LeaseGrant` + `LeaseRevoked` STUB arms BEFORE the `_ =>`
  wildcard @1062:**
  - **Before (@1062-1064):**
```rust
            _ => {
                tracing::debug!(?hive_msg, "ignoring unhandled hive message");
            }
```
  - **After:**
```rust
            HiveMessage::LeaseGrant { assignment_id, fencing_token, lease_expires_at } => {
                // STUB — filled by task 206 (store token+lease, emit HiveEvent::LeaseGranted). Explicit
                // arm so it is NOT swallowed by the `_ =>` wildcard below.
                tracing::debug!(%assignment_id, fencing_token, %lease_expires_at,
                    "received lease_grant (store TODO: task 206)");
            }
            HiveMessage::LeaseRevoked { assignment_id, reason } => {
                // STUB — filled by task 206 (emit HiveEvent::LeaseRevoked → self-fence). Explicit arm.
                tracing::debug!(%assignment_id, %reason, "received lease_revoked (handle TODO: task 206)");
            }
            _ => {
                tracing::debug!(?hive_msg, "ignoring unhandled hive message");
            }
```
  > The `_ =>` wildcard at @1062 silently drops unhandled hive→node variants — the explicit `LeaseGrant`/
  > `LeaseRevoked` arms MUST precede it or 206's lease store is silently dead. Keep the wildcard.

### 2. Hive enums — `crates/remote/src/nodes/ws/message.rs`
- **Anchor:** `enum NodeMessage` (@15, tail @83-86), `enum HiveMessage` (@91, tail @139-142). `Uuid` is in
  scope (@8) and `chrono::Utc`/`DateTime` is the convention used by `HeartbeatAck { server_time }`.
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

    /// Periodic lease renewal: the node's in-flight hive-assignment ids to keep alive (SC3, CONTRACT §A).
    #[serde(rename = "lease_heartbeat")]
    LeaseHeartbeat { assignment_ids: Vec<Uuid> },
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

    /// Lease granted/renewed: the assignment's current fencing token + lease expiry (SC3, CONTRACT §A).
    #[serde(rename = "lease_grant")]
    LeaseGrant {
        assignment_id: Uuid,
        fencing_token: i64,
        lease_expires_at: chrono::DateTime<Utc>,
    },

    /// Lease revoked: the hive reclaimed/expired this assignment; the node must self-fence (SC3).
    #[serde(rename = "lease_revoked")]
    LeaseRevoked { assignment_id: Uuid, reason: String },
}
```

### 3. Hive dispatch STUB arm — `crates/remote/src/nodes/ws/session.rs`
- **Anchor:** `handle_node_message` match (@512-578) — EXHAUSTIVE (no `_`), so the new
  `NodeMessage::LeaseHeartbeat` forces a new arm (Trap 3).
- **Before (the `BackfillResponse` tail arm @575-578):**
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
        NodeMessage::LeaseHeartbeat { assignment_ids } => {
            // STUB — filled by task 204 (renew leases, reply LeaseGrant per assignment). Logs so the
            // exhaustive match compiles now; 204 replaces the body with handle_lease_heartbeat(...).
            tracing::debug!(node_id = %node_id, count = assignment_ids.len(),
                "received lease_heartbeat (renew TODO: task 204)");
            Ok(())
        }
    }
}
```

## Allowed moves
ONLY: add `LeaseHeartbeat` to `NodeMessage` and `LeaseGrant`/`LeaseRevoked` to `HiveMessage` in BOTH
crates (identical variant names, serde renames, field names/order/types), add the explicit node stub arms
BEFORE its `_ =>` wildcard, and add the named hive `LeaseHeartbeat` stub arm in the exhaustive match. Do
NOT write renew/fencing/self-fence logic (204/205/206 own that), do NOT touch `HiveEvent`, do NOT thread
`ws_sender` anywhere new, do NOT touch the migration or `task_assignments.rs`.

## STOP triggers
- The `_ =>` wildcard is NOT at `hive_client.rs:1062` (file drifted, e.g. 103 already shifted line numbers)
  → re-locate it; the explicit `LeaseGrant`/`LeaseRevoked` arms MUST precede the wildcard or 206 is dead.
- `handle_node_message` (`session.rs:512`) has gained a `_` wildcard since authoring → STILL add the named
  `LeaseHeartbeat` arm (204 needs a named arm to replace).
- A THIRD exhaustive (no-`_`) match on either enum exists beyond the two listed → STOP: `files:` is
  incomplete; add the site and record in the ledger.
- The variant definitions diverge in name/order/type between the two crates → STOP; the wire contract must
  be byte-for-byte identical (dual-definition convention, decisions-ledger).
- `chrono::DateTime<Utc>` does not resolve in `message.rs` → use the same path `HeartbeatAck` uses in that
  file; do NOT add a new import beyond what that variant already relies on.

## Manual verification (record in decisions-ledger)
This task adds wire variants + stub arms only (no runtime behavior to unit-test); verification is the
workspace compile under `-D warnings`, which proves BOTH exhaustive match sites are satisfied (Trap 3):
- `cargo check --workspace` → exits 0. Record the exit status. A missing arm on either end fails this.
- `grep -n "LeaseHeartbeat\|LeaseGrant\|LeaseRevoked" crates/services/src/services/hive_client.rs
  crates/remote/src/nodes/ws/message.rs` → shows the variants present in BOTH crates with identical field
  shapes. Record the diff of the two variant blocks (must match).
- `grep -n "HiveMessage::LeaseGrant\|HiveMessage::LeaseRevoked" crates/services/src/services/hive_client.rs`
  → the explicit arms appear BEFORE the `_ =>` line (compare line numbers). Record both line numbers.

## Done when
`WAI_TYPECHECK_CMD="cargo check --workspace" WAI_TEST_CMD="cargo check --workspace" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-hive-redesign 202` exits 0
(workspace check because the arms span the `services` and `remote` crates — Trap 1/Trap 3.)
