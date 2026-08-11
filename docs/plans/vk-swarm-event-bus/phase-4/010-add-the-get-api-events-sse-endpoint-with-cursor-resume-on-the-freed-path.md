---
id: "010"
phase: 4
title: "Add the GET /api/events SSE endpoint with cursor resume on the freed path"
status: ready
depends_on: ["001","005"]
parallel: false
conflicts_with: []
files:
  - "crates/server/src/routes/events.rs"
  - "crates/server/src/routes/mod.rs"
siblings: ["crates/server/src/routes/logs.rs"]
irreversible: false
scope_test: "crates/server"
allowed_change: mixed
covers_criteria: ["SC4"]
covers_tests: ["TS5"]
---
## Failing test (write first)
**File:** `crates/server/tests/` route tests (or colocated, matching however the server crate
already tests routes — check first). These ARE TS5.

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
**Sibling to read FIRST:** `crates/server/src/routes/logs.rs` and the `stream_raw_stream` /
`stream_normalized_logs` SSE precedent in `crates/services/src/services/container.rs:827,878`
(named by the spec's Constraints as the serving convention to reuse). List their keep-alive
handling, error mapping into the stream, and client-disconnect behaviour, and justify any divergence
in the ledger.
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
ONLY the new route file and the two re-added lines in routes/mod.rs. Do NOT add
authentication or filtering beyond `cursor` — event-type filtering is not in this spec. Do NOT
re-introduce anything resembling the deleted record-patch stream.


## STOP triggers
- `crates/server/src/routes/events.rs` still exists when this task starts — task 001 did not run or
  did not complete; STOP.
- The server crate has no existing route-test harness — record how the tests are structured instead
  of silently downgrading TS5 to manual verification.
- Distinguishing "no cursor" from `cursor=0` is awkward in the extractor — it MUST be distinguished;
  STOP rather than collapsing them.


## Manual verification (record in decisions-ledger)
Gate invocation (the Done-when placeholders): this is a Rust crate, so the runner MUST be overridden — the auto-detected runner would try vitest. Use WAI_TYPECHECK_CMD="cargo check --workspace" with the WAI_TEST_CMD given below.
WAI_TEST_CMD="cargo test -p server events"

Live SC4 check (record the transcript in the ledger), against a running node:
1. `curl -N http://<node>/api/events` — receives live events only; note the highest seq seen.
2. Disconnect. Create/move several tasks.
3. `curl -N "http://<node>/api/events?cursor=<last-seen-seq>"` — every event created while
   disconnected arrives, in ascending seq order, none missing.


## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-event-bus 010` exits 0
