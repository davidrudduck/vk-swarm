---
id: "206"
phase: 2
title: Node lease state — HiveEvent Lease variants, store token+expiry on ActiveAssignment, send LeaseHeartbeat
status: done
depends_on: ["202"]
parallel: false
conflicts_with: ["202", "207", "208"]
files:
  - crates/services/src/services/hive_client.rs
  - crates/services/src/services/node_runner.rs
irreversible: false
scope_test: "crates/services/src/services/node_runner.rs"
allowed_change: edit
covers_criteria: [SC3]
covers_tests: []
---
## Failing test (write first)
A live round-trip is out of hermetic scope here (the WS connection + hive are integration concerns
covered by 210); this task's own obligation is the node-local lease **store** + the `HiveEvent` plumbing.
Add a hermetic unit test (no
DB, no WS — the state is in-memory) in a `#[cfg(test)] mod` inside `node_runner.rs` beside the existing
tests. The lease fields live ON `ActiveAssignment` in `NodeRunnerState` (NOT a separate store — see Change),
so the test exercises the grant/revoke effect on that map:
```rust
#[cfg(test)]
mod lease_state_tests {
    use super::*;

    #[tokio::test]
    async fn lease_grant_sets_token_and_expiry_on_active_assignment_then_revoke_clears() {
        let state = std::sync::Arc::new(tokio::sync::RwLock::new(NodeRunnerState::default()));
        let aid = uuid::Uuid::new_v4();
        let local = uuid::Uuid::new_v4();
        // Seed an active assignment (as HiveEvent::TaskAssigned would).
        state.write().await.active_assignments.insert(aid, ActiveAssignment {
            assignment_id: aid, task_id: uuid::Uuid::new_v4(), local_task_id: Some(local),
            local_attempt_id: None, status: TaskExecutionStatus::Pending,
            fencing_token: None, lease_expires_at: None,
        });
        let expires = chrono::Utc::now() + chrono::Duration::seconds(60);

        // apply_lease_grant is the small helper the process_event arm calls (testable, no WS).
        apply_lease_grant(&state, aid, 7, expires).await;
        {
            let s = state.read().await;
            let a = s.active_assignments.get(&aid).unwrap();
            assert_eq!(a.fencing_token, Some(7));
            assert_eq!(a.lease_expires_at, Some(expires));
        }
        // A higher token replaces a lower one; a grant never lowers a token.
        apply_lease_grant(&state, aid, 9, expires).await;
        assert_eq!(state.read().await.active_assignments.get(&aid).unwrap().fencing_token, Some(9));

        apply_lease_revoke(&state, aid).await;
        assert_eq!(state.read().await.active_assignments.get(&aid).unwrap().fencing_token, None);
    }
}
```
> Match `TaskExecutionStatus`'s real variant name (read it; the literal above is illustrative). If you
> prefer inline arms over `apply_lease_grant`/`apply_lease_revoke` helpers, test the equivalent effect;
> keep the test ws-free and db-free. Record the placement in the ledger.

