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
flow change well beyond this task.

**Corrected anchors (amended 2026-08-12 — the previous ones were wrong).** This task previously
anchored at `L353`/`L375` and asserted those arms have a `DBService` in scope. They do NOT:
`:352-379` sits inside `NodeRunnerHandle::process_event` (`:349`), a method on a struct whose only
fields are `event_rx`, `command_tx`, `state`, `_join_handle` (`:334-343`) — no pool, no `DBService`.
That is the SAME defect this task diagnoses about `hive_client.rs`, reproduced one layer up; the
tournament moved the anchor off `hive_client.rs` but landed on the wrong function.

The loop that genuinely holds the database is in `spawn_node_runner` (`:697-701` takes
`db: DBService`; the loop runs `:804-1175`) and receives the same `HiveEvent` values.

**Anchor 1:** the `Some(HiveEvent::Connected { … })` arm at `:806`.
**Anchor 2:** a NEW `Some(HiveEvent::Disconnected { reason })` arm that you must ADD — the loop has
no such arm today (its arms are Connected `:806`, TaskAssigned `:863`, TaskCancelled `:882`,
TaskSyncResponse `:899`, LabelSync `:957`, BackfillRequest `:998`, OpAck `:1064`, LeaseRevoked
`:1069`, DigestResult `:1085`), so `Disconnected` currently falls through to `Some(_)` at `:1166`
and is silently ignored. Adding that arm is REQUIRED and is authorised in Allowed moves below.
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
reconcile. If a substep failed and was logged-and-continued, still emit, because the arm completed —
record that "completed" means "the arm ran to its end", not "every substep succeeded", in the ledger.

**Sourcing `entity_count` is DICTATED (amended 2026-08-12).** The earlier wording — "the count
returned by the project-sync step" — named something that does not exist: `sync_remote_projects`
(`crates/services/src/services/node_runner.rs:1213-1218`) returns `Result<(), NodeRunnerError>`. It
already COMPUTES the count for its own tracing field (`:1250` `project_count = remote_projects.len()`)
and then throws it away. Do exactly this, and nothing more:

1. Change its signature to `Result<usize, NodeRunnerError>` and its tail `Ok(())` (`:1254`) to
   `Ok(remote_projects.len())`. Leave the `tracing::info!` at `:1249-1252` as it is.
2. Both existing call sites match on `if let Err(e) = …` (`:818-820` and `:1159-1161`), a pattern that
   is unaffected by the `Ok` type, so **the digest-heal caller at `:1159` needs no edit at all** — do
   not touch it (this task must not anchor anything at the heal path).
3. At the Connected-arm call site (`:817-822`) capture the value instead of discarding it. All THREE
   branches are dictated — do not choose:
   - `remote_client` is `Some` and the call returns `Ok(n)` → `n`.
   - `remote_client` is `Some` and the call returns `Err(e)` → log the existing warning, use `0`.
   - `remote_client` is `None` (`:701` types it `Option<RemoteClient>`, so the sync never runs) → use
     `0` and STILL emit `ReconcileCompleted`, because the arm ran to its end. Do not skip the event.
   Convert at the emission site with `n as i64`, matching this file's own idiom at `:1118`
   (`let batch_size = rows.len() as i64;`) and the 15 other `.len() as i64` sites in `crates/`
   (`i64::try_from` has zero). This is lossless, not a truncating cast: `remote_projects` is a `Vec`
   (`:1229-1233`) and `Vec::len() <= isize::MAX == i64::MAX` on 64-bit, while `usize::MAX < i64::MAX`
   on 32-bit. Do NOT use `i64::try_from(n).unwrap_or(i64::MAX)` — the fallback is unreachable, and
   reporting a 9-quintillion count is the same meaningful-looking-false-value class that
   `unwrap_or(0)` is forbidden for in task 007. (Amended 2026-08-12: an earlier draft of this step
   dictated exactly that `try_from` form on a false premise.)
   `entity_count` means REMOTE PROJECTS RECONCILED — not tasks, not labels, and not the count that
   `sync_owned_project_ids_from_hive` (`:825-826`) already returns. Do not aggregate the other steps.
4. Record in the ledger that a failed project-sync yields `entity_count = 0` on an event that still
   reports the reconcile as completed — that is the deliberate reading of "completed" above, and a
   consumer must not read `0` as "the sync succeeded and found nothing". Also file a backlog row via
   `/wai:finding-new` recording that `reconcile_completed` cannot distinguish "sync failed" from
   "synced, found nothing": a ledger note alone would be a silent deferral under CLAUDE.md's
   no-deferred-remediation rule. Fixing it properly would mean amending task 003's enum only — the
   frozen spec never names `entity_count` (spec `:83` says "as applicable") — but that is OUT of scope
   here, because the event has no live consumer yet.

Do NOT anchor `ReconcileCompleted` at the digest-heal pull near L1150. That is a SEPARATE match arm
with a different completion point, and treating the two as one "reconcile leg" is what made the
original anchor ambiguous. If the spec's reconcile coverage is judged to need the heal path too,
that is a spec question — escalate rather than emitting the same variant from two unrelated sites.

## Allowed moves
ONLY: the journal-append additions; the `was_connected` transition-gate
bookkeeping; the `sync_remote_projects` return-type change; the addition of a
`Some(HiveEvent::Disconnected { reason })` arm to the loop's match; and the `:817-822` call-site
restructure needed to capture the count, exactly as dictated in step 3 (all inside `node_runner.rs`). Do NOT alter existing `HiveEvent` mpsc sends, reconnect/backoff logic, or the
reconcile algorithm. Do NOT touch `hive_client.rs` — it holds no database handle and restructuring it
is out of scope. Do NOT touch `hive_sync.rs`. Nothing broadcasts here.

## STOP triggers
- Connect/disconnect can fire repeatedly during backoff retries, producing an event storm — emit only
  on genuine transitions using the local `was_connected: bool` dictated in the Change section, and
  record it. Do NOT reach for `hive_client.rs`'s `ConnectionState` (`hive_client.rs:761`) — it is a
  private struct in a file Allowed moves puts out of bounds. (Amended 2026-08-12: this trigger
  previously named `ConnectionState`, contradicting both Allowed moves and the Change section.)
- The reconcile leg has no single completion point — STOP rather than emitting at several places.
- `sync_remote_projects` does not compute a project count at all, or its signature/call sites differ
  from the line numbers cited above — STOP with the actual code, do NOT substitute a different
  quantity for `entity_count`. (Amended 2026-08-12: this trigger previously read "`entity_count` is
  not available at reconcile completion — record what is used instead", which invited the implementer
  to pick a substitute quantity silently. The sourcing is now dictated in the Change section.)

## Manual verification (record in decisions-ledger)
Gate invocation (the Done-when placeholders): this is a Rust crate, so the runner MUST be overridden — the auto-detected runner would try vitest. Use WAI_TYPECHECK_CMD="cargo check --workspace" with the WAI_TEST_CMD given below.
WAI_TEST_CMD="cargo test -p services --test event_emission"

Live SC3 check (record in the ledger): on a running node with the hive reachable, kill the hive link,
restore it, then
`sqlite3 $VK_DATABASE_PATH "select seq, event_type from event_journal where event_type like 'hive_%' or event_type='reconcile_completed' order by seq"`
shows `hive_disconnected` → `hive_connected` → `reconcile_completed` in ascending seq order.

## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-event-bus 008` exits 0
