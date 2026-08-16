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
  - "crates/services/src/services/hive_client.rs"
irreversible: false
scope_test: "crates/services"
allowed_change: edit
covers_criteria: ["SC3"]
covers_tests: []
---
## Failing test (write first)
**File:** `crates/services/src/services/node_runner.rs` colocated tests (the cross-site
suite is task 015; this task proves SC3 only).

**Harness (DICTATED — amended 2026-08-16).** The event loop is inline in `spawn_node_runner` and
cannot be driven from a test without standing up the whole runner (hive connection, sync service,
heartbeat). Do NOT attempt that, and do NOT invent a mock WS harness. The Change section factors the
transition gate into a colocated `ConnectivityJournal` struct; the tests construct it directly,
obtain a pool via `db::test_utils::create_test_pool()` (never hand-written `CREATE TABLE`), and
drive its methods. All six tests go in a new `#[cfg(test)] mod connectivity_event_tests`
(mirroring task 007's `lifecycle_event_tests`). Assert journal rows by filtering `event_journal`
on `event_type` (`'hive_connected'` / `'hive_disconnected'` / `'reconcile_completed'`) — NEVER
`rows.is_empty()`-style assertions.

1. `disconnect_emits_hive_disconnected_with_reason` — `on_connected` then
   `on_disconnected(pool, "connection reset")`; assert exactly one `hive_disconnected` row whose
   payload contains the reason string.
2. `reconnect_emits_hive_connected` — `on_connected`, `on_disconnected`, `on_connected`; assert
   exactly two `hive_connected` rows (boot false→true edge AND the reconnect edge both count).
3. `reconcile_completion_emits_reconcile_completed_with_entity_count` — `on_reconcile_completed(pool, 3)`;
   assert one `reconcile_completed` row whose payload carries `entity_count` 3.
4. `connectivity_events_are_ordered` — drive `on_connected`, `on_disconnected`, `on_connected`,
   `on_reconcile_completed`; assert the journal shows `hive_disconnected` → `hive_connected` →
   `reconcile_completed` with strictly increasing seq (ignore rows before the disconnect). SC3
   requires the ORDER, not merely the presence.
5. `repeated_failed_connection_attempts_emit_one_disconnect` — `on_connected`, then
   `on_disconnected` THREE times (the link died, then two failed retries each surface another
   `Disconnected` event); assert exactly ONE `hive_disconnected` row. This is the test that fails
   if the transition gate is missing.
6. `clean_close_emits_disconnected` — `on_connected`, then
   `on_disconnected(pool, "connection closed cleanly")`; assert exactly one `hive_disconnected`
   row. Upstream, the clean-close `Ok(())` arm today sends NO event at all — the one-line
   `hive_client.rs` addition in the Change section is what makes this event exist; this test pins
   the gate's handling of it.

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

Hold the gate in `node_runner`, and close the upstream clean-close hole with ONE dictated line
(amended 2026-08-16 — the previous instruction to "derive the clean-close case from the `Connected`
event ceasing" was unimplementable: the `Ok(())` arm sends nothing, and at this layer the absence
of events is indistinguishable from an idle connection; the old STOP trigger for this is resolved).

**`hive_client.rs` — exactly one addition, nothing else in that file.** In the `Ok(())` clean-close
arm of the connection loop (`hive_client.rs:810-814`), after the
`tracing::info!("hive connection closed cleanly")` line, send the same event the `Err` arm already
sends (`:817-822`):

```rust
let _ = self
    .event_tx
    .send(HiveEvent::Disconnected {
        reason: "connection closed cleanly".to_string(),
    })
    .await;
```

Safe by inspection (verified 2026-08-16): the ONLY consumer of `HiveEvent::Disconnected` is
`process_event` (`node_runner.rs:375`), which idempotently sets `state.connected = false` and logs.
Today a clean close leaves that shared state stale-true, so this send FIXES a latent state bug as
well as making test 6's event exist. The transition gate below absorbs repeat sends. This one line
has no colocated test (driving it needs a real WS session) — it is proven at the seam by task 015
and live by task 012's SC3 check; record exactly that in the ledger rather than inventing a mock.

**The gate is a colocated helper the loop arms delegate to (DICTATED).** Add to `node_runner.rs`:

```rust
struct ConnectivityJournal {
    was_connected: bool,
}
```

with private async methods `on_connected(&mut self, pool: &SqlitePool)`,
`on_disconnected(&mut self, pool: &SqlitePool, reason: &str)`, and
`on_reconcile_completed(&self, pool: &SqlitePool, entity_count: i64)`, each appending via
`db::models::event_journal::append(pool, &event)`:

- `hive_connected` ONLY on a false→true edge; `hive_disconnected` ONLY on a true→false edge;
  `reconcile_completed` unconditionally (it is an occurrence, not a level).
- The edge bookkeeping updates `was_connected` from the EVENT, never from journal success: if the
  append errors, still flip the flag — the gate tracks real connectivity, and tying it to journal
  success would re-emit on every subsequent event. Journal-append errors are logged at `error!`
  with the event type and NOT propagated: connectivity handling must not die because a journal
  write failed, and there is no accompanying state write to roll back. Record this
  log-and-continue choice in the ledger.
- Methods return `()`; errors are handled inside.

The loop declares `let mut connectivity = ConnectivityJournal { was_connected: false };`
immediately before the `loop` at `:804` and the arms call the methods — the arms themselves stay
one-line delegations, which is what keeps the gate unit-testable without the loop.

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
ONLY: the `ConnectivityJournal` struct + methods and the journal-append calls inside them; the
`connectivity_event_tests` module; the `sync_remote_projects` return-type change; the addition of a
`Some(HiveEvent::Disconnected { reason })` arm to the loop's match; the `:817-822` call-site
restructure needed to capture the count, exactly as dictated in step 3 (all inside
`node_runner.rs`); and in `hive_client.rs` EXACTLY the one clean-close `event_tx.send` addition
dictated in the Change section — nothing else in that file. Do NOT alter existing `HiveEvent` mpsc
sends, reconnect/backoff logic, or the reconcile algorithm. Do NOT touch `hive_sync.rs`. Nothing
broadcasts here.

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
WAI_TEST_CMD="cargo test -p services --lib"
(Amended 2026-08-16: the previous value `cargo test -p services --test event_emission` named a test
target that does not exist — `crates/services/tests/` has no `event_emission.rs`; this task's tests
are colocated in `node_runner.rs` and run under `--lib`. Full `--lib` rather than a filter so the
existing colocated `node_runner` test modules also gate the `sync_remote_projects` signature change.)

Live SC3 check (record in the ledger): on a running node with the hive reachable, kill the hive link,
restore it, then
`sqlite3 $VK_DATABASE_PATH "select seq, event_type from event_journal where event_type like 'hive_%' or event_type='reconcile_completed' order by seq"`
shows `hive_disconnected` → `hive_connected` → `reconcile_completed` in ascending seq order.

## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-event-bus 008` exits 0
