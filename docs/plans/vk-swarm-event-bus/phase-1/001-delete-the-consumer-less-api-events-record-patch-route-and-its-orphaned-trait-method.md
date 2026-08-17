---
id: "001"
phase: 1
title: "Delete the consumer-less /api/events record-patch route and its orphaned trait method"
status: passed
depends_on: []
parallel: false
conflicts_with: []
files:
  - "crates/server/src/routes/events.rs"
  - "crates/server/src/routes/mod.rs"
  - "crates/deployment/src/lib.rs"
irreversible: true
scope_test: "crates/server"
allowed_change: mixed
forbid_after: ["stream_events"]
covers_criteria: []
covers_tests: []
---
## Failing test (write first)
N/A — this is a pure removal of dead code with no consumers. Covered by existing tests
staying green (`cargo test --workspace`) plus the mechanical `forbid_after: ["stream_events"]` check,
which greps every tracked file in the validated commit and rejects any surviving reference.

Evidence the route is dead: a repo-wide grep for `api/events` across *.ts/*.tsx/*.rs/*.md/*.mdx
(excluding node_modules, target, docs/plans) returns only the route's own definition and the spec.
No `EventSource` usage exists anywhere in frontend/src or remote-frontend/src.


## Change
**File:** `crates/server/src/routes/events.rs`
**Anchor:** the entire file (28 lines)
**Change:** `git rm crates/server/src/routes/events.rs`

**File:** `crates/server/src/routes/mod.rs`
**Anchor:** module declaration list, L20
**Before:** `pub mod events;`
**After:** (line removed entirely)

**Anchor:** `base_routes` builder chain in `pub async fn router`, L72
**Before:** `        .merge(events::router(&deployment))`
**After:** (line removed entirely)

**File:** `crates/deployment/src/lib.rs`
**Anchor:** the `stream_events` default trait method, L197-205 — its ONLY caller is the route
deleted above.
**Before:**
```rust
    async fn stream_events(
        &self,
    ) -> futures::stream::BoxStream<'static, Result<Event, std::io::Error>> {
        self.events()
            .msg_store()
            .history_plus_stream()
            .map_ok(|m| m.to_sse_event())
            .boxed()
    }
```
**After:** (method removed entirely)

Remove any import in `crates/deployment/src/lib.rs` that becomes unused as a result (likely the
`Event` SSE type and `map_ok`/`boxed` stream adaptors) — `cargo check` will name them exactly.


## Allowed moves
ONLY the deletions above. Do NOT touch `EventService`
(`crates/services/src/services/events.rs` and `crates/services/src/services/events/`) — it stays and
continues to back `/api/tasks/stream/ws`. Do NOT touch `crates/server/src/routes/tasks/`. Do NOT add
the new bus route here; that is task 010.


## STOP triggers
- `crates/server/src/routes/events.rs` contains anything other than the 28-line record-patch route
  (i.e. someone already changed it).
- Removing `stream_events` breaks a caller other than the deleted route — STOP; the grep said there
  was exactly one, so a second caller means the assumption is stale.
- `cargo check --workspace` reports an error outside the three listed files.


## Manual verification (record in decisions-ledger)
Gate invocation (the Done-when placeholders): this is a Rust crate, so the runner MUST be overridden — the auto-detected runner would try vitest. Use WAI_TYPECHECK_CMD="cargo check --workspace" with the WAI_TEST_CMD given below.
WAI_TEST_CMD="cargo test --workspace"

1. `cargo check --workspace` — clean.
2. `git grep -n stream_events` — no hits (this is also enforced by `forbid_after`).
3. `git grep -rn "api/events"` — no hits outside the spec and docs/plans.
4. Confirm the board still streams: `git grep -n "stream/ws" crates/server/src/routes/tasks/mod.rs`
   still shows the route at L66, and `crates/services/src/services/events.rs` is untouched.


## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-event-bus 001` exits 0
