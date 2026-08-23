---
id: "010"
phase: 2
title: "Claim the handoff atomically against the presenting browser before any Hive redemption"
status: ready
depends_on: ["009"]
parallel: false
conflicts_with: ["008","009","011","012","022"]
files:
  - "crates/server/src/routes/oauth.rs"
  - "crates/server/tests/browser_oauth.rs"
irreversible: false
scope_test: "crates/server/tests/browser_oauth.rs"
allowed_change: edit
covers_criteria: ["SC3","SC4"]
covers_tests: []
---
## Failing test (write first)
Append to `crates/server/tests/browser_oauth.rs` (all `#[serial_test::serial]`):

```rust
/// Drive initiation in `jar` and return (handoff_id, callback_path).
async fn start_login(h: &common::HiveHarness, jar: &mut common::CookieJar,
    handoff_id: uuid::Uuid) -> String {
    let res = h.post_with("/api/auth/handoff/init",
        serde_json::json!({"provider":"github","return_to":"/"}), jar).await;
    assert_eq!(res.status, 200, "body: {}", res.body);
    format!("/api/auth/handoff/complete?handoff_id={handoff_id}&app_code=code-1")
}

#[tokio::test]
#[serial_test::serial]
async fn a_copied_callback_url_cannot_be_completed_in_another_browser() {
    let h = common::HiveHarness::configured().await;
    let id = h.mock_hive_oauth("code-1", "acc", "ref", uuid::Uuid::new_v4()).await;
    let mut a = common::CookieJar::new();
    let mut b = common::CookieJar::fresh();
    let cb = start_login(&h, &mut a, id).await;

    // Browser B copies the URL. B has no binding cookie at all.
    let stolen = h.get_with(&cb, &mut b).await;
    assert_eq!(stolen.status, 400, "body: {}", stolen.body);
    assert!(stolen.set_cookie.iter().all(|c| !c.starts_with("vks_browser_session=")),
        "a wrong-browser callback must not mint a session");
    let state: String = sqlx::query_scalar(
        "SELECT state FROM browser_oauth_handoffs WHERE handoff_id = ?")
        .bind(id).fetch_one(h.pool()).await.unwrap();
    assert_eq!(state, "pending", "the rightful handoff must NOT have been consumed");

    // The rightful browser still completes.
    let ok = h.get_with(&cb, &mut a).await;
    assert_eq!(ok.status, 200, "body: {}", ok.body);
}

#[tokio::test]
#[serial_test::serial]
async fn a_forged_binding_cookie_does_not_consume_the_handoff() {
    let h = common::HiveHarness::configured().await;
    let id = h.mock_hive_oauth("code-1", "acc", "ref", uuid::Uuid::new_v4()).await;
    let mut a = common::CookieJar::new();
    let cb = start_login(&h, &mut a, id).await;
    let mut forged = common::CookieJar::fresh();
    forged.insert("vks_browser_binding", "not-the-real-secret");
    assert_eq!(h.get_with(&cb, &mut forged).await.status, 400);
    let state: String = sqlx::query_scalar(
        "SELECT state FROM browser_oauth_handoffs WHERE handoff_id = ?")
        .bind(id).fetch_one(h.pool()).await.unwrap();
    assert_eq!(state, "pending");
}

#[tokio::test]
#[serial_test::serial]
async fn replaying_a_completed_callback_is_rejected() {
    let h = common::HiveHarness::configured().await;
    let id = h.mock_hive_oauth("code-1", "acc", "ref", uuid::Uuid::new_v4()).await;
    let mut a = common::CookieJar::new();
    let cb = start_login(&h, &mut a, id).await;
    assert_eq!(h.get_with(&cb, &mut a).await.status, 200);
    let replay = h.get_with(&cb, &mut a).await;
    assert_eq!(replay.status, 400, "a claimed handoff is terminal: {}", replay.body);
}

#[tokio::test]
#[serial_test::serial]
async fn an_expired_handoff_cannot_be_completed() {
    let h = common::HiveHarness::configured().await;
    let id = h.mock_hive_oauth("code-1", "acc", "ref", uuid::Uuid::new_v4()).await;
    let mut a = common::CookieJar::new();
    let cb = start_login(&h, &mut a, id).await;
    // Age the row past its TTL through the DB rather than by sleeping.
    sqlx::query("UPDATE browser_oauth_handoffs SET expires_at = created_at WHERE handoff_id = ?")
        .bind(id).execute(h.pool()).await.unwrap();
    assert_eq!(h.get_with(&cb, &mut a).await.status, 400);
}
```
Note: an unauthorized browser B is also asserted to still see `authorized:false` from `/api/auth/state` in task 011's suite once sessions exist.

