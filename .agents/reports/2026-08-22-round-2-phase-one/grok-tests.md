# Phase-1 integrated review — grok-4.6 (tests / API / platform)

**Target:** disposable worktree `41f55c4b..ae5ee15f` (`ae5ee15f`)
**Lens:** test isolation/destructiveness, mutant resistance, backward compatibility,
build feature/platform behavior, public API/trait impact, latent phase-1 infrastructure
omissions.
**Governing intent:** spec `docs/superpowers/specs/2026-08-21-local-node-browser-oauth.md`,
phase-1 tasks 001–005 + 022, `decisions-ledger.md`.
**Read-only:** source and git untouched.

## Findings

### [INFO] `invalidate_pending_handoffs` tests do not distinguish UPDATE from DELETE

`crates/db/src/models/browser_auth/handoff.rs:227-252` asserts
`rows_affected == 1`, then `claim_handoff(...).is_none()`, then a second invalidate
returns 0. A mutant `DELETE FROM browser_oauth_handoffs WHERE state = 'pending'`
satisfies every assertion: claim of a missing id is also `None`, and there is no
follow-up `SELECT state` (unlike `concurrent_claim_has_exactly_one_consumer` at
`:212-218`).

Task 022 STOP trigger forbids deletion in favor of observable terminal `claimed`
(`022-...md:189`). Implementation is the specified UPDATE (`handoff.rs:90-94`).
The hollow assertion is the task's prescribed test (`022-...md:35-37`), not an
implementation drift. Same shape in `revoke_all_counts_only_live_sessions`
(`session.rs:153-177`): DELETE-as-revoke-all would also make authenticate return
`None`; only `revoke_session` checks the persisted timestamp (`session.rs:143-149`).

Not extra-contractual work for this phase. Strengthen later if a panel wants
row-existence proof next to the STOP trigger.

### [INFO] Configured-startup constructor test starts leftover daemon tasks

`configured_startup_sync_is_installed_before_constructor_returns`
(`local-deployment/src/lib.rs:1316-1358`) is the first unit test that drives
`from_parts` with both loaded credentials and a parseable `api_base`. That combination
makes `start_node_cache_sync` actually spawn (`lib.rs:499-504`, `:876-907`):
`remote_client` is `Ok` and `get_credentials()` is `Some`.

On `current_thread`, `share_sync_handle().lock().await` is uncontended (tokio Mutex
`try_lock` succeeds without parking), so the install-before-return assertion is
deterministic against the old `spawn_remote_sync` mutant. The later
`shutdown().await` / `event_bus().shutdown().await` **do** yield, so
`NodeCacheSyncService::run` (`node_cache.rs:306-311`) can start `do_sync()` against
`http://127.0.0.1:1` (30s reqwest timeout, connection-refused is typically instant).
`RemoteSync::run` (`share.rs:180-188`) also begins `sync_unshared_tasks_on_startup`
before it observes the shutdown oneshot; on an empty migrated pool
`migrate_unlinked_projects` returns at the local `find_unlinked` empty check
(`share.rs:760-764`) and does not hit the network.

The tokio test runtime aborts leftovers when the test function returns. No
cross-test DB sharing. Not a production defect. Hygiene only: the test never
stops node-cache sync (there is no deployment-level stop).

### [INFO] `Deployment::browser_auth_epoch` is a required trait method

`crates/deployment/src/lib.rs:105` adds a non-defaulted method. The only impl is
`LocalDeployment` (`local-deployment/src/lib.rs:729-731`). Cloud remains commented
out (`server/src/lib.rs:10-13`). `install_remote_sync` is a default method
(`deployment/src/lib.rs:125-135`) and does not break existing impls. No test double
implements `Deployment`. Intentional public-trait expansion required by task 022;
no in-repo compile break.

### [INFO] Test fakes ship in the production `server` lib

`FixedClock` / `ScriptedTokenSource` are unconditionally public
(`server/src/auth/seams.rs:22-72`). Required: `crates/server/tests/` links the lib
without `cfg(test)`, and there is no `test-utils` feature (task 002 STOP).
`OsTokenSource` correctly depends on the one-line `base64 = "0.22"` addition;
`Cargo.lock` only adds `"base64"` to the existing `server` package list.

### [INFO] No automated `hash_token` ≡ `hash_sha256_hex` identity test

Task 002 requires byte-identical encoding with `routes/oauth.rs:221-229`. Both
loops are `{:02x}` over `Sha256::digest`. `hash_token_pins_the_stored_encoding`
(`seams.rs:93-107`) pins the empty-string digest from outside the implementation,
so a `hash_token` encoding drift fails. A later edit to only `hash_sha256_hex`
would not. Phase-2 tasks 009–011 migrate those call sites; not a phase-1 hole.

## Disproved suspicions

- **Startup install vs detached spawn is a hollow current_thread test.** After
  `install_remote_sync`, `from_parts` only `tokio::spawn`s node-cache sync and
  returns (`lib.rs:493-506`). No further `.await`. An uncontended
  `Mutex::lock().await` does not park, so `spawn_remote_sync` cannot publish
  `Some` before the assertion. Mutant-resistant for the race it claims to catch.
