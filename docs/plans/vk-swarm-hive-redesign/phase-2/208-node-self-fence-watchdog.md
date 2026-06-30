---
id: "208"
phase: 2
title: Node self-fence watchdog — halt the agent on lease-revoke or renew-deadline miss
status: ready
depends_on: ["206"]
parallel: false
conflicts_with: ["206", "207"]
files:
  - crates/services/src/services/node_runner.rs
irreversible: false
scope_test: "crates/services/src/services/node_runner.rs"
allowed_change: edit
covers_criteria: [SC3]
covers_tests: []
---
## Failing test (write first)
Hermetic, ws-free, db-free unit test of the **decision** the watchdog makes — "which assignments are past
their renew deadline / revoked and must be fenced" — separated from the actual `stop_execution` side
effect (which needs a container + DB). In `node_runner.rs`'s `#[cfg(test)] mod` (beside 206's
`lease_state_tests`):
```rust
#[cfg(test)]
mod self_fence_tests {
    use super::*;

    #[tokio::test]
    async fn assignments_with_expired_or_missing_lease_are_selected_for_fencing() {
        let state = std::sync::Arc::new(tokio::sync::RwLock::new(NodeRunnerState::default()));
        let live = uuid::Uuid::new_v4();
        let expired = uuid::Uuid::new_v4();
        let revoked = uuid::Uuid::new_v4();
        {
            let mut s = state.write().await;
            let mk = |aid, expires| ActiveAssignment {
                assignment_id: aid, task_id: uuid::Uuid::new_v4(), local_task_id: Some(uuid::Uuid::new_v4()),
                local_attempt_id: Some(uuid::Uuid::new_v4()), status: TaskExecutionStatus::Running,
                fencing_token: Some(1), lease_expires_at: expires,
            };
            // live: lease in the future → NOT fenced.
            s.active_assignments.insert(live, mk(live, Some(chrono::Utc::now() + chrono::Duration::seconds(60))));
            // expired: lease in the past → fenced (renew-deadline miss).
            s.active_assignments.insert(expired, mk(expired, Some(chrono::Utc::now() - chrono::Duration::seconds(1))));
            // revoked: lease cleared by HiveEvent::LeaseRevoked (token+expiry None) but still Running → fenced.
            let mut r = mk(revoked, None); r.fencing_token = None;
            s.active_assignments.insert(revoked, r);
        }
        let to_fence = assignments_to_self_fence(&state, chrono::Utc::now()).await;
        assert!(to_fence.contains(&expired), "an expired lease self-fences (ADR-0009)");
        assert!(to_fence.contains(&revoked), "a revoked lease self-fences");
        assert!(!to_fence.contains(&live), "a live lease is not fenced");
    }
}
```
> Match `TaskExecutionStatus`'s real variant names (read them). The pure selector
> `assignments_to_self_fence(&state, now) -> Vec<Uuid>` is testable without a container; the watchdog loop
> calls it then invokes the halt path on each id. Record this split in the ledger.

## Change
This reuses the EXISTING agent-halt mechanism (ADR-0001 / ADR-0009 §4): `AssignmentHandler::handle_cancellation`
(`assignment_handler.rs:194`) already finds the assignment's running execution processes and calls
`container.stop_execution(.., Killed)` — the process-group kill. The self-fence is a watchdog that invokes
that SAME halt when a lease cannot be renewed within its TTL (or is revoked). It does NOT re-implement the kill.

