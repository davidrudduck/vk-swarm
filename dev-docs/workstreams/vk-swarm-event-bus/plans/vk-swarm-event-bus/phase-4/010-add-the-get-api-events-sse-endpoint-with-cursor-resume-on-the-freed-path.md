---
id: "010"
phase: 4
title: "Add the GET /api/events SSE endpoint with cursor resume on the freed path"
status: passed
depends_on: ["001","005","014"]
parallel: false
conflicts_with: ["001"]
files:
  - "crates/server/src/routes/events.rs"
  - "crates/server/src/routes/mod.rs"
  - "crates/server/tests/events.rs"
  - "crates/server/tests/common/mod.rs"
irreversible: false
scope_test: "crates/server"
allowed_change: mixed
covers_criteria: ["SC4"]
covers_tests: ["TS5"]
---
## Failing test (write first)
**File:** `crates/server/tests/events.rs` — a NEW integration test file, declared in `files:`
because TS5 requires route tests and the file-set gate rejects writing to an undeclared path. Reuse
the existing harness in `crates/server/tests/common/mod.rs` and follow the shape of the neighbouring
`*_routes.rs` suites. These ARE TS5.

1. `events_without_cursor_streams_live_only` — subscribe with no `cursor`; assert pre-existing
   journal rows are NOT replayed and a subsequently emitted event IS received.
2. `events_with_cursor_replays_then_goes_live` — journal 5 events, subscribe with `cursor=2`, assert
   seqs 3,4,5 arrive and then a live 6th arrives on the same connection.
3. `each_sse_message_carries_seq` — assert every frame exposes its seq so a client can resume. A
   stream that omits seq makes SC4 unimplementable client-side.
4. `reconnect_with_last_seen_cursor_skips_nothing` — the SC4 scenario end to end: subscribe,
   disconnect, emit N events while disconnected, resubscribe with the last-seen cursor, assert every
   journaled event above the cursor arrives (duplicates tolerated, none skipped).
5. `removed_record_patch_route_is_gone` — the TS5 guard: assert no route serves the old record-patch
   payload shape and that `stream_events` no longer exists.


## Change
**File:** `crates/server/src/routes/events.rs`
**Anchor:** new file — task 001 deleted the previous occupant, so this is a clean create.
**Sibling to read FIRST — and note the correction.** The original breakdown pointed at
`crates/server/src/routes/logs.rs` and a symbol `stream_raw_stream`. Neither is a usable SSE
precedent: the real symbol is `stream_raw_logs` (`crates/services/src/services/container.rs:819-868`,
with `stream_normalized_logs` at `:870-883`), and both are generic stream SOURCES, not SSE handlers;
`routes/logs.rs` serves REST and WebSocket (`WsKeepAlive`), not axum `Sse`. It can teach you nothing
about `Sse`/`KeepAlive` framing or SSE error mapping.

The correct precedent is the route task 001 deleted — it was a real `Sse` + `KeepAlive::default()`
handler. Read it from git rather than from the working tree:
`git show $(git rev-parse HEAD~1):crates/server/src/routes/events.rs` (or any commit before task
001 landed). Cite `stream_raw_logs` / `stream_normalized_logs` only for boxed-stream construction.
Justify any divergence in the ledger.

Note also that `EventBus::subscribe_from` returns a Result-of-stream-of-Results (task 005): map a
setup error to an HTTP error response, and a mid-stream error to a terminal SSE error frame rather
than a silent close.
**After:** `GET /api/events?cursor=N`:
- parse an OPTIONAL `cursor` query param,
- absent cursor ⇒ live-only from now (NOT `cursor=0`, which would replay the whole journal — the
  spec is explicit that these differ),
- present cursor ⇒ `EventBus::subscribe_from(cursor)`, which replays then goes live,
- map each `SequencedEvent` to an SSE `Event` whose `id` (or an explicit field) carries `seq`,
- `KeepAlive::default()`, matching the existing SSE routes.

**File:** `crates/server/src/routes/mod.rs`
**Anchor:** the module list and the `base_routes` builder chain in `pub async fn router` — task 001
removed both lines from here.
**After:** re-add `pub mod events;` and `.merge(events::router(&deployment))` in their original
alphabetical/chain positions, now pointing at the bus route.


## Allowed moves
ONLY the new route file, the new test file, and the two re-added lines in
routes/mod.rs. Do NOT add authentication or filtering beyond `cursor` — event-type filtering is not
in this spec. Do NOT re-introduce anything resembling the deleted record-patch stream.


## STOP triggers
- `crates/server/src/routes/events.rs` still exists when this task starts — task 001 did not run or
  did not complete; STOP.