- **`ftp://` compatibility test mutates process env.** It injects
  `StartupRemoteConfig { api_base: Some("ftp://example.invalid"), share_config: None }`
  (`lib.rs:1377-1380`). Matches production: `ShareConfig::from_env` returns `None`
  when `derive_ws_url` fails (`share/config.rs:38-47`), while `RemoteClient::new`
  only `Url::parse`s (`remote_client.rs:200-211`). `new()` still reads the two
  values independently (`lib.rs:657-672`). Legacy client boundary preserved.
- **Direct `from_parts` tests skip the orphan-cleanup guard.** Both call
  `disable_orphan_cleanup_for_tests()` first (`lib.rs:1318`, `:1363`). `for_test`
  uses the same helper (`:546`). Pre-existing `new_for_drain_test` exposure is
  tracked in `dev-docs/workstreams/local-deployment-test-orphan-cleanup-safety/README.md`,
  not silently deferred.
- **Invalidation / revoke-all touch owner or credentials.**
  `handoff_invalidation_does_not_touch_owner_or_sessions` (`handoff.rs:255-278`)
  pins an owner, creates a live session, and re-reads both. Session SQL is
  `UPDATE ... WHERE revoked_at IS NULL` with no owner/credential writes
  (`session.rs:79-86`). No `node_owner` writer except `pin_or_verify_owner`.
- **Template pool misses the new migration.** `create_test_pool` copies a
  process `OnceCell` template built with `sqlx::migrate!("./migrations")`
  (`test_utils.rs:48-51`). `20260821000000_add_browser_auth.sql` is the highest
  versioned file. Tests keep `TempDir` alive (`_t` / `_tmp` / `_temp_dir`).
- **Macro SQLx / `expires_at` on sessions / `Utc::now` in models.** No
  `query!`/`query_as!`/`query_scalar!` under `browser_auth/`. Session model has
  no `expires_at`. Only `SystemClock` uses `Utc::now` (`seams.rs:18`).
- **Concurrent tests are coin-flips without busy_timeout.** Owner and handoff
  race tests set `PRAGMA busy_timeout = 5000` (`owner.rs:98-101`,
  `handoff.rs:198-201`). Persisted-state assertions are the real proof, matching
  the locked tasks and ledger dismissal of the task-004 loser-error objection.
- **Clone epoch is a unique Mutex per clone.** Test increments via the clone and
  reads the original (`lib.rs:1310-1312`). Independent mutex would fail.

## Planned later work (not phase-1 defects)

- Epoch / invalidate / `install_remote_sync` are not wired into OAuth routes.
  Tasks 009–012 own initiation linearization, claim+epoch capture, callback
  re-check, and disconnect. SC8 is explicitly incomplete until 012
  (`022-...md:200`, ledger “Task 022”).
- Task 012 still uses `spawn_remote_sync` in its first two harness tests
  (`phase-3/012-...md:34-63`); 011 moves login-path install to the synchronous
  method. `spawn_remote_sync` is intentionally unchanged (022 STOP).
- O8 crash window (SQLite revoke-all vs file/Keychain clear) needs a new
  approved migration. Accepted residual (ledger + round-1 report).
- In-memory epoch is not durable across process restart. Durable pending-handoff
  invalidation covers the restart case without a second schema (ledger).
- No FK from `browser_sessions.hive_user_id` to `node_owner`. Caller (task 011)
  pins then mints. Schema matches the approved task-001 SQL.
- `NodeRunnerConfig::from_env` live-hive hazard in `from_parts` is pre-existing
  `F-2026-08-19-02`, not introduced here.
- HTTP/cookie/Wiremock coverage is TS2 (phase 2), not phase 1.

## Fidelity (phase-1 contracts)

| Task | Contract vs tree |
|---|---|
| 001 | Additive SQL byte-matches the task; migration test covers tables, slot CHECK, no session `expires_at`. Approval token present. |
| 002 | Public seams + colocated five tests; fakes not `cfg(test)`; lockfile is the one-line `base64` add. |
| 003 | One-statement pin-or-compare; incumbent `pinned_at` frozen; mismatch side-effect-free on owner row; concurrent one-winner. |
| 004 | TTL computed at create; claim is one `UPDATE ... RETURNING`; strict `expires_at > now`; wrong-browser non-consuming; replay/unknown → `None`. |
| 005 | Auth has no time argument; scoped/idempotent revoke; revoke-all counts live only; unique `token_hash`. |
| 022 | `invalidate_pending_handoffs` reuses `claimed`; epoch is per-deployment `Arc<Mutex<u64>>` shared by clone; startup awaits `install_remote_sync`; raw API base independent of `ShareConfig`; orphan guard extracted; no route/schema/credential edits. |

`from_parts` remains `pub(crate)`. Production `new()` still resolves env and
forwards both values. `for_test()` passes `None`/`None`.

## Verdict

No [BLOCKING] or [SHOULD-FIX] production or test-isolation defect in the phase-1
range. Hollow DELETE-vs-UPDATE assertions and constructor-test leftover tasks are
accepted [INFO]. Route-level SC8 closure belongs to tasks 009–012.

VERDICT: APPROVE
