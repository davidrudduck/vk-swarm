---
id: "015"
phase: 3
title: "Prove restart persistence and Hive-outage continuity for established sessions"
status: ready
depends_on: ["011","012","013"]
parallel: false
conflicts_with: []
files:
  - "crates/server/tests/restart_outage.rs"
siblings: ["crates/server/tests/events.rs","crates/server/tests/harness_smoke.rs","crates/server/tests/mcp_context_test.rs"]
irreversible: false
scope_test: "crates/server/tests/restart_outage.rs"
allowed_change: create
red_proof: mutation-self-check
covers_criteria: ["SC9"]
covers_tests: ["TS4"]
---
## Failing test (write first)
File: `crates/server/tests/restart_outage.rs` — create seven `#[serial_test::serial]` deterministic integration tests. Keep `red_proof: mutation-self-check`.

Shared helpers:
- `login(h, subject, app_code, access_label) -> CookieJar` uses task 006 `mock_hive_oauth`; labels remain readable while the harness returns a real future-expiring JWT.
- `seed_local_identity(h)` calls `seed_project("continuity", &[db::models::task::TaskStatus::Todo])`, obtains the seeded task UUID from the migrated pool, and returns both IDs.
- `snapshot_state(h)` captures **exact credential bytes** (`std::fs::read`), pinned owner UUID, and live-session count.
- `assert_local_seams(h, jar, project_id, task_id)` asserts identity/content through `GET /api/info`, `GET /api/projects`, `GET /api/tasks?project_id=<id>`, `GET /api/auth/state` (`authorized:true`), and `sse_probe("/api/events", Some(jar))` with status 200 and content type beginning `text/event-stream`. Every `/api/events` call uses `sse_probe`, never `ws_probe`.
- `await_reached(signal)` wraps the Wiremock one-shot in a 2-second diagnostic watchdog. Signals fire in the priority-1 responder when the exact real method/path arrives.

The seven named tests are:
1. `an_established_session_survives_a_planned_idle_restart`: login, seed project/task, snapshot, restart over the same assets/SQLite path, assert generation completion and all local seams/identities/state bytes unchanged.
2. `restart_rejects_the_stored_hash_presented_as_a_cookie`: retain the mutation self-check; stored hash and unknown token both 401 after restart.
3. `a_revoked_session_stays_revoked_across_restart`: capture raw token before logout, prove 200, logout, restart, replay raw token from fresh jar -> 401, and assert its row remains revoked.
4. `transport_failure_continuity`: mount priority-1 signalled connection reset for exact `POST /v1/oauth/web/init`; spawn a fresh OAuth initiation, await signal/count, allow the request to fail or abort/await if retrying, then assert all local seams and exact state snapshot unchanged.
5. `timeout_in_progress_continuity`: mount priority-1 signalled delayed responder for exact `POST /v1/oauth/web/init`; spawn fresh initiation, await signal under the short watchdog, assert all local seams while the Hive request is still pending, then `abort()` and await the request task. Never wait for the configured delay or RemoteClient retries.
6. `post_restart_refresh_503_continuity`: login, seed the local identity, then restart so no in-memory access token survives and replacement RemoteSync exercises unlinked-project migration. Call task-006 `write_refresh_only_credentials("post-restart-refresh")` to await both current-generation RemoteSync and node-cache shutdown before deterministically replacing persisted credentials. Capture the current refresh-request count, then mount priority-1 signalled 503 on exact `POST /v1/tokens/refresh`; spawn authorized `GET /api/organizations` with the browser cookie to trigger the real refresh path, await the signal, assert the refresh count increased by exactly one, then abort/await the retrying served-router caller. Owned background tasks are stopped, so only the served caller can satisfy the provenance proof. Then assert all local seams and state snapshot unchanged.
7. `hive_5xx_continuity`: priority-1 signalled 503 on exact `POST /v1/oauth/web/init`; fresh OAuth fails with generic 5xx while all established local seams and the exact snapshot remain unchanged.

For tests 4-7, assert `hive_request_count(method, path)` observed the named real request (exactly one new request relative to a captured baseline at the signal/check point), fresh OAuth fails or remains demonstrably pending as appropriate, credentials bytes/owner/live sessions are unchanged, and local seams remain usable. The refresh case uses task 006's owned-background-task shutdown helper before capturing the baseline; after signal plus one-request delta establish provenance, abort/await the retrying caller. No generic mock-only proof, request-count quiescence heuristic, long sleep, or waiting through retries.