## Panel-strengthened assertions (round-1 remediation)
The four tests must additionally pin claim-before-redeem and headers-only binding, and one new test pins epoch-drop-before-redeem:
1. In `a_copied_callback_url_cannot_be_completed_in_another_browser`: before the stolen attempt record `redeems = h.hive_request_count("POST", "/v1/oauth/web/redeem").await`; after browser B's 400 assert the count is unchanged; then a second stolen attempt whose URL appends browser A's RAW binding token as `&vks_browser_binding=<raw>` (extract from `a.header_value()`) must also 400, leave the count unchanged and the row `pending` — pinning that the binding secret is read only from request headers, never a query parameter; the rightful completion then takes the count to exactly `redeems + 1`.
2. In `a_forged_binding_cookie_does_not_consume_the_handoff` and `an_expired_handoff_cannot_be_completed`: assert the redeem count is unchanged by the 400 attempt.
3. In `replaying_a_completed_callback_is_rejected`: after the successful completion the redeem count is 1; after the rejected replay it is still 1.
4. New test `completion_drops_the_epoch_fence_before_hive_redemption` (`#[serial_test::serial]`): mount `mock_hive_oauth`, drive `start_login` in jar A, mount `let redeem_arrived = h.mock_hive_delayed("POST", "/v1/oauth/web/redeem").await` (priority-1, 60s delay), spawn the callback GET via raw reqwest with jar A's Cookie header (clone `h.addr()`, `use deployment::Deployment;` for the epoch); await `redeem_arrived` under a 2s timeout so redemption is provably in flight; then `h.deployment().browser_auth_epoch().try_lock()` must succeed — the epoch guard was dropped before Hive I/O; drop the guard and abort the spawned request task (a mutant holding the epoch across redemption makes `try_lock` fail while the delayed response is pending).


## Change
**File:** `crates/server/src/routes/oauth.rs`
**Anchor:** `async fn handoff_complete`, the handoff lookup at L92-106.
**Before:**
```rust
    let (provider, app_verifier) = match deployment.take_oauth_handoff(&query.handoff_id).await {
        Some(state) => state,
        None => {
            tracing::warn!(
                handoff_id = %query.handoff_id,
                "received callback for unknown handoff"
            );
            return Ok(simple_html_response(
                StatusCode::BAD_REQUEST,
                "OAuth handoff not found or already completed".to_string(),
            ));
        }
    };
```
**After:**
```rust
    // Claim BEFORE any hive I/O. One conditional UPDATE decides the single consumer: a
    // wrong-browser, expired or replayed attempt matches no row and therefore consumes nothing,
    // leaving a rightful pending handoff exactly as it was (SC3/SC4).
    let binding_hash = match read_cookie(&headers, BINDING_COOKIE) {
        Some(raw) => hash_token(&raw),
        None => {
            tracing::warn!(handoff_id = %query.handoff_id, "callback without a binding cookie");
            return Ok(simple_html_response(
                StatusCode::BAD_REQUEST,
                "This browser did not start this sign-in. Start again from the app.".to_string(),
            ));
        }
    };

    // Claim and epoch capture are one short linearization section. Disconnect cannot fit between
    // them and make a stale callback appear current at commit time. No Hive I/O runs under it.
    let epoch_guard = deployment.browser_auth_epoch().lock().await;
    let epoch_at_claim = *epoch_guard;
    let claimed = claim_handoff(
        &deployment.db().pool,
        query.handoff_id,
        &binding_hash,
        SystemClock.now_millis(),
    )
    .await
    .map_err(ApiError::Database)?;
    drop(epoch_guard);

    let Some(handoff) = claimed else {
        // Deliberately ONE message for unknown / wrong-browser / expired / already-claimed: the
        // distinction is not the browser's business, and a claimed row is terminal either way --
        // recovery is a fresh initiation, never a re-claim.
        tracing::warn!(handoff_id = %query.handoff_id, "handoff not claimable");
        return Ok(simple_html_response(
            StatusCode::BAD_REQUEST,
            "OAuth handoff not found, expired, or already completed".to_string(),
        ));
    };
    let (provider, app_verifier) = (handoff.provider, handoff.app_verifier);
```
The handler signature gains `headers: axum::http::HeaderMap` (axum extracts it before `Query`/`State` ordering constraints apply — put it after `State` and before `Query`). Imports to add: `crate::auth::cookies::{BINDING_COOKIE, read_cookie}`, `db::models::browser_auth::claim_handoff`.

