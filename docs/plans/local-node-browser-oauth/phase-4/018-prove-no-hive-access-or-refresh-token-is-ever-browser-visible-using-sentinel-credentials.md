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
siblings: ["crates/server/tests/events.rs","crates/server/tests/harness_smoke.rs","crates/server/tests/mcp_context_test.rs","crates/server/tests/browser_auth_routes.rs","crates/server/tests/browser_oauth.rs","crates/server/tests/restart_outage.rs","crates/server/tests/tasks_delete_routes.rs","frontend/src/components/auth/__tests__/AuthBoundary.test.tsx"]
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

Local helpers this task introduces: `assert_clean()`, `scan_resp()`, `scan_logs()`, `login()`, `storageText()`, `assertClean()`, `scanBrowserSurfaces()`.

### Locked backend (`crates/server/tests/token_disclosure.rs`)

Public harness only. Do not edit `crates/server/tests/common/mod.rs`. Do not construct `RemoteClient`. Do not call `ws_probe`, `write_refresh_only_credentials`, `mock_hive_delayed`, `mock_hive_connection_reset`, or `mock_hive_failure` (empty body). `#[allow(dead_code)] mod common;` then `use common::{CookieJar, HiveHarness, Resp};`. Every test is `#[tokio::test]`, `#[serial_test::serial]`, `#[tracing_test::traced_test]`. Use `HiveHarness::configured()`.

```rust
const ACCESS_LABEL: &str = "SENTINEL-ACCESS-8f31c0d2";
const REFRESH_SENTINEL: &str = "SENTINEL-REFRESH-4b7ae19f";

fn assert_clean(label: &str, haystack: &str, access_jwt: &str) {
    assert!(
        !haystack.contains(access_jwt),
        "{label} leaked access JWT: {haystack}"
    );
    assert!(
        !haystack.contains(REFRESH_SENTINEL),
        "{label} leaked refresh sentinel: {haystack}"
    );
}

fn scan_logs(label: &str, access_jwt: &str, logs_contain: impl Fn(&str) -> bool) {
    assert!(
        !logs_contain(access_jwt),
        "{label} logs leaked access JWT"
    );
    assert!(
        !logs_contain(REFRESH_SENTINEL),
        "{label} logs leaked refresh sentinel"
    );
}

fn scan_resp(
    label: &str,
    resp: &Resp,
    access_jwt: &str,
    jar: &CookieJar,
    logs_contain: impl Fn(&str) -> bool,
) {
    assert_clean(&format!("{label} body"), &resp.body, access_jwt);
    for (name, value) in resp.headers.iter() {
        assert_clean(
            &format!("{label} header {name}"),
            value.to_str().unwrap_or("<bin>"),
            access_jwt,
        );
    }
    for cookie in &resp.set_cookie {
        assert_clean(&format!("{label} set-cookie"), cookie, access_jwt);
    }
    if let Some(cookie) = jar.header_value() {
        assert_clean(&format!("{label} jar"), &cookie, access_jwt);
    }
    scan_logs(label, access_jwt, logs_contain);
}

async fn login(
    h: &HiveHarness,
    subject: uuid::Uuid,
    app_code: &str,
    access_label: &str,
    refresh: &str,
    logs_contain: impl Fn(&str) -> bool,
) -> CookieJar {
    let handoff_id = h
        .mock_hive_oauth(app_code, access_label, refresh, subject)
        .await;
    let mut jar = CookieJar::fresh();
    let init = h
        .post_with(
            "/api/auth/handoff/init",
            serde_json::json!({"provider":"github","return_to":"/"}),
            &mut jar,
        )
        .await;
    assert_eq!(init.status, 200, "init body: {}", init.body);
    let access_jwt = h.access_token_for_label(ACCESS_LABEL);
    scan_resp(&format!("init {app_code}"), &init, &access_jwt, &jar, &logs_contain);
    let complete = h
        .get_no_redirect(
            &format!("/api/auth/handoff/complete?handoff_id={handoff_id}&app_code={app_code}"),
            &mut jar,
        )
        .await;
    scan_resp(&format!("complete {app_code}"), &complete, &access_jwt, &jar, &logs_contain);
    assert!(
        matches!(complete.status, 200 | 204 | 302),
        "complete {app_code} status: {} body: {}",
        complete.status,
        complete.body
    );
    jar
}
```