## Change
**File:** `crates/server/tests/restart_outage.rs` — create exactly the seven-test module above plus its local helpers. No production code changes.

This task uses only public/task-006 harness contracts: `restart`, generation completion, `get_with`, `post_with`, `sse_probe`, `pool`, `credentials_path`, `write_refresh_only_credentials`, `mock_hive_oauth`, `mock_hive_failure`, `mock_hive_connection_reset`, `mock_hive_delayed`, `hive_request_count`, `deployment`, and `CookieJar`. It never calls a private RemoteClient/RemoteSync constructor.

The access-token argument passed to `mock_hive_oauth` is a stable label; task 006 derives the real JWT. State comparison is byte/identity exact, not “file still exists”. `/api/events` is SSE: require 200 plus `text/event-stream`; WebSocket probes are forbidden here. Each outage is tied to the exact production Hive method/path by a priority-1 signalled responder and recorded-request count. Timeout/refresh caller tasks are explicitly aborted and awaited after the proof point, so the suite never waits 30 seconds or sleeps through retry backoff.

This task adds no production code. If local authorization fails during an outage, fix the owning production task rather than weakening these tests.


## Allowed moves
[
  "Create exactly one integration test file containing the seven named deterministic tests.",
  "Use sse_probe for every /api/events assertion and require 200 plus text/event-stream.",
  "Seed a real project/task and assert exact identities across info/projects/tasks/events/auth-state.",
  "Use priority-1 signalled task-006 outage helpers and exact method/path request counts; abort/await pending or retrying requests.",
  "Use real JWT labels through mock_hive_oauth and compare exact credential bytes, owner UUID, and live session count.",
  "Make no production code changes."
]


## STOP triggers
[
  "Any /api/events ws_probe call — this endpoint is SSE and must use sse_probe with 200 + text/event-stream.",
  "Fewer or more than the seven named serial tests, or ledger/manual wording that still says five.",
  "An outage test that does not observe its exact real Hive method/path through a priority-1 signalled responder.",
  "Sleeping through a delayed response or RemoteClient retries — await the signal, prove continuity while pending, then abort/await.",
  "Refresh coverage without restart first or without GET /api/organizations triggering observed POST /v1/tokens/refresh 503.",
  "Refresh coverage that omits task 006's awaited RemoteSync and node-cache shutdown or aborts the served-router caller before signal plus exact one-request delta are proven — background startup traffic must not satisfy the provenance assertion.",
  "Comparing only credential-file existence rather than exact bytes, or omitting owner UUID/live-session snapshots.",
  "Using a plaintext access label as the bearer token; task 006 must supply the real generated JWT."
]


Declared decision points (from the spec; do not edit here):
- DP1: Execution reaches the additive SQLite migration task and requires explicit approval before applying an irreversible production schema migration.  [codes: human_gate_required]


## Manual verification (record in decisions-ledger)
1. `WAI_ROOT="$HOME/.agents/wai"; test -x "$WAI_ROOT/scripts/task-gate.sh"; WAI_TYPECHECK_CMD="cargo fmt --all -- --check" WAI_TEST_CMD="cargo test -p server --test restart_outage" bash "$WAI_ROOT/scripts/task-gate.sh" local-node-browser-oauth 015` exits 0.
2. `cargo test -p server --test restart_outage` reports exactly **7 tests** green; run the complete suite 3x for flake.
3. Confirm every `/api/events` reference is `sse_probe` and asserts 200 + `text/event-stream`; no `ws_probe` remains.
4. Paste exact method/path arrival/count evidence for transport reset, delayed timeout, refresh 503 and Hive 5xx.
5. RED evidence: preserve `red_proof: mutation-self-check` and prove stored-hash presentation trips the intended mutation.
6. SC9 ledger walk-through maps all seven tests and records exact credential bytes, owner UUID, live count, seeded project/task identities, and local seam assertions.


## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash "$HOME/.agents/wai/scripts/task-gate.sh" local-node-browser-oauth 015` exits 0