- **File:** `crates/services/src/services/node_runner.rs`
- **Anchor A — the pure selector** (new free fn near 206's `apply_lease_*` helpers):
```rust
    /// Assignments whose hive lease has lapsed (expired or revoked) while still Running — the node must
    /// self-fence their agents (ADR-0009 §4: bounded overlap). A revoked lease has `lease_expires_at = None`
    /// (cleared by HiveEvent::LeaseRevoked, task 206) but a still-Running status; an expired lease has
    /// `lease_expires_at < now`. A live lease (future expiry) is left alone.
    async fn assignments_to_self_fence(
        state: &std::sync::Arc<tokio::sync::RwLock<NodeRunnerState>>,
        now: chrono::DateTime<Utc>,
    ) -> Vec<Uuid> {
        let s = state.read().await;
        s.active_assignments.values()
            .filter(|a| matches!(a.status, TaskExecutionStatus::Running)) // only halt live execution
            .filter(|a| match a.lease_expires_at {
                Some(exp) => exp < now,   // renew-deadline miss
                None => a.fencing_token.is_none(), // revoked (lease cleared) — distinguishes from
                                                   // never-yet-granted? see STOP triggers
            })
            .map(|a| a.assignment_id)
            .collect()
    }
```
  > **Never-granted vs revoked ambiguity (STOP trigger below):** a freshly-assigned assignment that has not
  > YET received its first `LeaseGrant` also has `lease_expires_at = None`. Do NOT fence it (it is starting,
  > not lapsed). The watchdog MUST only consider assignments that were once granted then lost their lease.
  > Resolve by gating the `None` arm on a "was previously leased" signal — e.g. add a `lease_revoked: bool`
  > flag set by 206's `apply_lease_revoke`, OR only fence on `Some(exp) where exp < now` (expiry) and let
  > the explicit `HiveEvent::LeaseRevoked` arm (206) trigger an immediate fence directly (preferred — see
  > Anchor C). Pick ONE and record it; the simplest is: watchdog fences on EXPIRY only, and the
  > `LeaseRevoked` event arm fences immediately. Then the `None`/`fencing_token.is_none()` leg above is
  > unnecessary — drop it and fence only `Some(exp) < now`.
- **Anchor B — the watchdog loop:** where the runner spawns background tasks (near @658, beside 206's
  `LeaseHeartbeat` sender), spawn a `tokio::time::interval` loop (cadence ≤ the lease TTL granularity, e.g.
  every few seconds) that calls `assignments_to_self_fence(&state, Utc::now())` and, for each id, invokes
  the halt. The halt needs the `AssignmentHandler` (which owns `container` + `db`). The handler is built at
  @662-663 (`Option<AssignmentHandler<C>>`); clone what `handle_cancellation` needs, OR call
  `handler.handle_cancellation(assignment_id)` directly (it sets status to `Cancelled` and stops processes —
  acceptable as the fence effect; a dedicated `self_fence(assignment_id)` that sets a Failed/Fenced status
  is nicer but reuses the same stop logic). Reuse `handle_cancellation` unless a distinct status is required;
  record the choice.
- **Anchor C — immediate fence on revoke:** in 206's `HiveEvent::LeaseRevoked` arm (which 206 added to
  `process_event`), ADD the immediate halt: after clearing the lease, invoke the same halt path for that
  assignment id (the partitioned node learns its lease is gone and stops NOW, not at the next watchdog tick).
  This is the one line 208 adds to the 206-authored arm (hence conflicts_with: 206 on this file).

## Allowed moves
ONLY: add `assignments_to_self_fence`, spawn the watchdog loop, invoke the existing halt
(`AssignmentHandler::handle_cancellation` or a thin `self_fence` wrapper around the same stop logic) on
selected/revoked assignments, and the immediate-fence line in the `LeaseRevoked` arm, plus the
`self_fence_tests` module. Reuse `AssignmentHandler`, `self.state`/`active_assignments`, the existing
`stop_execution` path. Do NOT re-implement the process kill, do NOT add the lease fields (206 owns
`ActiveAssignment`), do NOT touch the WS protocol (202), the outbox stamp (207), the hive side, or any
migration.

## STOP triggers
- Fencing a freshly-assigned assignment that has not yet received its first `LeaseGrant`
  (`lease_expires_at = None` because it is STARTING, not lapsed) → BUG: that kills a healthy starting agent.
  Resolve per Anchor A's note: fence on EXPIRY (`Some(exp) < now`) via the watchdog and on the explicit
  `LeaseRevoked` event via Anchor C — do NOT fence a bare `None` expiry. Record the chosen rule.
- Re-implementing the process-group kill instead of reusing `handle_cancellation`/`stop_execution` → STOP:
  ADR-0009 §4 says reuse the ADR-0001 mechanism; the kill already exists at assignment_handler.rs:215-227.
- The watchdog fences a non-Running assignment (already done/cancelled) → BUG: only `Running` assignments
  have a live agent to halt. The selector filters on `Running`.
- `AssignmentHandler` is `None` (container-less node, node_runner.rs:640 "logged but not executed") → there
  is no agent to halt; the watchdog must no-op safely. Guard on the `Option`.
- `ActiveAssignment.lease_expires_at`/`fencing_token` absent → 206 must be `passed` first (depends_on: 206).

## Done when
`WAI_TYPECHECK_CMD="cargo check -p services" WAI_TEST_CMD="cargo test -p services self_fence" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-hive-redesign 208` exits 0
(node side — hermetic in-memory selector test; no Postgres precondition. `cargo check -p services` is the
Trap-1 typecheck override.)