`#[tracing_test::traced_test]` injects a **local** `fn logs_contain(val: &str) -> bool` into each test body. There is no `tracing_test::logs_contain`. Pass that injected function into `login` / `scan_resp` / `scan_logs`. Do not call `tracing_test::internal::logs_with_scope_contain` (it needs the per-test span name).

Four named tests, nothing else:

1. `scanner_detects_deliberate_jwt_log_leak` — `let access_jwt = h.access_token_for_label(ACCESS_LABEL); tracing::error!("{access_jwt}"); assert!(logs_contain(&access_jwt));` Use the injected `logs_contain`, not a crate path. Do **not** call `scan_logs` after the deliberate leak. This is the backend mutation self-check.

2. `sentinel_oauth_surfaces_do_not_disclose_tokens` — F7 ordering:
   - `owner = Uuid::new_v4()`
   - `let access_jwt = h.access_token_for_label(ACCESS_LABEL);` (call this once; never scan `ACCESS_LABEL` plaintext)
   - `mut other_jar = login(h, owner, "code-a", "other-access", "other-refresh", logs_contain).await` first
   - `mut sentinel_jar = login(h, owner, "code-1", ACCESS_LABEL, REFRESH_SENTINEL, logs_contain).await` second
   - With `sentinel_jar`, `get_with` each of `/api/auth/state`, `/api/info`, `/api/auth/status`, `/api/projects` and `scan_resp(..., logs_contain)` each
   - `POST /api/auth/browser/logout` with `sentinel_jar`; assert `200 | 204`; `scan_resp(..., logs_contain)`
   - `POST /api/auth/logout` with `other_jar`; assert success **not** 401 (`200 | 204`); `scan_resp(..., logs_contain)` + `scan_logs("disconnect", &access_jwt, logs_contain)`

3. `different_owner_complete_does_not_disclose_or_write` — pin owner via `login(h, owner, "code-a", "other-access", "other-refresh", logs_contain)`. Then `mock_hive_oauth("code-intruder", ACCESS_LABEL, REFRESH_SENTINEL, Uuid::new_v4())`, fresh jar, init + `get_no_redirect` complete. Assert status 400 and body contains `owned by a different account`. `scan_resp(..., logs_contain)` init and complete. Assert `node_owner.hive_user_id` still equals `owner` (same SQL as `browser_auth_routes.rs` `stored_owner_uuid`) and `h.credentials_path().exists()`.

4. `upstream_5xx_body_with_sentinels_is_not_forwarded` — do **not** spawn/abort/count/retry. Isolated fixture:
   ```rust
   let access_jwt = h.access_token_for_label(ACCESS_LABEL);
   let handoff_id = uuid::Uuid::new_v4();
   h.mock_json(
       "POST",
       "/v1/oauth/web/init",
       200,
       serde_json::json!({"handoff_id": handoff_id, "authorize_url": "https://github.com/login/oauth/authorize"}),
   ).await;
   h.mock_json(
       "POST",
       "/v1/oauth/web/redeem",
       500,
       serde_json::json!({"access_token": access_jwt, "refresh_token": REFRESH_SENTINEL, "error": "upstream"}),
   ).await;
   ```
   Fresh jar → POST `/api/auth/handoff/init` → `get_no_redirect` `/api/auth/handoff/complete?handoff_id={handoff_id}&app_code=code-5xx`. `scan_resp(..., logs_contain)` both responses. Do not assert a specific success status (this is a failure fixture). Do not use `post_with` as a Hive-outage oracle.

### Locked frontend (`frontend/src/components/auth/__tests__/tokenDisclosure.test.tsx`)