Everything after this block (redeem, credential save, sync spawn) is UNCHANGED in this task; task 011 reorders it.

**Symbol grounding:** This task introduces no new function: it rewrites the handoff-lookup block inside the existing `handoff_complete()` handler and adds a local `start_login()` helper to the test file. It calls `claim_handoff()` (task 004), `read_cookie()` (task 007) and `hash_token()` (task 002), replacing the pre-existing `take_oauth_handoff()` call.


## Allowed moves
[
  "Replace exactly the take_oauth_handoff lookup block in handoff_complete, add the headers extractor and the two imports.",
  "Capture epoch_at_claim and claim_handoff while holding one short browser_auth_epoch guard, then drop it before Hive I/O.",
  "Append the four tests to crates/server/tests/browser_oauth.rs.",
  "Do not touch handoff_init, logout, status, the routers, or the code after the claim block."
]


## STOP triggers
[
  "Any SELECT/lookup of the handoff row BEFORE the claim UPDATE — that reintroduces wrong-browser consumption and breaks the SC3/SC4 tests.",
  "Different error messages among CLAIM failures — unknown id, wrong binding cookie, expired, and replayed must all share the one claim-failure message prescribed by the After block. (The earlier no-binding-cookie exit is a separate pre-claim branch: it performs no DB access, reveals nothing about any handoff row, and the browser already knows whether it holds a cookie; its 'start again' guidance message is prescribed by the After block.)",
  "Any attempt to re-claim or un-claim after a failed redemption — a claimed handoff is terminal by design; recovery is a fresh initiation.",
  "Reading the binding cookie from anywhere but the request headers (e.g. a query parameter) — it would then be copyable.",
  "The claim happening after `client.handoff_redeem(...)` — the one-time hive code would then be burned by a wrong browser.",
  "Reading epoch_at_claim outside the same guard that encloses claim_handoff — disconnect could linearize between them.",
  "Holding browser_auth_epoch across redemption/profile Hive I/O."
]


Declared decision points (from the spec; do not edit here):
- DP1: Execution reaches the additive SQLite migration task and requires explicit approval before applying an irreversible production schema migration.  [codes: human_gate_required]


## Manual verification (record in decisions-ledger)
1. `WAI_ROOT="$HOME/.agents/wai"; test -x "$WAI_ROOT/scripts/task-gate.sh"; WAI_TYPECHECK_CMD="cargo fmt --all -- --check" WAI_TEST_CMD="cargo test -p server --test browser_oauth" bash "$WAI_ROOT/scripts/task-gate.sh" local-node-browser-oauth 010` exits 0.
2. `cargo test -p server --test browser_oauth` — 8 tests green (3 from task 009 + 4 new + 1 panel-strengthening test).
3. `git grep -n 'take_oauth_handoff' crates/server/` returns nothing.
4. SC3/SC4 walk-through recorded in the ledger: name the test that proves each clause — expiry (an_expired_handoff_cannot_be_completed), single consumer (concurrent claim, task 004), copied URL (a_copied_callback_url_cannot_be_completed_in_another_browser), replay (replaying_a_completed_callback_is_rejected).


## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash "$HOME/.agents/wai/scripts/task-gate.sh" local-node-browser-oauth 010` exits 0