## Change
This is the **consumer** task for the 202 wire variants — it mirrors Phase-1 task 108 (which added
`HiveEvent::OpAck` + the node_runner arm for the 103 stub). `HiveEvent`'s ONLY two sites are
`hive_client.rs` (definition + `handle_hive_message` emit) and `node_runner.rs` (`process_event` consume) —
grep-verified in the Phase-1 ledger (#5). No third match site.

**DESIGN (reconciled with 207/208 — decisions-ledger judgment call):** the per-task fencing token must be
looked up at op-stream time BY `local_task_id` (207) and the lease expiry read for the watchdog (208). The
node already tracks `active_assignments: HashMap<assignment_id, ActiveAssignment{ …, local_task_id }>` on
the shared `Arc<RwLock<NodeRunnerState>>` (node_runner.rs:257, populated @423). So the lease state lives as
**two new fields on `ActiveAssignment`**, NOT a separate `LeaseStore` — one structure serves 206/207/208 and
keeps the `local_task_id → token` lookup a single map read.

### 1. `crates/services/src/services/node_runner.rs`
- **Anchor A — `struct ActiveAssignment` (@238-246):** add two fields:
```rust
    /// Current fencing token from the hive lease grant (SC3). None until a LeaseGrant arrives, or for
    /// node-owned work (no hive assignment).
    pub fencing_token: Option<i64>,
    /// Lease expiry from the hive (SC3). The self-fence watchdog (task 208) halts the agent if this
    /// passes without a renewal.
    pub lease_expires_at: Option<chrono::DateTime<Utc>>,
```
  Update the `ActiveAssignment { … }` literal where assignments are inserted (@423-428) to set both to
  `None` initially (the grant fills them). (`Utc` is in scope — the file uses `chrono`/`Utc` already; if
  not imported, add `use chrono::{DateTime, Utc};` confined to this file.)
- **Anchor B — `process_event` match (@341, exhaustive over `&HiveEvent`):** add arms for the two new
  variants (added in step 2), mirroring the existing `TaskAssigned`/`TaskCancelled` arms that already
  `self.state.write().await` and mutate `active_assignments`:
```rust
            HiveEvent::LeaseGranted { assignment_id, fencing_token, lease_expires_at } => {
                apply_lease_grant(&self.state, *assignment_id, *fencing_token, *lease_expires_at).await;
                tracing::debug!(%assignment_id, fencing_token, "stored lease grant");
            }
            HiveEvent::LeaseRevoked { assignment_id, reason } => {
                apply_lease_revoke(&self.state, *assignment_id).await;
                tracing::warn!(%assignment_id, %reason, "lease revoked — agent halt is task 208");
                // The actual agent halt is task 208's watchdog/handler; here we only clear the lease.
            }
```
  Add the two small free helpers (testable, ws-free) near the bottom of the module:
```rust
    async fn apply_lease_grant(
        state: &std::sync::Arc<tokio::sync::RwLock<NodeRunnerState>>,
        assignment_id: Uuid, fencing_token: i64, lease_expires_at: chrono::DateTime<Utc>,
    ) {
        let mut s = state.write().await;
        if let Some(a) = s.active_assignments.get_mut(&assignment_id) {
            // Never lower a token (monotonic): only accept a >= token.
            if a.fencing_token.map_or(true, |t| fencing_token >= t) {
                a.fencing_token = Some(fencing_token);
                a.lease_expires_at = Some(lease_expires_at);
            }
        }
    }
    async fn apply_lease_revoke(
        state: &std::sync::Arc<tokio::sync::RwLock<NodeRunnerState>>, assignment_id: Uuid,
    ) {
        let mut s = state.write().await;
        if let Some(a) = s.active_assignments.get_mut(&assignment_id) {
            a.fencing_token = None;
            a.lease_expires_at = None;
        }
    }
```
- **Anchor C — periodic `LeaseHeartbeat` sender:** where the runner spawns its background tasks (near @658,
  the `spawn_hive_sync_service` / `command_tx` clone), spawn a task that on an interval STRICTLY SHORTER
  than the hive `LEASE_TTL` (204) sends `command_tx.send(NodeMessage::LeaseHeartbeat { assignment_ids })`
  where `assignment_ids` = the keys of `state.read().await.active_assignments`. Use the existing
  `tokio::time::interval` pattern (see hive_sync.rs:131). Record the chosen cadence + its relationship to
  204's TTL (TTL must exceed cadence with margin) in the ledger.

### 2. `crates/services/src/services/hive_client.rs`
- **Anchor D — `enum HiveEvent` (@661-688):** add two variants after `BackfillRequest` (@685):
```rust
    /// Lease granted/renewed by the hive (assignment's current fencing token + expiry).
    LeaseGranted { assignment_id: Uuid, fencing_token: i64, lease_expires_at: chrono::DateTime<Utc> },
    /// Lease revoked by the hive — the node must self-fence the assignment's agent.
    LeaseRevoked { assignment_id: Uuid, reason: String },
```
- **Anchor E — the 202 `HiveMessage::LeaseGrant`/`LeaseRevoked` STUB arms in `handle_hive_message`
  (@~1062, BEFORE the `_ =>` wildcard):** replace the two stub bodies (currently only `tracing::debug!`)
  with `event_tx` sends, mirroring `TaskAssign` (@979) / `BackfillRequest` (@1059):
  - **Before (the 202 stubs):**
```rust
            HiveMessage::LeaseGrant { assignment_id, fencing_token, lease_expires_at } => {
                tracing::debug!(%assignment_id, fencing_token, %lease_expires_at,
                    "received lease_grant (store TODO: task 206)");
            }
            HiveMessage::LeaseRevoked { assignment_id, reason } => {
                tracing::debug!(%assignment_id, %reason, "received lease_revoked (handle TODO: task 206)");
            }
```
  - **After:**
```rust
            HiveMessage::LeaseGrant { assignment_id, fencing_token, lease_expires_at } => {
                tracing::debug!(%assignment_id, fencing_token, "lease granted");
                let _ = self.event_tx
                    .send(HiveEvent::LeaseGranted { assignment_id, fencing_token, lease_expires_at })
                    .await;
            }
            HiveMessage::LeaseRevoked { assignment_id, reason } => {
                tracing::warn!(%assignment_id, %reason, "lease revoked by hive");
                let _ = self.event_tx
                    .send(HiveEvent::LeaseRevoked { assignment_id, reason })
                    .await;
            }
```

## Allowed moves
ONLY: add the two `fencing_token`/`lease_expires_at` fields to `ActiveAssignment` (+ set them `None` at the
insert site), add the two `HiveEvent` variants, wire the two `handle_hive_message` arms to `event_tx`, add
the two `process_event` arms + the `apply_lease_grant`/`apply_lease_revoke` helpers + the periodic
`LeaseHeartbeat` sender. Reuse `event_tx`, `command_tx`, `self.state`, existing interval/`tokio::spawn`
patterns. Do NOT touch the WS enum definitions (202 owns `NodeMessage`/`HiveMessage`), the outbox STAMP
(207), the agent-halt watchdog (208), the hive side, or any migration.

## STOP triggers
- Adding a SEPARATE `LeaseStore` keyed only by `assignment_id` → BUG-by-design: 207 needs the token by
  `local_task_id`; the lease MUST live on `ActiveAssignment` (which carries `local_task_id`) so the lookup
  is one map read. This reconciliation is the whole point (decisions-ledger).
- A `process_event` arm left as a stub (not mutating `active_assignments`) → the lease is silently dropped.
- `process_event` gained a `_` wildcard → STILL add named arms (208 extends the `LeaseRevoked` arm; do not
  let it fall into a wildcard).
- The `LeaseHeartbeat` cadence ≥ hive `LEASE_TTL` (204) → a healthy node's lease expires. Cadence MUST be
  shorter (e.g. TTL/2). Cross-check 204's const.
- A third `HiveEvent` match site exists (grep `HiveEvent::` across `crates/services/src
  crates/local-deployment/src`) beyond the two files → STOP: `files:` incomplete; record (ledger #5 says two).
- `ActiveAssignment` gains a non-`Option` field → BUG: a freshly-assigned (pre-grant) or node-owned
  assignment has no token; both fields MUST be `Option` and default `None`.

## Done when
`WAI_TYPECHECK_CMD="cargo check -p services" WAI_TEST_CMD="cargo test -p services lease_state" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-hive-redesign 206` exits 0
(node side is SQLite/in-memory — no Postgres precondition; the lease-state test is hermetic. `cargo check
-p services` is the Trap-1 typecheck override.)