Hoist-mock `browserAuthApi` exactly like `AuthBoundary.test.tsx` (`vi.hoisted` + `vi.mock('@/lib/api/browserAuth')`). Import real `AuthBoundary` from `../AuthBoundary` and real `configApi` from `@/lib/api`. Do **not** mount `App`, `AppContent`, or `UserSystemProvider` (Config shape is too large; STOP if you think you must). Do not edit `OAuthDialog`. `fireEvent` only if needed; no `userEvent`.

```ts
const ACCESS_JWT =
  'eyJhbGciOiJub25lIn0.eyJ0ZXN0X2xhYmVsIjoiU0VOVElORUwtQUNDRVNTLThmMzFjMGQyIn0.sentinel';
const REFRESH_SENTINEL = 'SENTINEL-REFRESH-4b7ae19f';

function storageText(storage: Storage): string {
  const entries: string[] = [];
  for (let i = 0; i < storage.length; i++) {
    const key = storage.key(i)!;
    entries.push(key, storage.getItem(key) ?? '');
  }
  return entries.join('\n');
}

function assertClean(haystack: string) {
  expect(haystack).not.toContain(ACCESS_JWT);
  expect(haystack).not.toContain(REFRESH_SENTINEL);
}

function scanBrowserSurfaces() {
  assertClean(document.body.textContent ?? '');
  assertClean(storageText(localStorage));
  assertClean(storageText(sessionStorage));
  assertClean(window.location.href);
}
```

Never `JSON.stringify(localStorage)` / `JSON.stringify(sessionStorage)`. Never `JSON.stringify` an `/api/info` payload into the DOM.

Three tests:

1. `scanner detects deliberate JWT leak in DOM and storage` — `localStorage.setItem('leak', ACCESS_JWT); render(<div>{ACCESS_JWT}{REFRESH_SENTINEL}</div>);` then `expect(() => assertClean(document.body.textContent ?? '')).toThrow();` and `expect(() => assertClean(storageText(localStorage))).toThrow();` then `localStorage.removeItem('leak')`. This is the frontend mutation self-check.

2. `unauthorized auth-state with unexpected sentinel fields does not disclose` — `browserAuthApi.getState.mockResolvedValue({ authorized: false, oauth_available: true, access_token: ACCESS_JWT, refresh_token: REFRESH_SENTINEL });` render `<AuthBoundary>protected</AuthBoundary>`. Wait for `login-shell`. `scanBrowserSurfaces()`.

3. `authorized bootstrap with unexpected sentinel fields does not disclose` — `getState` resolves `{ authorized: true, oauth_available: true, access_token: ACCESS_JWT, refresh_token: REFRESH_SENTINEL }`. Spy `globalThis.fetch` so a URL containing `/api/info` returns HTTP 200 JSON:
   `{ analytics_user_id: 'probe-user', access_token: ACCESS_JWT, refresh_token: REFRESH_SENTINEL, config: {}, login_status: { status: 'loggedout' }, environment: {}, executors: {}, capabilities: {} }`
   and any other URL returns 404. Probe child:
   ```tsx
   function Probe() {
     const [id, setId] = React.useState('');
     React.useEffect(() => {
       void configApi.getConfig().then((info) => setId(info.analytics_user_id));
     }, []);
     return <div data-testid="authorized-probe">{id}</div>;
   }
   ```
   Render `<AuthBoundary><Probe /></AuthBoundary>`. `waitFor` `authorized-probe` to have text `probe-user`. `scanBrowserSurfaces()`. The probe must render only `analytics_user_id`, never the raw info object.

Siblings listed in frontmatter are read-only. Ledger any undictated choice under `## Task 018 decisions`.


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
  "A sentinel appearing anywhere — fix its owning task in-session; never weaken the assertion.",
  "Mounting App, AppContent, or UserSystemProvider, or JSON.stringify-ing the /api/info payload into the DOM.",
  "Using mock_hive_failure, spawn/abort/hive_request_count, or write_refresh_only_credentials — those are task 015."
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
