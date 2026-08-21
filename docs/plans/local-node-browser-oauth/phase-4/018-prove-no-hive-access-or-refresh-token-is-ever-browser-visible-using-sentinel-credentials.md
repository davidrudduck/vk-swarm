---
id: "018"
phase: 4
title: "Prove no Hive access or refresh token is ever browser-visible, using sentinel credentials"
status: ready
depends_on: ["011","012","016"]
parallel: false
conflicts_with: []
files:
  - "crates/server/tests/token_disclosure.rs"
  - "frontend/src/components/auth/__tests__/tokenDisclosure.test.tsx"
siblings: ["crates/server/tests/events.rs","crates/server/tests/harness_smoke.rs","crates/server/tests/mcp_context_test.rs"]
irreversible: false
scope_test: "crates/server/tests/token_disclosure.rs"
allowed_change: create
covers_criteria: ["SC10"]
covers_tests: ["TS6"]
---
## Failing test (write first)
File: `crates/server/tests/token_disclosure.rs` — create header-aware, log-capturing integration tests.

Use stable labels:
```rust
const ACCESS_LABEL: &str = "SENTINEL-ACCESS-8f31c0d2";
const REFRESH_SENTINEL: &str = "SENTINEL-REFRESH-4b7ae19f";
```
After `mock_hive_oauth(... ACCESS_LABEL, REFRESH_SENTINEL, ...)`, obtain `let access_jwt = h.access_token_for_label(ACCESS_LABEL);`. `assert_clean(label, body_or_header, &access_jwt)` scans the **exact complete compact JWT string** plus plaintext refresh sentinel. Do not scan the plaintext label as if it appeared literally inside a base64url claim.

`scan_resp` checks body and **every** `Resp.headers` name/value, including all repeated Location and Set-Cookie values, before scanning the cookie jar and captured tracing logs. Cover initiation, completion through `get_no_redirect`, auth state, info, status, projects, browser logout, Hive disconnect, owner mismatch, and one concrete upstream 5xx-body fixture containing both sentinels.

Create two valid local sessions for the successful-disconnect path. The sentinel-login browser performs `/api/auth/browser/logout`. A separate still-valid browser invokes protected `POST /api/auth/logout`; assert success (not 401), then scan disconnect body/all headers/logs. This proves the disconnect handler executed instead of testing middleware rejection. Keep the different-owner rejection and its no-write assertions. General transport/timeout/refresh continuity remains exclusively task 015.

File: `frontend/src/components/auth/__tests__/tokenDisclosure.test.tsx` — create non-vacuous tests. Mock unauthorized auth-state with unexpected sentinel-bearing fields, render the real `AuthBoundary`, and assert the shell/DOM and storage scans reject the exact access JWT and refresh sentinel. Then mock authorized bootstrap, including sentinel-bearing unexpected fields in auth-state and `/api/info`, mount the actual authorized app path (with only required heavy dependencies mocked), and inspect the resulting DOM.

Enumerate storage correctly:
```ts
function storageText(storage: Storage): string {
  const entries: string[] = [];
  for (let i = 0; i < storage.length; i++) {
    const key = storage.key(i)!;
    entries.push(key, storage.getItem(key) ?? '');
  }
  return entries.join('\n');
}
```
Never use `JSON.stringify(localStorage/sessionStorage)`. Scan exact complete JWT and refresh sentinel in DOM, both Storage objects, URLs/redirect mocks, and any frontend-captured errors. Include a mutation self-check: intentionally render or store the exact JWT inside the test fixture, assert the scanner throws/fails, then remove the mutation and run the real assertions. This proves the detector is live.


## Change
Create only the two test files listed by this task; no production change is expected.

Backend helpers: `assert_clean`, `scan_resp`, `scan_logs`, and local login helpers. They consume task 006's exact generated JWT (`access_token_for_label`), all-header `Resp`, no-redirect Location, and real session harness. Use two sessions so browser logout and protected Hive disconnect are both executed and scanned. A concrete 5xx upstream body carrying sentinels is allowed and desired; transport/timeout/refresh behavior belongs to task 015 and must not be duplicated here.

Frontend helpers enumerate Storage via `length/key/getItem`, mount unauthorized and authorized bootstrap paths, and scan the exact JWT plus refresh sentinel. Inject sentinel-bearing unexpected response fields so a UI/API leak would be observable. The mutation self-check deliberately leaks into DOM/storage and proves the scanner fails before the clean case runs.

If any sentinel appears, fix it in the owning earlier task in the same execution session; do not weaken or blacklist only known field names.


## Allowed moves
[
  "Create exactly the two test files.",
  "No production code changes; no edits to crates/server/tests/common/mod.rs."
]


## STOP triggers
[
  "Scanning ACCESS_LABEL instead of the exact complete generated JWT returned by access_token_for_label.",
  "Scanning only response bodies — every header value, especially Location and repeated Set-Cookie, plus logs and jars must be scanned.",
  "Calling Hive disconnect with the session already revoked by browser logout; use a second valid session and assert disconnect succeeds rather than 401.",
  "Duplicating transport/timeout/refresh continuity from task 015; only a concrete upstream-body 5xx leak fixture belongs here.",
  "Using JSON.stringify(Storage), omitting authorized bootstrap, or mocking responses without sentinel-bearing unexpected fields.",
  "A frontend or backend scanner without a deliberate exact-JWT DOM/storage/log mutation self-check.",
  "A sentinel appearing anywhere — fix its owning task in-session; never weaken the assertion."
]


Declared decision points (from the spec; do not edit here):
- DP1: Execution reaches the additive SQLite migration task and requires explicit approval before applying an irreversible production schema migration.  [codes: human_gate_required]


## Manual verification (record in decisions-ledger)
1. `WAI_ROOT="$HOME/.agents/wai"; test -x "$WAI_ROOT/scripts/task-gate.sh"; WAI_TYPECHECK_CMD="cargo fmt --all -- --check" WAI_TEST_CMD="cargo test -p server --test token_disclosure" bash "$WAI_ROOT/scripts/task-gate.sh" local-node-browser-oauth 018` exits 0.
2. `cargo test -p server --test token_disclosure` — header-aware backend tests green, including successful two-session disconnect and owner mismatch.
3. `cd frontend && npx vitest run src/components/auth/__tests__/tokenDisclosure.test.tsx` green.
4. Vacuity check recorded in the ledger: run the built-in mutation self-checks that leak the exact generated JWT into captured backend logs and frontend DOM/storage; each scanner must fail, then the clean assertions pass. A non-disclosure test that cannot detect a disclosure is worthless.
5. SC10 surface walk-through: initiation, completion, normal use, logout, disconnect and failure — name the assertion covering each.


## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash "$HOME/.agents/wai/scripts/task-gate.sh" local-node-browser-oauth 018` exits 0
