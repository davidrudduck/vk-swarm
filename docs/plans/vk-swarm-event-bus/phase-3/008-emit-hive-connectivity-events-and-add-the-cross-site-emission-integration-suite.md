---
id: "008"
phase: 3
title: "Emit hive connectivity events and add the cross-site emission integration suite"
status: ready
depends_on: ["007"]
parallel: false
conflicts_with: []
files:
  - "crates/services/src/services/hive_client.rs"
  - "crates/services/src/services/node_runner.rs"
  - "crates/services/tests/event_emission.rs"
irreversible: false
scope_test: "crates/services"
allowed_change: mixed
covers_criteria: ["SC3"]
covers_tests: ["TS3"]
---
## Failing test (write first)
**File:** `crates/services/tests/event_emission.rs` (new integration suite — this IS TS3, and it
spans all three emission sites, which is why it lives in one place rather than being smuggled into a
single site's unit tests).

Connectivity (SC3):
1. `disconnect_emits_hive_disconnected_with_reason`
2. `reconnect_emits_hive_connected`
3. `reconcile_completion_emits_reconcile_completed`
4. `connectivity_events_are_ordered` — kill then restore the link; assert the journal shows
   `hive_disconnected` → `hive_connected` → `reconcile_completed` with strictly increasing seq. SC3
   requires the ORDER, not merely the presence.

Cross-site (TS3), asserting exactly-one-event-per-state-change across all three choke points:
5. `task_crud_emits_exactly_one_event_each`
6. `attempt_lifecycle_emits_exactly_one_event_each`
7. `connectivity_transitions_emit_exactly_one_event_each`
8. `no_duplicate_events_for_a_single_state_change` — the regression guard for double-emission if a
   site is ever instrumented at two layers.


## Change
Connectivity events have NO accompanying state write, so per spec D2 there is no transaction to
share — the journal row IS the record and is appended directly, then broadcast.

**File:** `crates/services/src/services/hive_client.rs`
**Anchor:** the `HiveEvent::Disconnected` send at L819 and the `HiveEvent::Connected` send at L907
(both inside the connection loop; `enum HiveEvent` is at L714, `struct ConnectionState` at L761).
**After:** at each site, alongside the existing `event_tx.send(HiveEvent::…)`, append
`NodeEvent::HiveDisconnected { reason }` / `NodeEvent::HiveConnected {}` to the journal and broadcast.
Do NOT replace the existing `HiveEvent` mpsc send — it drives existing behaviour and stays.

**File:** `crates/services/src/services/node_runner.rs`
**Anchor:** the bulk-snapshot reconcile leg — the completion point of the reconcile described at
L812 ("ADR-0007 SINGLE LIVE INBOUND CHANNEL: the bulk-snapshot reconcile runs ONLY here") and the
hive-has/node-lacks pull at L1150.
**After:** on reconcile COMPLETION (not start), append `NodeEvent::ReconcileCompleted { entity_count }`
and broadcast. Emit once per completed reconcile.

Note the spec's original anchor named `HiveSyncService` (`hive_sync.rs`); that was corrected in the
2026-08-11 amendment because that service has only `sync_once` (L167) and `sync_local_projects`
(L697) and no connectivity transitions at all.


## Allowed moves
ONLY the journal-append + broadcast additions at the three named sites, plus the new
integration test file. Do NOT alter existing `HiveEvent` mpsc sends, reconnect/backoff logic, or the
reconcile algorithm. Do NOT touch `hive_sync.rs`.


## STOP triggers
- Connect/disconnect can fire repeatedly during backoff retries, producing an event storm — STOP and
  decide (emit only on genuine state TRANSITIONS, tracked via `ConnectionState`) and record it.
- The reconcile leg has no single completion point — STOP rather than emitting at several places.
- `entity_count` is not available at reconcile completion — record what is used instead.


## Manual verification (record in decisions-ledger)
Gate invocation (the Done-when placeholders): this is a Rust crate, so the runner MUST be overridden — the auto-detected runner would try vitest. Use WAI_TYPECHECK_CMD="cargo check --workspace" with the WAI_TEST_CMD given below.
WAI_TEST_CMD="cargo test -p services --test event_emission"

Live SC3 check (record in the ledger): on a running node with the hive reachable, kill the hive link,
restore it, then
`sqlite3 $VK_DATABASE_PATH "select seq, event_type from event_journal where event_type like 'hive_%' or event_type='reconcile_completed' order by seq"`
shows `hive_disconnected` → `hive_connected` → `reconcile_completed` in ascending seq order.


## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-event-bus 008` exits 0
