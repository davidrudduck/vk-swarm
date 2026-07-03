---
id: "204"
phase: 2
title: Hive handle_lease_heartbeat — renew leases and reply LeaseGrant per assignment
status: ready
depends_on: ["202", "203"]
parallel: false
conflicts_with: ["202", "205"]
files:
  - crates/remote/src/nodes/ws/session.rs
irreversible: false
scope_test: "crates/remote/src/nodes/ws/session.rs"
allowed_change: edit
covers_criteria: [SC3]
covers_tests: []
---
## Failing test (write first)
**PRECONDITION (Trap 2b — NON-NEGOTIABLE):** a `#[cfg(test)] mod` INSIDE `session.rs` (the handler is
private to the module; an integration test in `crates/remote/tests/` sees only `pub` items). REQUIRES a
live, migrated Postgres (the `201` columns + `node_fencing_token_seq`). A run without `DATABASE_URL`
returns early (skip) = HOLLOW pass. Stand up Postgres, export `DATABASE_URL=postgres://…`, or RAISE.

**Sibling read (rubric #9):** `crates/remote/tests/backfill_e2e.rs` for the `database_url()`/
`skip_without_db!`/`create_pool()`/`create_test_organization`/`create_test_node` helpers (inline verbatim;
no `common` module). The `ws_sender` problem is the SAME one task 106 solved: `send_message` needs a
`SplitSink<WebSocket, Message>` that cannot be cheaply built in a unit test. **Resolve identically to 106:**
extract the renewal logic into a `handle_lease_heartbeat_renew(node_id, assignment_ids, pool) ->
Vec<LeaseGrantOutcome>` (pure DB, no send) that the wire handler calls then sends; test the renew fn
directly. Record this split in the ledger (it keeps the test ws-free while preserving the reply path).

Add to `session.rs`:
```rust
#[cfg(test)]
mod lease_heartbeat_tests {
    use super::*;
    // inline database_url() / skip_without_db! / create_pool() + org/node/assignment fixtures.

    #[tokio::test]
    async fn renew_extends_held_leases_and_returns_a_grant_per_assignment() {
        skip_without_db!();
        let pool = create_pool().await;
        // seed org + node_a; try_claim two tasks for node_a (TaskAssignmentRepository::try_claim).
        // Call handle_lease_heartbeat_renew(node_a, [a1, a2], &pool).await.
        // Assert: one LeaseGrantOutcome per held assignment, each carrying the assignment's CURRENT
        //   fencing_token (UNCHANGED by renewal) and a lease_expires_at strictly in the future.
    }

    #[tokio::test]
    async fn renew_skips_assignments_not_held_by_this_node() {
        skip_without_db!();
        let pool = create_pool().await;
        // node_a holds a1; node_b heartbeats [a1] (a1 is NOT node_b's). renew(node_b, [a1]) yields NO
        // grant for a1 (renew_lease returns None for a foreign holder — task 203). The hive does NOT
        // reply a grant for a lease the node does not hold.
    }
}
```

## Change
- **File:** `crates/remote/src/nodes/ws/session.rs`
- **Anchor:** the `NodeMessage::LeaseHeartbeat { assignment_ids }` STUB arm added by 202 in
  `handle_node_message` (@~580 after 202 lands), plus a NEW `handle_lease_heartbeat` fn placed beside
  `handle_heartbeat` (@582) / `handle_task_sync` (@1547).
- **Sibling read (rubric #9):** `handle_heartbeat` (@582-612) is the reply template — it ends with
  `send_message(ws_sender, &HiveMessage::HeartbeatAck { server_time: Utc::now() }).await`. `handle_op_batch`
  (task 106, beside `handle_task_sync`) is the apply-then-reply template. Reuse `send_message` and the
  `TaskAssignmentRepository` (`crate::db::task_assignments`); do NOT re-implement either.
- **Before (the 202 stub arm):**
```rust
        NodeMessage::LeaseHeartbeat { assignment_ids } => {
            // STUB — filled by task 204 (renew leases, reply LeaseGrant per assignment). Logs so the
            // exhaustive match compiles now; 204 replaces the body with handle_lease_heartbeat(...).
            tracing::debug!(node_id = %node_id, count = assignment_ids.len(),
                "received lease_heartbeat (renew TODO: task 204)");
            Ok(())
        }
```
- **After:**
```rust
        NodeMessage::LeaseHeartbeat { assignment_ids } => {
            handle_lease_heartbeat(node_id, assignment_ids, pool, ws_sender).await
        }
```
- **Add `handle_lease_heartbeat`** (new fn). EXACT contract:
  - **`assignment_ids` is borrowed, NOT owned** (same R1/F3 constraint as 106): `handle_node_message`
    matches on `&NodeMessage`, so the arm binds `assignment_ids: &Vec<Uuid>`. Take a slice:
    `async fn handle_lease_heartbeat(node_id: Uuid, assignment_ids: &[Uuid], pool: &PgPool, ws_sender:
    &mut SplitSink<WebSocket, Message>) -> Result<(), HandleError>`. The call arm passes `assignment_ids`
    (the `&Vec` coerces to `&[Uuid]`).
  - **Renew step (the testable `handle_lease_heartbeat_renew`):** for each `assignment_id`, call
    `TaskAssignmentRepository::new(pool).renew_lease(*assignment_id, node_id, <lease TTL>)`. Use a single
    module-level `const LEASE_TTL: chrono::Duration` (e.g. `Duration::seconds(60)`); document it must
    exceed the node's heartbeat interval (task 206) so a renewing node never expires. `renew_lease`
    returns `Some(LeaseClaim)` for a held lease, `None` for a foreign/missing one — collect the `Some`s.
  - **Reply step:** for each renewed `LeaseClaim`, `send_message(ws_sender, &HiveMessage::LeaseGrant {
    assignment_id: claim.assignment_id, fencing_token: claim.fencing_token, lease_expires_at:
    claim.lease_expires_at }).await.map_err(|_| HandleError::Send)?;`. Do NOT reply for assignments the
    node does not hold (no grant). Return `Ok(())`.
  - **(optional) `LeaseGrantOutcome`** = the renew fn's return element if a named struct is cleaner than
    returning `Vec<LeaseClaim>` directly; either is fine — keep it confined to this file.

## Allowed moves
ONLY: replace the 202 `LeaseHeartbeat` stub body with the `handle_lease_heartbeat(...)` call, add the
`handle_lease_heartbeat` fn (+ the extracted `handle_lease_heartbeat_renew` for the ws-free test), the
`LEASE_TTL` const, and the `#[cfg(test)] mod lease_heartbeat_tests`. Reuse `send_message`,
`TaskAssignmentRepository`, `HiveMessage::LeaseGrant`. Do NOT touch the WS enum definitions (202 owns them),
the fencing check (205), `try_claim`/`renew_lease` bodies (203), the node side, or any migration. Do NOT
change the signatures of existing handlers.

## STOP triggers
- Renewing a lease the node does NOT hold (ignoring `renew_lease`'s `None`) and replying a grant anyway →
  BUG: that hands a node a token for a foreign lease. Only reply grants for `Some(LeaseClaim)`.
- Bumping the fencing token on renewal → BUG: renewal is NOT a reassignment; `renew_lease` (203) leaves the
  token unchanged. If a grant's token differs from the pre-renew token, the renew SQL is wrong (203's test
  also guards this).
- `LEASE_TTL` ≤ the node heartbeat interval (206) → leases expire under a healthy node. Choose TTL > the
  node's renew cadence; cross-check with 206's interval const and note the relationship in the ledger.
- `handle_lease_heartbeat` cannot reach `ws_sender` → it is a parameter threaded from `handle_node_message`
  (which has `ws_sender` @508), exactly like `handle_op_batch`. Take `ws_sender` from the start; do NOT
  change existing handler signatures.
- `query!`/macro forms tempting → use `TaskAssignmentRepository::renew_lease` (runtime queries, 203); no
  offline cache entry needed.

## Done when
`WAI_TYPECHECK_CMD="cargo check -p remote" WAI_TEST_CMD='test -n "$DATABASE_URL" && cargo test -p remote lease_heartbeat' bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-hive-redesign 204` exits 0
(run with `DATABASE_URL=postgres://…` against a migrated Postgres — Trap 2b; the `test -n` prefix is
FAIL-CLOSED.)
