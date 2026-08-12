---
id: "008"
phase: 3
title: "Emit hive connectivity events from the node_runner event loop"
status: ready
depends_on: ["007"]
parallel: false
conflicts_with: []
files:
  - "crates/services/src/services/node_runner.rs"
irreversible: false
scope_test: "crates/services"
allowed_change: edit
covers_criteria: ["SC3"]
covers_tests: []
---
## Failing test (write first)
**File:** `crates/services/src/services/node_runner.rs` colocated tests (the cross-site
suite is task 015; this task proves SC3 only).

1. `disconnect_emits_hive_disconnected_with_reason`
2. `reconnect_emits_hive_connected`
3. `reconcile_completion_emits_reconcile_completed_with_entity_count`
4. `connectivity_events_are_ordered` — kill then restore the link; assert the journal shows
   `hive_disconnected` → `hive_connected` → `reconcile_completed` with strictly increasing seq. SC3
   requires the ORDER, not merely the presence.
5. `repeated_failed_connection_attempts_emit_one_disconnect` — drive three consecutive failed
   connection attempts from an already-disconnected state; assert exactly ONE `hive_disconnected`
   row. This is the test that fails if the transition gate is missing.
6. `clean_close_emits_disconnected` — drive a clean close (no error); assert a `hive_disconnected`
   row IS produced. Without the gate this case emits nothing at all.


## Change
Connectivity events have NO accompanying state write, so per spec D2 there is no transaction to
share — the journal row IS the record and is appended directly. Nothing here broadcasts; the tailer
publishes (task 013).

**Anchor the emission in `node_runner.rs`, NOT in `hive_client.rs`.** The original breakdown pointed
at the `HiveEvent` send sites inside `HiveClient` — that is unbuildable. `HiveClient` is
`{ config, state, event_tx, command_tx }` (`crates/services/src/services/hive_client.rs:767-772`) and
its constructor takes only a `HiveClientConfig` (`:783-801`): no pool, no `DBService`, no journal
access. Threading a database handle through `HiveClient::new` and its spawn chain would be a control-
flow change well beyond this task. The `node_runner` event loop already receives the very same
`HiveEvent` values and DOES have a `DBService` in scope — it handles `HiveEvent::Connected` at L353
and `HiveEvent::Disconnected` at L375.

**Anchor:** the `HiveEvent::Connected` arm at L353 and the `HiveEvent::Disconnected` arm at L375.
**After:** in each arm, append `NodeEvent::HiveConnected {}` / `NodeEvent::HiveDisconnected { reason }`
to the journal, gated on an ACTUAL TRANSITION — see below. Leave the existing handling untouched.

**The transition gate is mandatory, and the reason is a real control-flow defect upstream.** Read
`hive_client.rs:808-824` before writing this. The connection loop emits `HiveEvent::Disconnected`
ONLY on the `Err(e)` arm; the `Ok(())` clean-close arm just logs "hive connection closed cleanly" and
emits nothing. `state.connected = false` is then set after BOTH arms with no `was_connected` check.
The consequences, both of which break SC3's "exactly one event per transition":

- A clean close produces NO disconnect event at all.
- Every failed initial connection and every failed retry produces ANOTHER disconnect event, even
  though the node was already disconnected.

Since this task may not restructure `HiveClient`, hold the gate in `node_runner`: keep a local
`was_connected: bool` alongside the loop, journal `hive_disconnected` only on a true→false edge and
`hive_connected` only on a false→true edge, and derive the clean-close case from the `Connected`
event ceasing rather than from a `Disconnected` event that never arrives. If a clean close cannot be
distinguished from an idle connection at this layer, STOP and escalate — do not paper over it by
emitting on every loop iteration.

**Anchor:** the reconcile completion — the END of the `Connected` arm's reconcile sequence
(L806-862), after all sync steps have run.
**After:** append `NodeEvent::ReconcileCompleted { entity_count }` exactly once per completed
reconcile. Define `entity_count` concretely as the count returned by the project-sync step; if a
substep failed and was logged-and-continued, still emit, because the arm completed — record that
"completed" means "the arm ran to its end", not "every substep succeeded", in the ledger.

Do NOT anchor `ReconcileCompleted` at the digest-heal pull near L1150. That is a SEPARATE match arm
with a different completion point, and treating the two as one "reconcile leg" is what made the
original anchor ambiguous. If the spec's reconcile coverage is judged to need the heal path too,
that is a spec question — escalate rather than emitting the same variant from two unrelated sites.


## Allowed moves
ONLY the journal-append additions and the transition-gate bookkeeping inside
`node_runner.rs`. Do NOT alter existing `HiveEvent` mpsc sends, reconnect/backoff logic, or the
reconcile algorithm. Do NOT touch `hive_client.rs` — it holds no database handle and restructuring it
is out of scope. Do NOT touch `hive_sync.rs`. Nothing broadcasts here.


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
