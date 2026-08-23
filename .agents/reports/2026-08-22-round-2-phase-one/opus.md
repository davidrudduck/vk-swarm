Stopped the remaining idle `sleep` wrappers — they were re-echoing outputs I had already read in full. No finding changed. Full report follows.

---

# Integrated phase-1 adversarial review — `local-node-browser-oauth`, range `41f55c4b..ae5ee15f`

## 1. Scope and what I actually ran

- **Range reviewed:** `41f55c4b..ae5ee15f` (25 commits), phase-1 tasks 001–005 plus corrective task 022.
  - Source files: migration, `crates/db/src/models/browser_auth/{mod,owner,handoff,session}.rs`, `crates/db/src/test_utils.rs`, `crates/server/src/auth/{mod,seams}.rs`, `crates/server/{Cargo.toml,src/lib.rs}`, `Cargo.lock`, `crates/deployment/src/lib.rs`, `crates/local-deployment/src/lib.rs`.
  - Governing docs: `docs/superpowers/specs/2026-08-21-local-node-browser-oauth.md` (note: the prompt's `…-design.md` path does not exist; the settled spec is the non-suffixed file), `docs/plans/local-node-browser-oauth/{plan.md,decisions-ledger.md}`, `phase-1/*.md`.
- **Working tree:** clean at `ae5ee15f`; I made no edits and ran no state-changing git command.
- **Commands executed (read-only / build-only):**
  - `cargo test -p db browser_auth` → **17 passed, 0 failed** (16 model tests + the migration test).
  - `cargo test -p local-deployment --lib -- browser_auth_epoch_is_shared_by_deployment_clones configured_startup_sync_is_installed_before_constructor_returns raw_api_base_remains_available_when_share_sync_config_is_unavailable` → **3 passed, 0 failed**.
  - `cargo check -p server --all-targets` → **exit 0, no warnings**. This crate was never re-checked after task 022 added a *required* trait method (`Deployment::browser_auth_epoch`); it compiles clean, including all test targets.
  - `cargo fmt --all -- --check` → **exit 0**.
  - `cargo test -p db` (full) → **FAILED: 301 passed, 1 failed, 7 ignored** — see F1.
  - Three focused re-runs of the failing test → failed 3/3 (4/4 including the suite run).
- **Deliberately not run:** a full `cargo test -p local-deployment`. `dev-docs/workstreams/local-deployment-test-orphan-cleanup-safety/README.md` records that `new_for_drain_test()` call sites still sweep real clean worktrees; running it would have deleted developer state. I set `DISABLE_WORKTREE_ORPHAN_CLEANUP=1`, `DISABLE_WORKTREE_EXPIRED_CLEANUP=1` and a scratch `VK_WORKTREE_DIR` for the three focused tests. Those overrides do not touch anything the three tests assert.

---

## 2. Findings

### F1 `[BLOCKING]` — `cargo test -p db` is red at HEAD, and the tracked scope split this range promised for it was never created

- **File/line:** `.agents/reports/2026-08-22-round-1-cross-model-phase-one.md:74-80` (the promise, committed inside this range) versus the absent `dev-docs/workstreams/sqlite-busy-snapshot-calibration-stability/README.md`. Failing assertion: `crates/db/src/models/execution_process/queries.rs:1437`.
- **Observed failure:**
  ```
  test models::execution_process::queries::lifecycle_event_tests::control_read_then_write_shape_reproduces_busy_snapshot ... FAILED
  panicked at crates/db/src/models/execution_process/queries.rs:1437:9:
  calibration control must reproduce at least one SQLITE_BUSY_SNAPSHOT — 0 here would mean the
  harness cannot detect the hazard, and the real test above would be proving nothing
  test result: FAILED. 301 passed; 1 failed; 7 ignored; 0 measured; 0 filtered out; finished in 81.99s
  ```
  Reproduced 4/4 on this machine (one full-suite run + three focused runs). The paired control in `crates/db/src/models/execution_process/lifecycle.rs:1182` passed in every run — only the `queries.rs` control is failing, contrary to the report's "the two … calibration controls".
- **The promise that was not kept.** The report committed in this range states verbatim:
  > "Panel runs intermittently failed the two execution-process `SQLITE_BUSY_SNAPSHOT` calibration controls because they could not provoke the expected hazard in 200 attempts. The reviewed diff does not touch those files and repeated identical runs oscillated. Per AGENTS.md this will be resolved as the explicit tracked scope split `sqlite-busy-snapshot-calibration-stability` before the session closes; it is not silently carried forward."

  I checked every place that split could live and it is in none of them:
  - `ls dev-docs/workstreams/` — 31 entries, no `sqlite-busy-snapshot-calibration-stability`.
  - `grep -rn "sqlite-busy-snapshot-calibration-stability" dev-docs/ docs/plans/local-node-browser-oauth/` — zero hits.
  - `dev-docs/BACKLOG.md` and `dev-docs/MASTER.md` — no calibration/busy-snapshot row.
  - `docs/plans/local-node-browser-oauth/decisions-ledger.md` — never mentions it.
- **Not caused by this range.** `crates/db/src/models/execution_process/queries.rs` was last modified by `1f2caaea` (#477), which `git merge-base --is-ancestor 1f2caaea 41f55c4b` confirms is an ancestor of the review base. **No browser-auth source change is required to fix this.**
- **Why it is blocking anyway.** AGENTS.md:42 and CLAUDE.md's "No Deferred Remediation" cover exactly this case: pre-existing debt discovered during a session must be *fixed now*, *split as a named scope split with a tracked follow-up workstream created in THIS session*, or *escalated* — and CLAUDE.md's "Finish What We Start" makes green `cargo test --workspace` a merge precondition. The session did honour that standard for the sibling hazard it found (`dev-docs/workstreams/local-deployment-test-orphan-cleanup-safety/README.md`, commit `055a59c0`), which is precisely why the omission here reads as a silent carry-forward rather than an oversight of process. The range therefore ships a committed assertion that a tracking artifact exists when it does not, while the crate's own test gate is reproducibly red.
- **Failure scenario:** a reviewer or CI job runs the documented gate `cargo test --workspace` on `ae5ee15f`; `db` fails; there is no ledger entry, backlog row, or workstream explaining it, so the next session inherits an unexplained red suite and an unfalsifiable "it's just flaky" claim.
- **Minimal remediation (no phase-1 source change):**
  1. Create `dev-docs/workstreams/sqlite-busy-snapshot-calibration-stability/README.md` with the reproduction above (0/200 busy-snapshot, 4/4 failures on this host) and link it from `docs/plans/local-node-browser-oauth/decisions-ledger.md`; **and**
  2. either stabilise the control (raise `ITERATIONS`, or increase writer contention so the read-then-upgrade hazard is provoked deterministically) or mark that single control `#[ignore]` per-item — leaving the real `no_read_then_upgrade` test and the `lifecycle.rs` control live, which satisfies AGENTS.md:49(a).

---

### F2 `[SHOULD-FIX]` — the new startup test is the repo's only test that calls `OAuthCredentials::save()`, and on macOS release-profile test builds it overwrites the developer's real Hive refresh token in the login Keychain

- **File/line:** `crates/local-deployment/src/lib.rs:1320-1330` (`configured_startup_sync_is_installed_before_constructor_returns`).
  ```rust
  let credentials = Arc::new(OAuthCredentials::new(temp_dir.path().join("credentials.json")));
  credentials.save(&services::services::oauth_credentials::Credentials {
      access_token: Some("test-access-token".to_owned()),
      refresh_token: "test-refresh-token".to_owned(),
      expires_at: None,
  }).await.unwrap();
  ```
- **Mechanism.** `crates/services/src/services/oauth_credentials.rs:93-115` chooses the backend:
  ```rust
  let use_file = match std::env::var("OAUTH_CREDENTIALS_BACKEND") {
      Ok(v) if v.eq_ignore_ascii_case("file") => true,
      Ok(v) if v.eq_ignore_ascii_case("keychain") => false,
      _ => cfg!(debug_assertions),
  };
  ```
  When the `KeychainBackend` branch is taken, the `PathBuf` handed to `OAuthCredentials::new` is **discarded** — `crates/services/src/services/oauth_credentials.rs:205-206` uses a fixed, process-independent slot shared with production:
  ```rust
  const SERVICE_NAME: &'static str = concat!(env!("CARGO_PKG_NAME"), ":oauth");
  const ACCOUNT_NAME: &'static str = "default";
  ```
  and `:228-233` calls `set_generic_password(SERVICE_NAME, ACCOUNT_NAME, …)`, an unconditional overwrite.
- **Trigger is an explicit conjunction:** `target_os = "macos"` **AND** `debug_assertions` off (e.g. `cargo test --release`, or a profile that sets `debug-assertions = false`) **AND** `OAUTH_CREDENTIALS_BACKEND` unset. The documented gate (`cargo test --workspace`, dev profile) never fires it, and this reviewing host is Linux — that is why it is not blocking.
- **Failure scenario:** a macOS developer runs `cargo test --release -p local-deployment`; the test writes `{"refresh_token":"test-refresh-token"}` into the `services:oauth`/`default` Keychain item; the next `vks-node-server` start loads that value, fails refresh against the Hive, and the node is silently signed out with no local artifact explaining why.
- **Verified this is a new exposure:** `grep -rn 'credentials.save(\|OAuthCredentials::new' crates/` returns exactly one `.save(` call in any test — line 1324, added by task 022 (`cc70f9d7`/`94e5aecc`). No prior test wrote credentials at all.
- **Why it belongs to this range:** it is the same class of hazard task 022 was mid-remediation for — a direct `from_parts` test constructor reaching real machine state. The orphan-worktree half was caught and split into `local-deployment-test-orphan-cleanup-safety`; this half was not examined.
- **Minimal remediation:** one line inside the `Once` that already exists at `crates/local-deployment/src/lib.rs:510-524`:
  ```rust
  std::env::set_var("OAUTH_CREDENTIALS_BACKEND", "file");
  ```
  It runs under the same `call_once`, carries the same already-documented `set_var` caveat, and pins the file backend on every platform and profile. (An assertion-based alternative — refuse to `save()` unless the file backend is selected — is equally acceptable.)

---

### F3 `[INFO]` — a `PRAGMA busy_timeout` "mitigation" that cannot work, justified by a comment the plan itself contradicts

- **File/line:** `crates/db/src/models/browser_auth/owner.rs:95-101` and `crates/db/src/models/browser_auth/handoff.rs:198-201`.
  ```rust
  // create_test_pool() sets NO busy_timeout (crates/db/src/test_utils.rs:90-100), unlike
  // DBService::new(); without this the loser gets an immediate SQLITE_BUSY and the
  // outcome assertion becomes a coin-flip.
  sqlx::query("PRAGMA busy_timeout = 5000").execute(&pool).await.unwrap();
  ```
- **Two independent reasons this is dead code:**
  1. `busy_timeout` is a **per-connection** SQLite setting. `.execute(&pool)` checks out one connection; `crates/db/src/test_utils.rs:96-97` configures `min_connections(1).max_connections(5)`, so the *second* concurrent future — the only one the mitigation is meant to protect — runs on a different connection that never saw the pragma. Even if the premise were true, the mitigation would not work.
  2. The premise is false. `sqlx-sqlite` 0.8.6 sets `busy_timeout: Duration::from_secs(5)` in `SqliteConnectOptions::default()` (`~/.cargo/registry/src/*/sqlx-sqlite-0.8.6/src/options/mod.rs:201`) and applies it on every connect, so `create_test_pool()` already has exactly 5000 ms. The statement is a no-op.
- **The contradiction is inside the plan, not introduced by the implementer.** The comment is prescribed verbatim by `docs/plans/local-node-browser-oauth/phase-1/003-…md:63-66` and `004-…md:82`, while the *same* task file states the opposite at `003-…md:165`: "sqlx 0.8.6 already installs a 5-second busy timeout on every SQLite connection, including `create_test_pool()`. The explicit `PRAGMA busy_timeout = 5000` in the concurrency test is belt-and-braces only". `decisions-ledger.md` "Review-time decisions" item 3 records the correct fact. The shipped source carries the wrong half.
- **Failure scenario:** a future maintainer tunes `create_test_pool()` (e.g. adds `.busy_timeout(Duration::ZERO)` to make a lock test deterministic), reads `owner.rs:95-97`, believes the pragma protects these tests, and ships a genuinely racy pair.
- **Explicitly disproved:** these two tests are **not** flaky by construction. `owner.rs:113-119` counts `ra.is_ok() as u8 + rb.is_ok() as u8 == 1`, which holds whether the loser lost on owner mismatch or on `SQLITE_BUSY`, and then re-asserts on persisted state (`assert_ne!(owner.hive_user_id, loser)`). `handoff.rs:208-224` uses `r1.as_ref().ok().map_or(...)` for the same reason and re-reads `state` from the row. Both passed here.
- **Minimal remediation:** delete both `PRAGMA` statements and replace the comment with the ledger's fact ("sqlx-sqlite 0.8.6 already applies a 5 s default busy timeout per connection"), or — if the belt-and-braces is genuinely wanted — move it to `SqliteConnectOptions::busy_timeout(...)` in `create_test_pool()` where it applies to every pooled connection.

---

### F4 `[INFO]` — SAFETY comment drift introduced by this range

- **File/line:** `crates/local-deployment/src/lib.rs:513-517`.
  > "…tests on other threads may be inside `from_parts` calling `ShareConfig::from_env`, `NodeRunnerConfig::from_env` or `database_path()` at this moment."
- Task 022 moved `ShareConfig::from_env()` **out** of `from_parts` and into `Deployment::new()` (`crates/local-deployment/src/lib.rs:657-660`), so that specific concurrent `getenv` no longer occurs via `from_parts`. The comment travelled unchanged from its old location during commit `cc70f9d7`'s extraction.
- The **unsoundness claim itself is still correct** — `from_parts` still reads env at `:214` (`VK_NODE_API_KEY`), `:468-470` (the three startup-summary reads), plus `NodeRunnerConfig::from_env()` and `database_path()`. Only the cited call list is now wrong, and it omits two readers it should name.
- **Failure scenario:** a reader grepping for `ShareConfig::from_env` inside `from_parts` finds nothing, concludes the comment is stale in general, and removes the guard or downgrades the caveat.
- **Minimal remediation:** drop `ShareConfig::from_env` from the list and add `std::env::var("VK_NODE_API_KEY")`.

---

### F5 `[INFO]` — no retention path for `browser_oauth_handoffs` or `browser_sessions`, and no task owns one

- **File/line:** `crates/db/migrations/20260821000000_add_browser_auth.sql:26-36` and `:41-48`.
- Every OAuth initiation appends a permanent row holding a **raw** `app_verifier`; every login appends a permanent session row that revocation only marks (`revoked_at`), never removes. Neither table has a retention sweep, and no task 001–022 in `docs/plans/local-node-browser-oauth/plan.md` adds one.
- This is a consequence of a *correct* decision, not a contradiction of it: task 004's STOP trigger forbids `DELETE` as the consumption mechanism ("deletion is indistinguishable from 'never existed' and loses the replay evidence"), and task 022 reuses the terminal `claimed` state for the same reason. Consumption and retention are simply different concerns, and only the first is owned.
- **Failure scenario:** a node left running for years accumulates one row per sign-in attempt; a `browser_oauth_handoffs` full-table scan in `invalidate_pending_handoffs` (`crates/db/src/models/browser_auth/handoff.rs:89-95`, no index on `state`) degrades every disconnect. Disclosure value is near nil — a redeemed verifier is single-use and worthless once Hive marks the handoff redeemed.
- **Minimal remediation:** record it as an accepted residual in `decisions-ledger.md` alongside O8, or open a follow-up workstream for a bounded retention sweep. No phase-1 code change.

---

### F6 `[INFO]` — task 022's frontmatter omits `siblings:`

- **File/line:** `docs/plans/local-node-browser-oauth/phase-1/022-…md:1-19`.
- Tasks 001–005 each carry a `siblings:` list; 022 does not, even though its body does the equivalent work in prose ("**Symbol grounding:** … follows the existing `share_sync_handle()` field/accessor pattern"). The ledger's "Sibling-alignment advisory acknowledgements" section enumerates 001, 008, 009, 011, 013–016, 018–020 — 022 is absent there too.
- **Failure scenario:** a future `wai-plan-lint.sh` run over the phase-1 directory reports an unacknowledged advisory for the one task with no `siblings:` key, and the ledger has no entry disposing of it.
- **Minimal remediation:** add `siblings: ["crates/deployment/src/lib.rs", "crates/db/src/models/browser_auth/session.rs"]` (or record the acknowledgement in the ledger's existing section).

---

## 3. Suspicions I raised and disproved (not filed)

| Suspicion | How it was disproved |
|---|---|
| `install_remote_sync`'s `if slot.is_none()` guard silently drops a startup sync that `spawn_remote_sync` would have installed | At `from_parts` the slot is freshly `Arc::new(Mutex::new(None))` (`crates/local-deployment/src/lib.rs:235`), so the guard is unreachable at startup. The old method's unconditional `*guard = Some(...)` (`crates/deployment/src/lib.rs:117-119`) is equivalent there. |
| The current-thread startup test is a tautology that would also pass under the old detached `spawn_remote_sync` | It would not. Under the pre-022 code there is no yield point between the detached `tokio::spawn` and the test's first `.lock().await`: an uncontended `tokio::sync::Mutex` resolves on first poll without yielding to the scheduler, so on a `current_thread` runtime the slot would still be `None`. It is a real mutation proof. |
| Overwriting a `RemoteSyncHandle` leaks a live orphan task | `RemoteSyncHandleInner::drop` (`crates/services/src/services/share.rs:682-689`) sends shutdown and calls `join.abort()`. Already disproved in the ledger; independently re-confirmed. |
| Moving `ShareConfig::from_env()`/`VK_SHARED_API_BASE` out of `from_parts` changes legacy remote-client behaviour | It does not. `Deployment::new()` (`crates/local-deployment/src/lib.rs:657-660`) computes both values with byte-identical logic to the deleted lines (`option_env!(...).map(String::from)` ≡ `.map(\|s\| s.to_string())`), and `from_parts` consumes them at the same points. `crates/services/src/services/share/config.rs:17-19` reads the same variable. `remote_client` remains driven solely by the raw base, which `raw_api_base_remains_available_when_share_sync_config_is_unavailable` pins with `ftp://example.invalid` + `share_config: None`. |
| Adding a required `Deployment::browser_auth_epoch()` breaks another implementer | `grep -rn 'impl Deployment for' crates/` returns exactly one: `crates/local-deployment/src/lib.rs:573`. `cargo check -p server --all-targets` is clean. |
| `disable_orphan_cleanup_for_tests` omits `DISABLE_WORKTREE_EXPIRED_CLEANUP`, so the new tests can delete real worktrees | Disproved. `spawn_worktree_cleanup` (`crates/local-deployment/src/container.rs:513-540`) reaches `cleanup_expired_attempts` only via `TaskAttempt::find_expired_for_cleanup(&db.pool)` — DB-scoped to the empty test pool, unlike `cleanup_orphaned_worktrees`, which scans the real filesystem base dir. (The first `interval.tick()` *does* fire immediately, so it runs at test start — but finds nothing.) The orphan sweep is the only filesystem-reaching path, and it is guarded. |
| `ON CONFLICT(slot) DO UPDATE SET hive_user_id = hive_user_id` writes on the rejection path, contradicting "on mismatch NOTHING is written" | In SQLite an unqualified column in `DO UPDATE SET` refers to the existing row, so the update is a genuine no-op; `first_subject_pins_and_same_subject_does_not_move_pinned_at` and `different_subject_is_rejected_without_side_effects` both pin `pinned_at == 100`. Nothing observable changes; not filed. |
| `cargo test -p db` red because of the new migration | The migration is `CREATE TABLE IF NOT EXISTS` only, highest-versioned (`ls crates/db/migrations/ \| tail -1`), and the sole failure is in `execution_process`, untouched by this range. |

---

## 4. Fidelity audit

### 4.1 File-set conformance (every commit against its task's `files:`)

| Task | Declared `files:` | Commits | Verdict |
|---|---|---|---|
| 001 | migration, `test_utils.rs` | `7650a425` (+ `reviews/001.approved`) | ✅ — the extra path is `docs/plans/$TOPIC/*`, explicitly exempt per ledger review-time decision 2 |
| 002 | `Cargo.lock`, `auth/mod.rs`, `auth/seams.rs`, `server/src/lib.rs`, `server/Cargo.toml` | `6425a16c` + `2d8b3aba` | ✅ — lockfile amendment recorded before validation; `git diff` shows exactly one added line (`+ "base64"` under the `server` package) |
| 003 | `browser_auth/mod.rs`, `owner.rs`, `models/mod.rs` | `54936598` | ✅ exact |
| 004 | `handoff.rs`, `browser_auth/mod.rs` | `acfbf691` | ✅ exact |
| 005 | `session.rs`, `browser_auth/mod.rs` | `7c8cf142` | ✅ exact |
| 022 | `handoff.rs`, `browser_auth/mod.rs`, `deployment/src/lib.rs`, `local-deployment/src/lib.rs` | `6eece603`, `a32804bc`, `cc70f9d7`, `94e5aecc`, `53a962b8` | ✅ — all five commits stay inside the four declared paths |

### 4.2 Contract text fidelity

- **Task 001:** the migration file is **byte-identical** to the task's single ```sql``` block (verified programmatically). The `test_utils.rs` test is byte-faithful to `001-…md:24-53`.
- **Task 002:** the five colocated tests, `Clock`/`SystemClock`/`FixedClock`, `TokenSource`/`OsTokenSource`/`ScriptedTokenSource` and free-function `hash_token` match the declared interface. `hash_token` (`seams.rs:77-86`) is byte-for-byte the same encoding as `crates/server/src/routes/oauth.rs:221-230` (`hash_sha256_hex`) — the task's STOP trigger. Fakes are unconditionally compiled, as required for `crates/server/tests/`. `base64 = "0.22"` matches `crates/services`/`crates/remote`. `rand 0.9` API (`rand::rng()` / `rng.random()`) is correct; 32 bytes base64url-unpadded is 43 chars, pinned by test.
- **Tasks 003/004/005:** SQL text, function signatures, doc comments and every test body match their contracts verbatim modulo `rustfmt` wrapping. Runtime query forms only — no `query!`/`query_as!` macro under `crates/db/src/models/browser_auth/`, so no `.sqlx` cache entry is needed.
- **Task 022:** all four prescribed edits landed exactly as specified. `invalidate_pending_handoffs` reuses the existing terminal `claimed` state (no third state, no `DELETE`, no schema change); `browser_auth_epoch` is one per-deployment `Arc<Mutex<u64>>` (not a process-global static), proven clone-shared; `install_remote_sync` sits immediately after `spawn_remote_sync`, which is unmodified. The contract's snippet writes `let pool = create_test_pool().await;` — the implementation correctly destructures the `(pool, TempDir)` tuple; that is a contract typo, not a deviation.

### 4.3 STOP triggers

All six tasks' STOP triggers hold. Specifically for 022: no migration/schema change; no third handoff state; no `DELETE`; `spawn_remote_sync` unchanged and the epoch never taken inside it; the epoch is per-deployment, not static; **and no OAuth route was edited** — `crates/server/src/routes/oauth.rs` is not in the diff at all.

### 4.4 Does task 022 close the disconnect/login race?

It closes exactly what it is contracted to close, and the range is honest about the rest.

- **Closed here — the startup variant.** `crates/local-deployment/src/lib.rs:493-495` now `await`s `install_remote_sync` before `from_parts` returns, so a served deployment can never present an empty sync slot to a concurrent disconnect. `crates/deployment/src/lib.rs:125-135` is the synchronous installer. `configured_startup_sync_is_installed_before_constructor_returns` is a genuine mutation proof (see §3).
- **Landed here — the primitives.** Durable `invalidate_pending_handoffs`, the shared epoch, and the synchronous installer.
- **Deliberately NOT closed here — the route wiring.** `crates/server/src/routes/oauth.rs:143-157` still calls the detached `spawn_remote_sync` after login, and `:167-186` (`logout`) still has no epoch bump, no `invalidate_pending_handoffs`, and no `refresh_guard`. SC8 is therefore still open at HEAD — **correctly so**: `022-…md:183` says "Do not edit any OAuth route in this task; tasks 009-012 consume these primitives", manual-verification item 4 (`022-…md:200`) requires recording that "tasks 009-012 must still wire the epoch/invalidation into real routes before SC8 is complete", and the ledger's "Integrated phase-1 review" section states the remediation is "corrective task 022 **plus amended tasks 009–012**". Reporting the open route race as a phase-1 defect would be wrong, and I am not doing so.
- **Legacy remote-client configuration behaviour is unchanged** — see §3, verified against `crates/services/src/services/share/config.rs:16-61`.
- **Lock ordering is consistent across the forward contracts.** Task 011 (`epoch` → `install_remote_sync` → `share_sync_handle`) and task 012 (`epoch` → `share_sync_handle`) both acquire `browser_auth_epoch` before `share_sync_handle`; no inversion is planned, so the fence does not introduce a deadlock. *(Phase-3 plan observation only, not a phase-1 defect: task 012 holds the epoch guard across `client.logout()` network I/O, which will block all OAuth initiation and claim for the duration of a disconnect. Worth a sentence in the ledger when 012 is executed.)*
- **Manual-verification item 4** is not recorded as its own ledger bullet under "Task 022"; the substance is carried by the immediately preceding "Integrated phase-1 review" section ("Remediation is corrective task 022 plus amended tasks 009–012"). I checked and consider this satisfied — noted for completeness, not filed.

### 4.5 Accepted residuals — all present and explicit

- **O8** (SQLite revoke-all and file/Keychain credential clear cannot share a transaction; a crash between them leaves an over-locked-out node) — `decisions-ledger.md`, "Integrated phase-1 review" section, and `session.rs:73-78`.
- **In-memory epoch resets on restart** — covered durably by `invalidate_pending_handoffs`; rationale (no approved schema for a durable generation) recorded.
- **Post-invalidation handoffs are indistinguishable from redeemed ones** — an accepted consequence of reusing `claimed`, stated in `handoff.rs:83-88`.

### 4.6 Success criteria

Phase-1 tasks declare `covers_criteria: []` throughout; only 004 claims `covers_tests: ["TS1"]`. Its five tests do cover TS1's exact-expiry, wrong-browser-non-consumption, concurrent-single-claim and replay clauses; TS1's owner-pin-race, session-persistence and revocation clauses are carried by 003/005 as supportive checks, and hash-only persistence is split forward to task 009 (`decisions-ledger.md`, "Task 004", dismissal 2). That accounting is consistent with `plan.md`'s "Coverage ownership" line. No phase-1 task over-claims an SC.

---

## 5. Verdict rationale

The browser-auth work itself is high quality and I found no correctness, security, concurrency, durability or compatibility defect in it. Claim/invalidation is genuinely single-statement and terminal; the epoch is correctly clone-shared and per-deployment; the startup install race is really closed and really proven; the constructor refactor preserves legacy remote-client configuration exactly; the migration is additive, highest-versioned and byte-identical to its contract; `crates/server` — never re-checked after a required trait method was added — compiles clean; `cargo fmt` is clean.

I am rejecting on F1 alone: the crate's own test suite is reproducibly red at `ae5ee15f`, and the tracking artifact this range committed a promise to create does not exist anywhere in the repository. The remediation touches no phase-1 source. F2 should be fixed in the same pass — it is one line, and it is the unexamined half of the very hazard class task 022 was remediating.

**VERDICT: REJECT**