- `crates/server/tests/common/mod.rs` does not exist or exposes no reusable harness — record the
  actual structure you used instead of silently downgrading TS5 to manual verification.
- Distinguishing "no cursor" from `cursor=0` is awkward in the extractor — it MUST be distinguished;
  STOP rather than collapsing them.


## Manual verification (record in decisions-ledger)
Gate invocation (the Done-when placeholders): this is a Rust crate, so the runner MUST be overridden — the auto-detected runner would try vitest. Use WAI_TYPECHECK_CMD="cargo check --workspace" with the WAI_TEST_CMD given below.
WAI_TEST_CMD="cargo test -p server --test events"

Live SC4 check (record the transcript in the ledger), against a running node:
1. `curl -N http://<node>/api/events` — receives live events only; note the highest seq seen.
2. Disconnect. Create/move several tasks.
3. `curl -N "http://<node>/api/events?cursor=<last-seen-seq>"` — every event created while
   disconnected arrives, in ascending seq order, none missing.


## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-event-bus 010` exits 0

## REQUIRED — STOP resolution (2026-08-17): how the handler reaches the bus

impl-010 (attempt 1) hit the dictated STOP: no `EventBus` is reachable from deployment state
(verified: `LocalDeployment` has no `event_bus` field/accessor; the `Deployment` trait has no
`event_bus()` method; `EventBus` is not imported by `local-deployment`). Resolution — this task
now `depends_on` 014, which wires the bus into startup and exposes it as an INHERENT accessor
`pub fn event_bus(&self) -> Arc<services::services::event_bus::EventBus>` on `LocalDeployment`.
No `Deployment`-trait change is needed or permitted: `crates/server/src/lib.rs:12` fixes
`pub type DeploymentImpl = local_deployment::LocalDeployment;`, so handlers see the concrete
type. The route handler obtains the bus as `deployment.event_bus()` from
`State(deployment): State<DeploymentImpl>`. Do not add trait methods; do not construct a bus in
the route.

## REQUIRED — added 2026-08-17 (orchestrator): real-write HTTP-seam test (reachability gate b)

Task 014 deferred the run-level reachability-gate (b) test here (its ledger names
`crates/server/tests/events.rs`). Therefore, IN ADDITION to tests 1-5:

6. `sse_delivers_an_event_from_a_real_task_write` — drive the REAL production write path, not
   `event_journal::append`: create a project and a task through the model functions the
   production routes call (`Project::create` / `Task::create` — the same write sites task 006
   instrumented), and assert the resulting `task_created` event arrives on a `GET /api/events`
   subscription taken BEFORE the write. This is the full-path proof: model write → journal →
   tailer → bus → SSE frame. If the test harness cannot construct the prerequisites for a task
   write, STOP and report what is missing — do not fall back to `event_journal::append` for
   this test (tests 1-5 may journal directly; this one exists precisely to avoid that seam).

## REQUIRED — added 2026-08-17 (orchestrator): harness file-set amendment

The shared harness `crates/server/tests/common/mod.rs` exposed no way for a test to reach the
listener address or the deployment (needed for a raw SSE client and bus/journal access), which
is the inadequacy the STOP trigger anticipated. `files:` now includes it, LIMITED to additive
accessor methods (e.g. `addr()`, `deployment()`) and attribute adjustments they force — no
behavioural change to the harness, no edits to existing method bodies.

## REQUIRED — added 2026-08-17 (orchestrator, after panel attempt-3 re-review)

### Gate-command correction (panel N1)
The Manual-verification/Done-when `WAI_TEST_CMD` previously read `cargo test -p server events`
— a test-NAME filter matching only 2 of the 6 tests (T3-T6 filtered out, including the SC4 and
reachability-gate tests). Corrected above to `cargo test -p server --test events` (target
selector, all tests). Task-file defect, this repo's; not an agent-plugins issue.

### Test 7 — pin the mid-stream error terminal frame (panel A §6, option a)
7. `mid_stream_error_emits_terminal_error_frame_then_ends` — pin the R1 dictate with fault
   injection via the run's established table-rename technique (tailer.rs:581 precedent):
   journal several events; then `ALTER TABLE event_journal RENAME TO event_journal_poisoned`
   via `h.deployment().db().pool`; connect `GET /api/events?cursor=0` (the replay path must
   read the journal and now errors); assert the client receives exactly one `event: error`
   SSE frame and the stream then ENDS (bounded read observes EOF, no further frames — a
   keep-alive-only hang is a failure). Restore the table name afterwards if the harness
   requires it for teardown (record either way). Red proof: revert the unfold Done transition
   (yield the error frame but keep the stream alive) → this test must FAIL on the
   stream-end assertion.
