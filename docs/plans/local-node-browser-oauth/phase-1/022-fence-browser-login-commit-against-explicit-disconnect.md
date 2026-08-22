---
id: "022"
phase: 1
title: "Fence browser-login commit against explicit disconnect"
status: ready
depends_on: ["004","005"]
parallel: false
conflicts_with: ["003","004","005","009","010","011","012"]
files:
  - "crates/db/src/models/browser_auth/handoff.rs"
  - "crates/db/src/models/browser_auth/mod.rs"
  - "crates/deployment/src/lib.rs"
  - "crates/local-deployment/src/lib.rs"
irreversible: false
scope_test: "crates/db/src/models/browser_auth/handoff.rs"
allowed_change: edit
covers_criteria: []
covers_tests: []
---
## Failing test (write first)
Append these tests before implementation.

In `crates/db/src/models/browser_auth/handoff.rs`:

```rust
#[tokio::test]
async fn invalidating_pending_handoffs_makes_them_durably_unclaimable() {
    let pool = create_test_pool().await;
    let pending = Uuid::new_v4();
    let already_claimed = Uuid::new_v4();
    create_handoff(&pool, pending, "github", "v1", "h1", 0).await.unwrap();
    create_handoff(&pool, already_claimed, "github", "v2", "h2", 0).await.unwrap();
    assert!(claim_handoff(&pool, already_claimed, "h2", 1).await.unwrap().is_some());

    assert_eq!(invalidate_pending_handoffs(&pool).await.unwrap(), 1);
    assert!(claim_handoff(&pool, pending, "h1", 1).await.unwrap().is_none());
    assert_eq!(invalidate_pending_handoffs(&pool).await.unwrap(), 0);
}

#[tokio::test]
async fn handoff_invalidation_does_not_touch_owner_or_sessions() {
    let pool = create_test_pool().await;
    let owner = Uuid::new_v4();
    pin_or_verify_owner(&pool, owner, 10).await.unwrap();
    create_session(&pool, Uuid::new_v4(), "session-hash", owner, 11).await.unwrap();
    create_handoff(&pool, Uuid::new_v4(), "github", "v", "binding-hash", 12)
        .await.unwrap();

    assert_eq!(invalidate_pending_handoffs(&pool).await.unwrap(), 1);
    assert_eq!(get_owner(&pool).await.unwrap().unwrap().hive_user_id, owner);
    assert!(authenticate_session(&pool, "session-hash").await.unwrap().is_some());
}
```

Import the already-public owner/session helpers only inside the test module.

In `crates/local-deployment/src/lib.rs`, append:

```rust
#[tokio::test]
async fn browser_auth_epoch_is_shared_by_deployment_clones() {
    let (pool, _temp_dir) = create_test_pool_with_migrations().await;
    let deployment = LocalDeployment::for_test(pool, test_tuning()).await.unwrap();
    let clone = deployment.clone();
    assert_eq!(*deployment.browser_auth_epoch().lock().await, 0);
    *clone.browser_auth_epoch().lock().await += 1;
    assert_eq!(*deployment.browser_auth_epoch().lock().await, 1);
    deployment.event_bus().shutdown().await;
}

#[tokio::test(flavor = "current_thread")]
async fn configured_startup_sync_is_installed_before_constructor_returns() {
    // Build through `from_parts` with injected ShareConfig and loaded test credentials. On a
    // current-thread runtime the old detached `spawn_remote_sync` cannot run before `from_parts`
    // returns, so this assertion deterministically catches the startup/disconnect race.
    let deployment = /* migrated test DB + loaded test credentials + injected ShareConfig */;
    assert!(deployment.share_sync_handle().lock().await.is_some());
    deployment
        .share_sync_handle()
        .lock()
        .await
        .take()
        .unwrap()
        .shutdown()
        .await;
    deployment.event_bus().shutdown().await;
}
```

Keep the second test's setup local to the test module. Refactor `from_parts` to accept the already
parsed `Option<ShareConfig>` as an internal dependency: production `new()` passes
`ShareConfig::from_env()`, ordinary `for_test()` passes `None`, and this test passes a loopback
configuration directly. Derive `api_base` from that injected configuration instead of reading the
same environment variable a second time. This is a behavior-preserving seam refactor and avoids
process-global environment mutation in a parallel test binary.

## Change
**File:** `crates/db/src/models/browser_auth/handoff.rs`

After `claim_handoff`, add exactly one durable invalidation operation:

```rust
/// Make every pending browser OAuth handoff terminal.
///
/// Explicit Hive disconnect calls this while holding the browser-auth epoch guard. Reusing the
/// existing terminal `claimed` state means callbacks that linearized before disconnect either
/// commit before it or fail their epoch re-check, while callbacks after disconnect cannot claim a
/// pre-disconnect handoff. The state survives restart; no new migration is required.
pub async fn invalidate_pending_handoffs(pool: &SqlitePool) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE browser_oauth_handoffs SET state = 'claimed' WHERE state = 'pending'",
    )
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}
```

**File:** `crates/db/src/models/browser_auth/mod.rs`

Add `invalidate_pending_handoffs` to the existing handoff re-export. Do not export another state
type and do not change the migration.

**File:** `crates/deployment/src/lib.rs`

Immediately after `share_sync_handle`, add the shared commit epoch accessor:

```rust
fn browser_auth_epoch(&self) -> &Arc<Mutex<u64>>;
```

Immediately after the existing detached `spawn_remote_sync`, add a synchronous install path used
by browser-login commit and configured startup:

```rust
async fn install_remote_sync(&self, config: ShareConfig) {
    let mut slot = self.share_sync_handle().lock().await;
    if slot.is_none() {
        tracing::info!("Starting shared task sync");
        *slot = Some(RemoteSync::spawn(
            self.db().clone(),
            config,
            self.auth_context().clone(),
        ));
    }
}
```

Do not change `spawn_remote_sync`; the legacy OAuth route retains its current behavior until task
011 moves it to the synchronous method. Configured startup must call
`deployment.install_remote_sync(sc).await` before `from_parts` returns. This closes the startup
variant of the detached-install race as well: once the deployment can be served, disconnect must
observe the installed handle.

**File:** `crates/local-deployment/src/lib.rs`

Add `browser_auth_epoch: Arc<Mutex<u64>>` beside `share_sync_handle`, construct it as
`Arc::new(Mutex::new(0))`, store it in the `Self` literal, and implement the trait accessor beside
`share_sync_handle()`. Clones must share the same `Arc`.

**Symbol grounding:** This task introduces function `invalidate_pending_handoffs()`,
`Deployment::browser_auth_epoch()` and `Deployment::install_remote_sync()`. It follows the existing
`share_sync_handle()` field/accessor pattern in `crates/deployment/src/lib.rs` and
`crates/local-deployment/src/lib.rs`; it reuses the synchronous `RemoteSync::spawn()` already called
inside `spawn_remote_sync()`.

## Allowed moves
[
  "Append the two handoff invalidation tests, the clone-sharing test, and the deterministic current-thread startup-install test before the behavioral startup edit.",
  "Add exactly one UPDATE helper and re-export it; do not change schema or existing owner/handoff/session semantics.",
  "Add one shared u64 mutex field/accessor and one synchronous sync-install default method.",
  "Refactor from_parts to inject the parsed optional ShareConfig, then synchronously install configured startup sync before returning.",
  "Do not edit any OAuth route in this task; tasks 009-012 consume these primitives."
]

## STOP triggers
[
  "Any migration or schema change — the approved irreversible task 001 does not authorize one.",
  "Adding a third handoff state — terminal invalidation must reuse the existing claimed state.",
  "Deleting handoffs or sessions instead of preserving observable terminal/revoked state.",
  "Holding the browser-auth epoch inside spawn_remote_sync or changing that existing method; configured startup changes its call site to install_remote_sync instead.",
  "Using a process-global static epoch instead of one Arc owned by each deployment.",
  "Any route edit or credential operation in this primitive task."
]

## Manual verification (record in decisions-ledger)
1. `WAI_ROOT="$HOME/.agents/wai"; WAI_TYPECHECK_CMD="cargo fmt --all -- --check" WAI_TEST_CMD="cargo test -p db browser_auth::handoff && cargo test -p local-deployment browser_auth_epoch_is_shared_by_deployment_clones && cargo test -p local-deployment configured_startup_sync_is_installed_before_constructor_returns" bash "$WAI_ROOT/scripts/task-gate.sh" local-node-browser-oauth 022` exits 0.
2. `cargo test -p db browser_auth` passes.
3. `cargo clippy -p db -p deployment -p local-deployment --all-targets --all-features -- -D warnings` passes.
4. Record that this task is the integrated phase-1 review remediation; tasks 009-012 must still wire the epoch/invalidation into real routes before SC8 is complete.
5. Record the Stage-2 follow-up evidence: dropping an overwritten `RemoteSyncHandle` does abort its task, but detached configured startup could still install after disconnect observed an empty slot; the synchronous startup call closes that remaining race.

## Done when
`WAI_TYPECHECK_CMD="cargo fmt --all -- --check" WAI_TEST_CMD="cargo test -p db browser_auth::handoff && cargo test -p local-deployment browser_auth_epoch_is_shared_by_deployment_clones && cargo test -p local-deployment configured_startup_sync_is_installed_before_constructor_returns" bash "$HOME/.agents/wai/scripts/task-gate.sh" local-node-browser-oauth 022` exits 0
