# Decisions Ledger — vk-swarm-node-ui-localize

> Implementer appends here for ANY choice the task didn't dictate. Empty section = perfect.

## Pre-existing decisions (from spec + ADRs)

| Decision | Source | Reversible? |
|----------|--------|-------------|
| D1: Restore node-surface routes as thin RemoteClient proxies, verbatim paths | spec / ADR-0013 | Irreversible |
| D2: Proxy through the node server, not browser→hive direct | spec / ADR-0013 | Irreversible |
| D3: Do not restore `/nodes/api-keys*`; delete the node's API-key UI | spec / ADR-0013 | Irreversible |
| D4: `ProjectWithStats` at `/api/projects/with-stats` replaces `MergedProject` | spec / ADR-0014 | Irreversible |
| D5: Delete `LocationBadges` and the `nodes`/`has_local`/`local_project_id` fields | spec / ADR-0014 | Irreversible |
| D6: Keep the four remote stream hooks; harden rather than remove | spec | Reversible |
| D7: Node and hive `components/swarm/` trees stay separate | spec | Reversible |

## Decomposition-time decisions (dictated, not left to the implementer)

- **Rust tasks verify via `## Manual verification`, not `scope_test`.** The gate's `scope_test`
  runner is toolchain-detected for Python/Node and would run vitest against a Rust path. TS tasks
  use real `scope_test` paths (`frontend/` has vitest + 34 test files).
- **`forbid_after` is omitted on tasks 202 and 303**, with the reason recorded in each task file.
  It greps every tracked file, and the obvious terms have legitimate survivors: `/merge-to/` in
  `crates/remote/` and `remote-frontend/` (the hive's, SC7), and `merged-projects` /
  `/api/nodes/api-keys` in ADRs, specs, and `docs/architecture/`. Scoped greps are used instead.
- **`ProjectWithStats` field types are copied from `MergedProject` verbatim** —
  `github_open_issues: i32`, `github_open_prs: i32`. The spec's illustrative sketch shows
  `Option<i64>`; its governing sentence ("identical to today's `MergedProject` minus `nodes`,
  `has_local`, `local_project_id`") is authoritative, so the real field types win. Not a spec
  contradiction — the spec's own prose resolves it, so no re-precheck was triggered.
- **Task 105 offers no handler-level unit test on purpose.** A test calling a restored handler
  directly would pass on `main` today, before any task runs, because the handler was never broken
  — registration was. The realest available seam is an HTTP request to a running server.

## Implementer decisions

_(empty — the implementer appends here)_

## Advisory sibling warnings (plan-lint W: lines) — adjudicated at decompose

Each `W:` line from `wai-plan-lint.sh` is acknowledged below. None was a real pattern sibling;
the real sibling in every case is already listed in the task's `siblings:` field.

| W: on | Suggested sibling | Verdict |
|---|---|---|
| 101–104 | `crates/server/src/routes/all_tasks.rs` | **Not a sibling.** It is a local-DB query router, not a `RemoteClient` proxy. The genuine pattern sibling is `crates/server/src/routes/organizations.rs` — the only live hive-proxy router in the crate — and it IS listed in `siblings:` on all four tasks, with a required read step. |
| 301 | `crates/server/src/routes/projects/handlers/core.rs` | **Not the sibling.** `with_stats.rs` is a near-copy of `merged.rs` (same query, same mapping, same sort), which is listed in `siblings:` with a required read step. `core.rs` holds unrelated CRUD handlers. |
| 302 | `frontend/src/hooks/index.ts` | **Not a sibling** (a barrel file, not a pattern). It IS a hazard, and the task handles it explicitly: a STOP trigger fires if `index.ts` re-exports `useMergedProjects`, because that file is outside `files:`. |
| 302 | `frontend/src/components/projects/CloneProgress.tsx` | **Not a sibling.** Unrelated component; the new file is a test for `ProjectList`. |
| 402 | `frontend/src/components/swarm/MergeLabelsDialog.tsx` | **Not a sibling.** A dialog, not a status/empty state. The real sibling is `frontend/src/components/ui/alert.tsx` plus the existing error branch in `SwarmProjectsSection.tsx`, both named in the task with a required read step. |
| 403 | `frontend/src/hooks/index.ts` | **Not a sibling** (barrel file). The pattern sibling is `useRemoteConnectionStatus.ts`, listed in `siblings:` with a required read step. |

## Frozen-spec test divergence — resolved by user decision (2026-07-30)

The codex seat of the breakdown tournament found that tasks 101-104 and 301 had replaced the
frozen spec's required tests ("per-module route tests … against a mocked `RemoteClient`" and the
`ProjectWithStats` enrichment test) with manual curl evidence, because no test in this repo can
build a `DeploymentImpl`. Per ADR-0001 the run did not amend the spec; the contradiction was
escalated.

**User decision: build the seam.** New task **100** creates
`crates/server/tests/common/mod.rs` (`HiveHarness`) from material that already exists —
`wiremock` in `crates/services`' dev-deps, `serial_test` and `db` test-utils already in
`crates/server`'s, and the `VK_SHARED_API_BASE` / `VK_DATABASE_PATH` env overrides — **without
touching production code**. Tasks 101-104 and 301 now depend on it and carry the tests the spec
asks for; task 105 remains as the end-to-end complement (and the deploy evidence).

Task 100 STOPs rather than refactoring `LocalDeployment` if `Deployment::new()` cannot be driven
from a test. The fallback (a `test-utils` feature on `crates/local-deployment` exposing a minimal
constructor) changes production types and would be authored as its own reviewed task.

## Task 100

- [Task 100 orchestrator] **Amendment A — the harness serves a real bound listener instead of
  `tower::ServiceExt::oneshot`.** Attempt 1 failed to compile with
  `error[E0599]: no method named 'into_inner' found for struct 'IntoMakeService<S>'`. The only
  public router constructor is `pub async fn router(deployment: DeploymentImpl) ->
  IntoMakeService<Router>` (`crates/server/src/routes/mod.rs:38`), which ends in
  `.into_make_service()` (`:78`). `IntoMakeService` is a MakeService, not a `Service<Request>`, so
  `oneshot` cannot drive it and there is no public accessor for the inner `Router`. The two
  alternatives were rejected: hand-assembling a `Router` in the harness would make tasks 101-104's
  registration tests pass *by construction* (hiding the exact missing-`.merge()` bug this
  workstream fixes), and adding a production accessor would violate this task's no-production-change
  rule. The harness now binds `127.0.0.1:0` and serves the real `server::routes::router(...)`
  exactly as `crates/server/src/main.rs:207,273` does, driving it over HTTP with `reqwest` (already
  a direct dependency, `crates/server/Cargo.toml:47`). This is strictly stronger than `oneshot` for
  the reachability gate: it exercises the true production entry point, route registration included.
  The dictated public surface (`configured`/`hive_absent`/`mock_json`/`get`/`post`/`Resp`) is
  unchanged, so tasks 101-104 and 301 are unaffected. — files: phase-1/100 task file
- [Task 100 orchestrator] **A.1** `std::env::set_var`/`remove_var` require `unsafe` blocks in
  edition 2024 (`error[E0133]`). — files: phase-1/100 task file
- [Task 100 orchestrator] **A.2** wiremock matchers compose with `.and(...)`; attempt 1 used
  `.expect(path(...))`, which takes a call-count
  (`error[E0277]: the trait bound 'Times: From<PathExactMatcher>' is not satisfied`). — files: phase-1/100 task file
- [Task 100 orchestrator] **A.3 — `hive_absent()` must assert `deployment.remote_client().is_err()`.**
  `VK_SHARED_API_BASE` resolves via `std::env::var(...).or_else(|| option_env!(...))`
  (`crates/local-deployment/src/lib.rs:188-190`), and `option_env!` is baked at COMPILE time by
  `crates/server/build.rs:10-12`, which calls `dotenv::dotenv()`. Removing the env var at runtime is
  therefore not sufficient: with `VK_SHARED_API_BASE` set in the environment or uncommented in
  `.env` (`.env:95-96`, both currently commented) at build time, `hive_absent()` would silently
  build a CONFIGURED deployment and every hive-absent assertion in tasks 101-104 and 402 would pass
  while testing the wrong thing. The assertion converts that silent false-green into a loud
  failure. — files: phase-1/100 task file
- [Task 100 orchestrator] `Deployment::new()` is a TRAIT method
  (`crates/deployment/src/lib.rs:77`, impl at `crates/local-deployment/src/lib.rs:101`), so the
  harness needs `use deployment::Deployment;` in scope. Recorded because the task's original
  construction sequence did not say so. — files: phase-1/100 task file
- [Task 100 orchestrator] `Cargo.lock` added to the task's `files:` — cargo rewrites it as an
  unavoidable byproduct of adding two dev-dependencies, and the Stage-1 gate enforces the file set.
  — files: phase-1/100 task file

### Task 100 — ESCALATION: the frozen spec's `200 + success:true` proxy test is unreachable

Found by the expedited Opus review of Amendment A; independently verified.

The spec's `## Test strategy` requires: *"Per-module route tests for each restored proxy:
hive-configured returns `200` + `success: true` (against a mocked `RemoteClient`)"*. This is not
achievable by the env-var mocking task 100 was built on, for **every** proxy in tasks 101-104 and
301 — not just the smoke test:

- `organizations.rs:69-76` → `deployment.remote_client()?` → `client.list_organizations()`
- `remote_client.rs:541-543` → `self.get_authed("/v1/organizations")`
- `remote_client.rs:242-246` → `auth_context.get_credentials().await.ok_or(RemoteClientError::Auth)?`
- credentials come from `credentials_path()` = `asset_dir()/credentials.json`
  (`crates/utils/src/assets.rs:35-36`), and **`asset_dir()` has NO env override** — under
  `debug_assertions` it is hard-wired to `dev_assets/` (`crates/utils/src/assets.rs:6-14`)
- `dev_assets/credentials.json` does not exist

So the request fails at token acquisition and never reaches wiremock: `RemoteClientError::Auth`
→ `StatusCode::UNAUTHORIZED` (`crates/server/src/error.rs:168`). The test observes **401**, not
200. Pointing `VK_SHARED_API_BASE` at a wiremock server is NOT sufficient for any OAuth-authed
endpoint.

The spec's parenthetical "(against a mocked `RemoteClient`)" is also unachievable as literally
written: `RemoteClient` is a concrete `pub struct` (`crates/services/src/services/remote_client.rs:155`),
not a trait, so it cannot be mocked without a production change.

Per ADR-0001 the run did NOT amend the spec. Escalated to the user.

**User decision (2026-07-30): Option A — add a `VK_ASSET_DIR` override.** Chosen over three
alternatives: (B) seeding the real `dev_assets/credentials.json` with backup/restore — rejected as
a patch that makes the test suite unsafe to run, since it mutates developer state, `Drop` does not
run on panic/SIGKILL, and it races a running dev server; (C) amending the frozen spec to weaken the
assertion to non-404/non-400/non-500 — rejected because the `200` is what proves the proxy actually
FORWARDS, and it would permanently bless "this layer is untestable"; (D) trait-ifying
`RemoteClient` — rejected as both the largest blast radius AND worse testing practice, since
mocking one's own HTTP client stubs out URL construction, serialization, auth-header handling and
error mapping, which is exactly the code most likely to be wrong.

New task **099** (`crates/utils/src/assets.rs`, `.env.example`,
`docs/configuration-customisation/storage-configuration.mdx`) makes `asset_dir()` honour
`VK_ASSET_DIR`. This completes an existing pattern rather than inventing one: `VK_DATABASE_PATH`,
`VK_BACKUP_DIR`, `VK_WORKTREE_DIR` and `VK_LOG_DIR` already exist, and `asset_dir()` was the only
root among them with no override. Task 100 now `depends_on: ["099"]`.

Two defects retired as a side effect: `Deployment::new()` unconditionally rewriting the
developer's `dev_assets/config.json` on every test run
(`crates/local-deployment/src/lib.rs:133`), and release-mode instances being unable to hold
separate state (a single `ProjectDirs` path).

**Mechanism substitution recorded, NOT a spec amendment.** The spec's Test strategy says
"(against a mocked `RemoteClient`)". `RemoteClient` is a concrete `pub struct`
(`crates/services/src/services/remote_client.rs:155`), so that literal mechanism has no seam. The
harness mocks at the HTTP boundary with `wiremock` instead. The spec's ASSERTION — `200` +
`success: true` — is met exactly; only the parenthetical mechanism differs, and the HTTP-boundary
fake is the stronger of the two. The spec file was NOT edited, so ADR-0001's freeze holds and no
re-`/wai:precheck` is required.

- [Task 100 orchestrator] **Amendment B** — `configured()` sets `VK_ASSET_DIR` to its `TempDir`,
  seeds `credentials.json` with `{"refresh_token":"test-refresh-token"}` (the on-disk
  `StoredCredentials` shape, `oauth_credentials.rs:25-28`), and registers a mock for
  `POST /v1/tokens/refresh`. The refresh mock is REQUIRED and easy to miss: stored credentials
  carry no access token, so `Credentials::from` yields `access_token: None` and `expires_soon()`
  returns `true` via its `_ => true` arm (`oauth_credentials.rs:16-23,30-38`) — every authed
  request therefore takes the refresh path (`remote_client.rs:330-340`) before reaching its real
  endpoint. — files: phase-1/100 task file
- [Task 100 orchestrator] Amendment A.5 (accepting `dev_assets/config.json` pollution) is
  SUPERSEDED by B.1: with `VK_ASSET_DIR` set, config is written inside the TempDir. Task 100's
  Manual verification now asserts `git status --porcelain dev_assets/` is empty. — files: phase-1/100 task file

### Task 099 + 100 — expedited breakdown review round 2 (Opus), findings applied

- **BLOCKER — the `200` was still unreachable, for a second reason.** Amendment B.3 originally
  mocked `"access_token": "test-access-token"`. After a refresh response arrives,
  `refresh_credentials` calls `extract_expiration(&access_token)`
  (`crates/services/src/services/remote_client.rs:312`) →
  `jsonwebtoken::dangerous::insecure_decode::<ExpClaim>` (`crates/utils/src/jwt.rs:22-26`). A
  plain string is not a 3-part JWT → `TokenClaimsError::Decode` → `RemoteClientError::Token` →
  `StatusCode::BAD_GATEWAY` (`crates/server/src/error.rs:175`). The smoke test would have failed
  with **502**. Fixed: the mock now returns a real JWT with a future `exp`, built by a helper
  mirroring the repo's own `make_jwt_with_exp` (`crates/utils/src/jwt.rs:34-48`); signature is
  never verified so the secret is arbitrary, but the structure and `exp` claim are mandatory.
  Adds `jsonwebtoken = { version = "10.2.0", features = ["rust_crypto"] }` to `crates/server`'s
  dev-deps (same version as `crates/utils`, so no new lock entry). Two STOP triggers added:
  502 → the token is not a decodable JWT; 401 → credential seeding did not take effect. Neither
  may be resolved by relaxing the assertion.
- **MAJOR — task 099's `.mdx` insertion had a malformed nested fence.** The block opened
  ```` ```mdx ````, nested a ```` ```bash ````, then emitted a stray unmatched ```` ```bash ````.
  A literal implementer would have pasted a dangling fence into
  `storage-configuration.mdx`. Rewritten as a 4-space-indented literal block with an explicit
  strip-the-indent instruction and a note that the inner ```` ```bash ```` is CONTENT, not a fence.
- **MAJOR — task 100's construction sequence contradicted itself.** Step "1b" referenced
  `temp_dir` one step before step 2 created it, and Amendment B.1 gave a third location
  ("alongside the Amendment A.4 block"). Replaced with a single authoritative 9-step list that
  explicitly supersedes any ordering implied elsewhere in the file, and which also pins the
  `TempDir`/`MockServer` lifetime (dropping the TempDir early would delete the seeded credentials
  out from under the running server).
- **MAJOR — "Amendment A.5 — SUPERSEDED" was a trap.** A literal implementer skips a section
  headed SUPERSEDED, but A.5 is where the `save_config_to_file` rationale lives and it is what
  the `git status --porcelain dev_assets/` verification tests. Renamed to "why `VK_ASSET_DIR` is
  mandatory" and the three conflicting supersession sentences removed.
- **MINOR** — `.env.example` insertion re-labelled `text` (not `bash`) to match its Before block,
  with the trailing blank-line separator stated in prose rather than left as an invisible line.
- **MINOR** — task 100's `git diff --stat crates/utils` check now states that it assumes task 099
  is COMMITTED, not merely applied.

Independently verified as NOT problems before dispatch: `{"organizations": []}` deserializes into
`ListOrganizationsResponse { organizations: Vec<OrganizationWithRole> }`
(`crates/utils/src/api/organizations.rs:55-57`); `post_public` is unauthenticated
(`send(..., false, ...)`, `remote_client.rs:472`) so the refresh call cannot recurse into the auth
check it exists to satisfy; and the refresh mock (POST `/v1/tokens/refresh`) and test mocks
(GET `/v1/organizations`) are disjoint on method+path, so wiremock mount order cannot cause one to
swallow the other.

### Task 099 attempt 1 (`6c31aa1e`) — Stage 1 PASS, Stage 2 REJECT

Implementer executed the task text exactly, empty ledger, gate CONFORMS (3 declared files only).
Two independent Opus challengers then found three real defects **in the task text I wrote**, not
in the implementation. Attempt 2 applies Amendment R1.

- **BLOCKER (panel B) — `VK_ASSET_DIR` missing from `productionOnlyVars`.**
  `scripts/setup-dev-environment.js:307-310` states the deny-list exists so spawned worktree dev
  servers "use their local `dev_assets/` paths"; it lists all four storage leaves at `:313-316`.
  The loop at `:342-347` **actively re-exports** any `VK_*` var not on the list. `VK_ASSET_DIR` is
  the ROOT those leaves default off, so a production value would leak into every worktree dev
  server — and since the script UNSETS `VK_DATABASE_PATH` while forwarding `VK_ASSET_DIR`,
  `database_path()` falls back to `asset_dir().join("db.sqlite")` (`assets.rs:72`) and the dev
  server opens the PRODUCTION database. Fixed in R1.1.
- **MAJOR (panel B) — blank `VK_ASSET_DIR` relocates all state to CWD.** `create_dir_all("")`
  returns `Ok(())`, so the `!path.exists()` guard never fires; every derived path becomes
  CWD-relative, and CWD differs between `cargo watch`, the packaged binary, and `cargo test`.
  Fixed in R1.2 with a `.filter(|s| !s.trim().is_empty())` guard plus a regression test.
  (Precision note: `database_path()`'s existing `!p.as_os_str().is_empty()` filter at
  `assets.rs:65` guards the PARENT directory, not a blank input — the sibling does not actually
  handle this case either, which is why F-2026-07-30-02 remains open for `VK_DATABASE_PATH`.)
- **MAJOR (panel B) — the docs overpromised.** "Set this to run two instances with fully separate
  state" is false: worktrees resolve from `VK_WORKTREE_DIR` or a shared temp dir, NOT from
  `asset_dir()` (`worktree_manager.rs:571-581`). Separate asset dirs ⇒ separate DBs ⇒ each
  instance's startup orphan sweep (`container.rs:319-383`, spawned at `:162`) treats the other's
  live worktrees as orphans and `remove_dir_all`s them. Fixed in R1.3 by qualifying the claim and
  directing users to also set `VK_WORKTREE_DIR`.
- **MINOR (panel A) — `test_asset_dir_tilde_expansion` was hollow.** Proved by a revert
  experiment: with the production hunk removed, only 2 of the 4 new tests failed; this one passed
  vacuously because the default `dev_assets` path is absolute and `~`-free. Fixed in R1.4 with an
  `ends_with` assertion. (Panel A also confirmed by the same experiment that
  `test_asset_dir_env_override_reaches_derived_paths` is NOT hollow — it is the load-bearing test
  and it genuinely pins `credentials_path()`/`config_path()` to the override.)

Pre-existing issues surfaced by the panels and filed rather than fixed in-branch (they predate
this workstream and are outside its scope):
**F-2026-07-30-01** `cargo test -p db` fails to compile (`db::test_utils` is feature-gated and the
crate lacks a self dev-dependency enabling it). Verified NOT to affect C6's gate:
`cargo test --workspace --no-run` exits 0, because feature unification enables `db/test-utils`
across the workspace. Only the isolated per-crate invocation breaks.
**F-2026-07-30-03** (high) the orphan-worktree sweep has no dirty-file guard, unlike
`cleanup_expired_attempt` (`container.rs:390-405`) which does check `get_dirty_files()`.
**F-2026-07-30-04**, **F-2026-07-30-05** minor pre-existing path/registry inconsistencies.

### Task 099 attempt 2 (`12ac5417`) — Stage 1 PASS, focused re-check REVISE

R1.1, R1.3, R1.4 all verified to bite by revert experiment. R1.2 was incomplete.

- **BLOCKING — the R1.2 guard was internally inconsistent.** `.filter(|s| !s.trim().is_empty())`
  tested the trimmed string but passed the UNTRIMMED value to `expand_tilde`
  (`crates/utils/src/assets.rs:17-21`). `VK_ASSET_DIR=" /tmp/foo "` therefore survived the guard,
  and the leading space made the path RELATIVE: the unconditional `create_dir_all` created a
  directory literally named `" "` under the process CWD with the real path nested inside, and
  config/credentials/database landed there silently. Same failure class R1.2 was written to kill,
  one space away. Fixed in R2.1 by trimming before use, with a regression test.
- **NIT — the deny-list comment asserted a false constraint.** It said `VK_ASSET_DIR` "must be
  unset first"; the script emits every `unset` before any `export`, so array position is
  immaterial. Corrected in R2.2 to state the real reason (it would otherwise be re-exported and
  become the dev server's default database).

Verified NOT defects: the deny-list diff is surgical — a full before/after `env`-output diff with
a representative environment showed exactly two lines changed (`+unset VK_ASSET_DIR`,
`-export VK_ASSET_DIR=...`) with `VK_SQLITE_MAX_CONNECTIONS` and friends still forwarded, so no
collateral unsets. `node --check` clean. The `.mdx` sentence sits inside the `<ParamField>` block
and its claims are true (worktrees resolve from `worktree_manager.rs:571-581`, and
`cleanup_orphaned_worktrees` at `container.rs:319-383` does delete other instances' worktrees).
Gates green at attempt 2: `cargo test -p utils` 136 passed, clippy clean, `cargo fmt --check` clean.

### Task 099 attempt 3 (`f81c4da6`) — PASSED

Stage 1 CONFORMS (2 declared files). R2.1 and R2.2 applied verbatim, empty ledger.
Orchestrator revert experiment confirms the new test is NOT hollow: with
`.map(|s| s.trim().to_string())` removed, `test_asset_dir_env_override_is_trimmed` FAILS —
`left: "  /tmp/.tmp1ZOpZV/assets  "  right: "/tmp/.tmp1ZOpZV/assets"`. Restored; `crates/` clean.
`cargo test -p utils --lib assets` 12 passed; clippy clean; `cargo fmt --check` clean.

Task 099 status → passed. Three attempts, every rejection traceable to the TASK TEXT rather than
the implementer (empty ledger on all three attempts) — the decompose-time breakdown review cannot
see these because they are properties of the surrounding system (a deny-list in a build script, a
compile-time `option_env!` fallback, a JWT decode on a mocked value), not of the task's own prose.

### Task 100 — Stage 2 REJECT at attempt 1 (`f9eacc29`), PASS at attempt 2 (`abf94dde`)

The most consequential finding of the run. The harness was built faithfully — construction
sequence exactly as dictated, anti-fake greps clean, real `server::routes::router()`, no
production code touched, and the `200` genuinely real (removing the `/v1/organizations` wiremock
stub makes the test fail; `received_requests()` shows `POST /v1/tokens/refresh` +
`GET /v1/organizations` actually traversed, proving the task-099 credential seam works). But it
could not do the one thing it exists for.

**Decisive experiment (throwaway probe through the harness):**

```text
PROBE /api/nodes?organization_id=000...   => 200 :: <!DOCTYPE html>
PROBE /api/swarm/projects                 => 200 :: <!DOCTYPE html>
PROBE /api/definitely-not-a-route         => 200 :: <!DOCTYPE html>
PROBE /api/organizations                  => 200 :: {"success":true,...}
```

**Root cause:** the outer router ends in `.route("/{*path}", get(frontend::serve_frontend))`
(`crates/server/src/routes/mod.rs:76`), and `serve_frontend` returns `StatusCode::OK` with
`index.html` for unknown routes (`crates/server/src/routes/frontend.rs:40-43`, commented "For SPA
routing, serve index.html for unknown routes").

**So `assert_ne!(status, 404)` proves NOTHING about registration** — it passes on `main` today for
routes that do not exist. That was the assertion pattern in tasks 101, 102, 103, 104, 105, 303 and
501: essentially every registration check in the workstream would have been a false green on
exactly the bug being fixed.

**Corrections applied across seven task files:**
- Task 100 Amendment C: `Resp::content_type`, `Resp::is_spa_fallback()`,
  `Resp::assert_registered()`, plus a third smoke test `harness_detects_an_unregistered_route`
  that asserts `/api/health` is NOT the fallback and `/api/definitely-not-a-route` IS — the
  meta-test that keeps the primitive honest for the rest of the workstream.
- Tasks 101-104: `assert_ne!(404)` → `res.assert_registered()`, on BOTH the configured and
  absent-hive tests, each with a warning banner explaining why.
- Task 105: the curl sweep now discriminates on **content-type** (`application/json` = registered,
  `text/html` = SPA fallback), not status code.
- Tasks 303 and 501: their `/api/merged-projects` → **404** expectation was not merely weak but
  WRONG — after deletion that path falls to the catch-all and returns 200 + `text/html`, so the
  assertion would have FAILED a correct implementation.

**Spec imprecision recorded (no spec edit — ADR-0001).** The spec describes the symptom as "404".
At HTTP level it is 200 + `text/html`; the user-visible failure is the node UI receiving HTML where
it expects JSON. The spec's "returns non-404" is *insufficient* rather than contradicted, so the
tasks satisfy its letter while asserting something stronger — additive, not an amendment, so no
re-`/wai:precheck` is required. Task 501 records the correction. Recommend fixing the spec wording
at `/wai:close`.

Also fixed at attempt 2: `cargo fmt --all -- --check` was RED at attempt 1
(`common/mod.rs:73`, `:128`, `harness_smoke.rs:4`) — a C6 gate left failing.

Noted, not blocking: one `/api/organizations` call produces THREE upstream `GET /v1/organizations`
requests plus the refresh. Not retry logic (`should_retry`, `remote_client.rs:82-88`, only retries
transport/5xx). Matters only if a later task mounts a call-count-bounded mock — none do, since
Amendment A.2 forbids `.up_to_n_times(...)`.

**Process note:** these task-file corrections were written once, lost to a subagent's
"restore the working tree" cleanup (`git checkout` over uncommitted `docs/plans/` edits), and
rewritten. Commit plan-doc edits promptly rather than batching them behind a task's code commit.

## Task 101

Implemented clean at `c01e18cc` (empty implementer ledger, Stage-1 CONFORMS). Orchestrator
verification: removing `.merge(nodes::router())` makes BOTH tests in
`crates/server/tests/nodes_routes.rs` fail with
`route is NOT registered: request fell through to the SPA catch-all (status 200, content-type
Some("text/html"))` — the registration primitive genuinely catches this workstream's bug.
Panel independently confirmed the configured-hive test is not hollow (removing the
`/v1/nodes` mock → `left: 404 right: 200`).

Panel verified as verbatim: `diff` against `35b378a5^:crates/server/src/routes/nodes.rs` contains
ONLY the dictated API-key deletions (3 structs, 3 handlers, 2 router lines) and the dictated import
fix. Zero other hunks. `delete_node` survives and is reachable via
`get(get_node).delete(delete_node)`. No api-key residue. Frontend contract for the three restored
routes matches field-by-field (`frontend/src/types/nodes.ts:18-42` vs
`crates/remote/src/nodes/domain.rs:102-152`).

- [Task 101 orchestrator] **Sibling divergences from `organizations.rs`, recorded here because the
  task required it and the implementer filed none.** All three are inherited verbatim from the
  pre-deletion original, so none is new drift: (a) `nodes.rs` handlers are `pub async fn`, the
  sibling's are private; (b) `nodes.rs` chains methods on one `.route()`
  (`get(get_node).delete(delete_node)`), the sibling uses one `.route()` per method; (c) —
  the substantive one — `nodes.rs` imports wire types from the `remote` crate
  (`use remote::nodes::{Node, NodeLocalProjectInfo};`) whereas `organizations.rs` uses the shared
  `utils::api::*` contract types. (c) couples the server route layer to `crates/remote` domain
  types; accepted for a verbatim restore, flagged as a candidate for a later unwind.
- [Task 101 orchestrator] `pub mod nodes;` landed at `routes/mod.rs:29` (after `organizations`)
  rather than the dictated anchor before `pub mod projects;`. The block is otherwise alphabetical,
  so `nodes` belongs between `message_queue` and `oauth`. Functionally identical; task 102 is
  directed to correct the ordering while it edits the same file.
- [Task 101 orchestrator] **Task 105's D3 assertion corrected as a consequence of 101.** Now that
  `/nodes/{node_id}` (`Path<Uuid>`) is registered, the literal path `/api/nodes/api-keys` MATCHES
  it and fails UUID parsing. Empirically: `/api/nodes/api-keys?organization_id=...` → **400
  text/plain** `"Invalid URL: Cannot parse 'node_id' with value 'api-keys'"`, while
  `/api/nodes/api-keys/x/unblock` → 200 text/html. The previous "expect text/html" assertion would
  have FAILED. 105 now uses `git grep` over `crates/server/src/routes/` as the primary D3 proof and
  records the HTTP behaviour accurately.
- [Task 101 orchestrator] Frontend still calls the removed API-key endpoints
  (`frontend/src/lib/api/nodes.ts:40-66`) plus `mergeNodes` (`:80`), which never existed even at
  `35b378a5^`. Correctly out of scope for 101 — task **202** removes them. This branch must not
  merge with 101 alone.

## Task 102 — restore swarm_projects routes

**Implementer ledger: empty.** No undictated choices.

**Pre-dispatch amendments (ORCHESTRATOR).** Three defects in MY task text, fixed before dispatch:
- **D1** — the drafted mock body `json!([])` cannot deserialise into `ListSwarmProjectsResponse`,
  which is a struct with a `projects` field (`crates/remote/src/routes/swarm_projects.rs:90-92`).
  Corrected to `json!({"projects": []})`. The panel's revert experiment C confirms this was
  load-bearing: with the old body the endpoint returns `400 "Unexpected response from remote
  service."`, not 200.
- **D2** — documented (not changed) that mocking `/v1/swarm/projects` is correct: the harness seeds
  OAuth credentials so `RemoteClient::list_swarm_projects` takes the `AuthMode::OAuth` arm
  (`crates/services/src/services/remote_client.rs:1014-1017`), and `wiremock::matchers::path`
  ignores the query string (`crates/server/tests/common/mod.rs:170-176`).
- **D3** — dictated the `pub mod nodes;` alphabetical correction that task 101's panel flagged, since
  this task already edits the same declaration block. Not a ledger-worthy implementer choice.

**Implementer verification claim was FALSE — process lesson.** The implementer reported
`cargo fmt --all -- --check` as "(nightly config warnings only — no actual formatting errors)".
The real exit code was **1**. The paraphrase hid it; the earlier session had also been reading
`$?` after a `| tail`, which reports *tail's* status, never cargo's. Two files had drift:
`crates/server/tests/swarm_projects_routes.rs` (this task) and
`crates/server/tests/nodes_routes.rs` (**task 101, already committed red**).

Both fixed in-session per CLAUDE.md "No Deferred Remediation"; 101's fix committed separately as
`98bff5be` so task 102's Stage-1 file-set gate stayed clean.

**Standing correction for the remainder of this run:** implementer verification output is a
CLAIM, not evidence. The orchestrator re-runs every check itself before gating, and captures real
exit codes (`cmd > file 2>&1; echo $?`), never `cmd | tail` followed by `$?`.

**Stage 2 — adversarial panel (opus): NO FINDINGS.** Three destructive-negative experiments, all
confirming the tests are load-bearing rather than hollow:
- **A. Unregister** — removing `.merge(swarm_projects::router())` fails BOTH tests with
  `route is NOT registered: request fell through to the SPA catch-all (status 200, content-type
  Some("text/html"))`.
- **B. Mock never matches** — repointing the mock to `/v1/never/matches` yields `left: 404`,
  proving the mock is actually exercised.
- **C. Deserialisation** — the pre-D1 `json!([])` body yields `left: 400`, proving the test
  exercises real `ListSwarmProjectsResponse` deserialisation, not just a status code.

Verbatim file confirmed by md5 (`af744d5938e1b16cce77ad14b5df62cd` both sides). No sibling
divergence from `organizations.rs`/`nodes.rs`. No route shadowing on `/swarm/*`. No scope creep —
`routes/mod.rs` diff is exactly the 4 lines D3 + step 2 dictated. Gates green with REAL exit codes:
fmt 0, clippy 0, check 0, `cargo test -p server` 0.

## Task 103 — restore swarm_labels routes

**Pre-dispatch amendments (ORCHESTRATOR).**
- **D1** — the drafted mock body `json!([])` cannot deserialise into `ListSwarmLabelsResponse`, a
  struct with a `labels` field (`crates/remote/src/routes/swarm_labels.rs:88-90`). Corrected to
  `json!({"labels": []})`. Same defect class as task 102 D1; caught before dispatch this time.
- **D2** — documented that `RemoteClient::list_swarm_labels`
  (`crates/services/src/services/remote_client.rs:1254-1262`) requests
  `/v1/swarm/labels?organization_id=...` unconditionally, with NO `AuthMode` branch — unlike
  `list_swarm_projects`. Mocking `"/v1/swarm/labels"` matches because `wiremock::matchers::path`
  ignores the query string.
- **D3** — `pub mod` block already alphabetised by task 102; insert only.
- **D4** — explicit instruction to capture the REAL `cargo fmt --all -- --check` exit code, added
  in response to task 102's false verification claim.

**Implementer deviation — CAUGHT AND CORRECTED (one retry within the haiku rung).** The
implementer alphabetised the `.merge(...)` chain, placing `.merge(swarm_labels::router())` BEFORE
`.merge(swarm_projects::router())`, contradicting step 2's dictated task-order placement — and
declared `LEDGER: empty` while having made exactly the kind of undictated choice the ledger
exists to record.

Resolution: the CODE was corrected, not the task text (never amend the contract to make a run
pass — ADR-0001's principle applied at task level). The implementer then re-declared the ledger
honestly. Functional impact was nil (`/swarm/labels*` and `/swarm/projects*` are disjoint
prefixes), but "harmless" is not "dictated", and an empty-ledger claim that hides a deviation is
the exact failure mode the two-stage design targets.

**Convention now explicit for tasks 104+:** the `pub mod` block is ALPHABETICAL; the
`.merge(...)` chain is TASK-ORDERED. They differ deliberately. Task 104 must not "fix" either.

**Stage 2 — adversarial panel (opus): NO FINDINGS.** md5 `ced2f55024c04235dc20ccea783281ce` both
sides confirms verbatim restore. Three destructive-negative experiments, each with a DISTINCT
failure signature:
- **(a) Unregister** → both tests fail at `common/mod.rs:38` (`Resp::assert_registered`), on the
  SPA-fallback assertion rather than a status code.
- **(b) Mock repointed** to `/v1/never/matches` → `"Remote service error. Please try again."`
- **(c) Pre-D1 body** `json!([])` → `"Unexpected response from remote service."`
  Distinct messages in (b) vs (c) prove the two failure modes are separable, so the test
  discriminates "mock not reached" from "response did not deserialise".

**Route shadowing probed empirically (angle 6).** `swarm_labels.rs` registers both
`/swarm/labels/promote` and `/swarm/labels/{label_id}`. The panel POSTed `/api/swarm/labels/promote`
against a stub at `/v1/swarm/labels/promote` and got the DESERIALISE error, not the
mock-not-matched error — proving `promote_to_swarm` actually executed and reached the hive. The
`/api/nodes/api-keys` vs `/nodes/{node_id}` shadowing class of bug (found earlier in this run) does
NOT occur here: `promote` is POST-only and `{label_id}` has no POST route.

Gates green with REAL exit codes: fmt 0 (0 `Diff in` lines), clippy 0, check 0,
`cargo test -p server` 0 (66 passed). Scope clean — `git show HEAD --stat` is exactly the 3
allowlisted files, `162 insertions(+), 0 deletions(-)`.

## Task 104 — restore swarm_templates routes (phase 1 route restores COMPLETE)

**Implementer ledger: empty — and verified genuinely empty** (unlike task 103's false empty).
Both dictated orderings were followed on the first attempt.

**Pre-dispatch amendments (ORCHESTRATOR).**
- **D1** — `json!([])` → `json!({"templates": []})`; `ListSwarmTemplatesResponse` has a `templates`
  field (`crates/remote/src/routes/swarm_templates.rs:82-84`). THIRD consecutive task with this
  defect in my drafted text — see the process note below.
- **D2** — `RemoteClient::list_swarm_templates`
  (`crates/services/src/services/remote_client.rs:1327-1335`) requests
  `/v1/swarm/templates?organization_id=...` unconditionally; no `AuthMode` branch.
- **D3** — stated the two-orderings rule explicitly (alphabetical `pub mod`, task-ordered
  `.merge(...)`) because that is precisely what task 103's implementer got wrong. It worked: 104
  got both right without a retry.
- **D4** — REAL `cargo fmt` exit code required.
- **D5** — NEW: "the ledger is not empty by default". Instructs the implementer to diff its own
  change against the task text before declaring empty. Added in response to 103's false claim.

**Process note — a decompose-time defect I made three times.** Tasks 102, 103 and 104 all drafted
`json!([])` as the mocked hive response. Every one was wrong, because each `list_*` method returns a
single-field wrapper struct, not a bare array. The decomposer cannot see this without opening
`crates/remote/src/routes/*.rs` for each module — it is exactly the class of defect the frozen spec
and the task rubric cannot catch, and it was caught only by pre-dispatch verification. **Convention
for any future proxy-test task: read the `RemoteClient` method AND its return type's struct
definition before drafting the mock body.**

**Stage 2 — adversarial panel (opus): NO FINDINGS.** md5 `d9390286823cb814f6f5da8ad1ba3e35` both
sides, byte counts equal (3958) so no EOF normalisation. Three revert experiments, (b) and (c)
yielding DISTINCT messages, proving the test separates unreached-mock from failed-deserialisation.

**Integrated review of all four restored modules (the panel's angle 6):**
- Full registered path set has no exact duplicates and no collision with pre-existing routes. The
  only near-neighbours, `projects/mod.rs:94 "/unlink-swarm"` and `tasks/mod.rs:51
  "/available-nodes"`, sit under different nest prefixes.
- A matchit path conflict panics at router construction; every harness test builds the FULL router
  and all pass, empirically ruling out that class of bug.
- Static-vs-dynamic: the only such sibling pair is `/swarm/labels/promote` (POST, swarm_labels.rs:125)
  vs `/swarm/labels/{label_id}` (get/patch/delete only, :119) — no POST on the dynamic route, so
  `promote` is reachable on method grounds in addition to matchit's static-over-dynamic priority.
  The `/api/nodes/api-keys` vs `/nodes/{node_id}` bug found earlier cannot recur: that path no
  longer exists in `nodes.rs` (D3 of the spec deletes the node API-key surface).
- All four modules obtain the client via `deployment.remote_client()?` and return
  `Ok(ResponseJson(ApiResponse::success(..)))`, matching sibling `organizations.rs`. No divergence.
- All four test files are structurally identical; only the mocked body differs, correctly per
  response type: nodes `json!([])`, labels `{"labels":[]}`, projects `{"projects":[]}`,
  templates `{"templates":[]}`.

**Whole-workspace gates green with REAL exit codes:** `cargo fmt --all -- --check` 0,
`cargo clippy --all --all-targets --all-features -- -D warnings` 0, `cargo test --workspace` 0
(server lib 217 passed; all four route suites 2 passed each). Scope clean: 3 files,
151 insertions(+), 0 deletions.

## Task 105 — end-to-end reachability evidence (PHASE 1 COMPLETE)

**Executed by the ORCHESTRATOR, not a constrained implementer.** The task changes no source; its
deliverable is evidence gathered from a really-running server, which requires judgement about
process safety (a foreign instance was running) that the constrained implementer role forbids.

**ORCHESTRATOR addition beyond the task text — the NEGATIVE CONTROL.** The task asked only for six
paths to return `application/json`. That is insufficient on its own: six JSON responses are equally
consistent with "this binary returns JSON for everything under /api". I added a control request to
`/api/definitely-not-a-route`, which returned `200 text/html`, proving the SPA fallback is live on
this binary and the content-type discrimination is real rather than an artefact. Recorded in the
evidence file under "Negative control".

**Process safety.** A vibe-kanban instance from a DIFFERENT checkout
(`/home/david/Tools/vk-swarm`, PID 1117432, backend port 9002) was running throughout. The probe
used port 9412 and an isolated `VK_ASSET_DIR` under the session scratchpad, was stopped by exact
PID, and the foreign instance was verified still alive afterwards. No `pkill`/`killall` was used
(CLAUDE.md §10.11). **Task 099's `VK_ASSET_DIR` override is what made this isolation possible** —
the seam built for the test harness paid off a second time here.

**Results.** Six restored paths, six `application/json` (`400 "Remote client not configured"`, the
expected no-hive state; task 401 later changes this to 503). Control path `200 text/html`.
`git grep 'api-keys\|api_key' -- crates/server/src/routes/` → `NO api-key surface in routes`.
`/api/nodes/api-keys` returns a `node_id` UUID parse error (`400 text/plain`), never a key listing.

**Stage 2 — adversarial panel (opus): NO FINDINGS**, and this panel INDEPENDENTLY REPRODUCED the
whole sweep rather than merely reading the file — the right standard for evidence the orchestrator
produced itself:
- Rebuilt the binary, ran on a different port (9413) with its own isolated asset dir. All six
  content-types matched the evidence file exactly, including the byte-exact UUID parse-error string.
- **Strengthened the negative control from one path to six**, including real-shaped siblings and
  prefix extensions: `/api/swarm/nonexistent`, `/api/swarm/templates/extra/segments`,
  `/api/nodes/{uuid}/bogus`, `/api/nodesx`, `/api/swarm/labelsz`. ALL returned `200 text/html`,
  ruling out any blanket JSON middleware on `/api`.
- **Disproved the alternative-match hypothesis** (angle 2): `/nodes` is defined in exactly one
  place (`crates/server/src/routes/nodes.rs:63-65`); the only other repo-wide `/nodes` hit is
  `swarm_projects.rs:157`'s `/swarm/projects/{project_id}/nodes/{node_id}`, a different prefix that
  cannot match. The `"Remote client not configured"` body is handler-emitted, not an error-layer
  artefact — an error layer would also have fired on `/api/nodesx`, which instead falls through to
  HTML.
- **Widened the D3/SC3 grep** beyond the task's `routes/` path: `git grep -ln 'api-keys\|api_key'
  -- crates/server/src/` returns NOTHING — no api-key surface anywhere in the node server crate,
  so the recorded claim is if anything understated.
- Provenance verified: the evidence commit `9dce4156` has parent `690ffab0` (task 104), matching
  the commit-under-test recorded in the file. Scope: 1 file, 102 insertions, no source smuggled in.
- Panel cleanup was equally disciplined: killed only its own PID, verified the foreign instance
  alive, removed its temp dirs, left `git status --porcelain` empty.

**Phase 1 (tasks 099–105) is COMPLETE.** SC1 and SC2 have end-to-end evidence: the four hive-proxy
route modules are restored, registered, covered by in-process registration tests that fail when
unregistered, and proven reachable on the real binary a user starts.

## Task 201 — delete the node's NodeApiKeySection UI (IRREVERSIBLE, human-approved)

**Human gate.** Surfaced to the user with the component size (451 lines), its single consumer, the
surviving hive copy, the D3/ADR-0013 rationale, and the known 201→202 dead-client transient. The
user chose "Approve — delete it". Token: `reviews/201.approved` (committed before dispatch).

**Implementer ledger: empty — verified genuinely empty.** The diff is exactly the two dictated
removals.

**Pre-dispatch amendments (ORCHESTRATOR).**
- **A1 — `scope_test` corrected from `frontend/src/pages/settings` to `N/A`.** The declared gate
  was both VACUOUS and IMPOSSIBLE. Vacuous: the only test in that directory referencing
  `OrganizationSettings` is `SettingsMobile.test.tsx:26`, which does
  `vi.mock('../OrganizationSettings', ...)` — it STUBS the component and never renders the real
  one, so deleting a child could not change its result. Impossible: the gate was already red before
  this task changed anything. Corrected the TASK TEXT here, which is the D1 class (a decompose-time
  defect of mine), explicitly NOT the task-103 class (contract right, code deviated → fix the code).
  The discriminator: at 103 the named gate was correct and passable; here it could never pass and
  did not cover the change.
- **A2 — baseline-delta discipline.** See the frontend test-debt section below.
- **A3 — proved the `isAdmin` STOP trigger would stay quiet BEFORE dispatch.**
  `frontend/tsconfig.json:16-17` sets `noUnusedLocals`/`noUnusedParameters: true`, so an orphaned
  variable is a HARD `tsc` error, not a warning. `isAdmin` is read at lines 82, 293, 341, 366, 383,
  all outside the deleted block. This is the deletion-task equivalent of the mock-shape pre-checks
  that caught D1 three times.

**Pre-existing frontend test debt — NAMED SCOPE SPLIT (CLAUDE.md "No Deferred Remediation").**
Discovered at this task's gate, on a clean tree, caused by nothing in this workstream (phase 1 was
Rust + docs only): `Test Files 8 failed | 26 passed (34)`, `Tests 15 failed | 408 passed (423)`.
Filed as **F-2026-07-31-01** (SettingsMobile asserts 6 accordion sections, component renders 8),
**F-2026-07-31-02** (`SystemSettings.test.tsx:40` — `vi.mock` factory closes over a hoisted import,
suite fails to LOAD), **F-2026-07-31-03** (the other six files), with a rationale section in the
workstream README. Sanctioned because it is out-of-scope pre-existing debt, tracked with evidence,
and NOT among CLAUDE.md's required PR gates — those are clippy, `cargo test --workspace`, frontend
lint + `tsc`, and remote-frontend lint + `tsc` + vitest. **`frontend`'s `tsc --noEmit` is GREEN
(exit 0) and stays load-bearing.** Every frontend task in this run asserts the DELTA (the failing
set must remain byte-identical), so redness cannot hide a regression.

**ORCHESTRATOR error caught and fixed before push — PARTIAL COMMIT.** My `git add` of the
already-`git rm`'d path errored, and the first commit captured ONLY the 451-line deletion, leaving
`OrganizationSettings.tsx` still importing a now-missing file. That commit was broken in isolation
(bisect-hostile) even though the working tree was fine. Caught by reading `git show HEAD --stat`
rather than trusting the commit succeeded; fixed with `--amend` (nothing had been pushed) and
re-verified `tsc` exit 0 at the amended HEAD. **Lesson: after any commit involving a deletion,
verify `git show HEAD --stat` lists EVERY expected path — a failed `git add` in a multi-path
command does not abort the commit.**

**Stage 2 — adversarial panel (opus): NO FINDINGS**, on the harshest brief of the run (a deletion's
damage does not surface as a failing test):
- Scope exact: 2 files, 456 deletions, ZERO insertions; the two authorised hunks and nothing else.
- SC7 holds: HEAD touched nothing under `remote-frontend/`; the hive's copy is intact at 20065
  bytes, last modified by `62cffb0c`, two commits before this workstream.
- Orphan hunt: every import of the deleted component (`@/components/ui/*`, `lucide-react`,
  `@tanstack/react-query`, `date-fns`) is heavily shared. `nodesApi` and `@/types/nodes` retain live
  consumers (`pages/Nodes.tsx`, `hooks/useNode.ts`, `components/swarm/NodeProjectsSection.tsx`).
  The only genuine orphans are the api-key functions at `lib/api/nodes.ts:41-73` and their types at
  `types/nodes.ts:46,67,72` — task 202's declared scope, correctly excluded from scoring.
- **The panel chased and DISPROVED its own lead**, which is the behaviour this stage is for: the
  i18n block `settings.swarm.apiKeys` (`frontend/src/i18n/locales/en/settings.json:744`) is
  unreferenced, but is NOT an orphan created by 201 — the deleted component never imported
  `useTranslation` and contained no `t(` calls, and `git grep` at HEAD^ finds no consumer either.
  Pre-existing dead i18n, untouched by this task.
- **Silenced-test check** (the deletion-specific risk): `git grep -ln 'NodeApiKeySection' HEAD^ --
  frontend/` returns ONLY the component and its mount — no test file at HEAD^ ever referenced it, so
  no test was deleted, made vacuous, or stopped running. Test-file count is 34 before and after.
- Runtime reachability: no route, nav entry, menu item, or deep link targets the removed section —
  it was an inline card inside `OrganizationSettings`, not its own route, so there is no blank area
  or crash path.
- Token audit: actual deletion is a SUBSET of the approved scope, nothing extra.
- Gates verbatim with REAL exit codes: `tsc` 0, `eslint --max-warnings 0` 0, vitest failing set
  byte-identical to the A2 baseline (same 8 files, same 15 assertions).

**SC3 is now partially satisfied** — the node's API-key UI is gone. Tasks 202 (client functions and
types) and 203 complete it.

## Task 202 — remove unreachable API-key/merge methods from nodesApi

**Implementer ledger: empty — verified genuinely empty.** Diff is `1 insertion(+), 61 deletions(-)`,
the single insertion being the narrowed import line.

**Pre-dispatch amendments (ORCHESTRATOR).**
- **B1 — `scope_test` corrected from `frontend/src/lib` to `N/A`.** SECOND instance of the task-201
  A1 defect, so this is now a pattern rather than a one-off. Vacuous: no test anywhere in
  `frontend/src` references `nodesApi` or `lib/api/nodes`. Impossible: `src/lib/taskSorting.test.ts`
  is one of the eight baseline-red files. **Standing check for remaining tasks: verify every
  `scope_test` both (a) covers the change and (b) passes on a clean tree, BEFORE dispatch.** Applied
  proactively from here on rather than discovering it at the gate.
- **B2/B3** — anchors and the "no surviving callers" STOP trigger verified against the live tree
  before dispatch; both clear.
- **B4** — baseline-delta discipline (unchanged from 201's A2).

**Stage 2 — adversarial panel (opus): NO FINDINGS / APPROVE.**
- Scope: exactly one path; zero hunks touch the four surviving methods (they appear only as
  unchanged context).
- Right five deleted: HEAD^ had 9 members, HEAD has exactly 4 (`list`, `getById`, `delete`,
  `listProjects`).
- **Route cross-check in BOTH directions** (the check that proves the deletion was correct rather
  than merely clean): no `/nodes/api-keys*` or `/merge-to/` route exists server-side, so the five
  deleted methods were genuinely unreachable; and all four survivors map 1:1 onto live routes at
  `crates/server/src/routes/nodes.rs:63-65`. Nothing reachable was deleted.
- No test silenced: no test at HEAD^ referenced any of the five names.
- SC7: HEAD touched zero paths under `remote-frontend/`; the hive's own
  `remote-frontend/src/lib/api/nodes.ts` retains its full surface (`listApiKeys:56`,
  `createApiKey:67`, `revokeApiKey:81`, `unblockApiKey:95`, `mergeNodes:111`).
- Gates: `tsc` 0, `lint` 0, vitest failing set byte-identical to baseline.

**PANEL-SURFACED GAP — orphaned type declarations with no owning task (fixed by new task 204).**
The panel checked something I had not asked any earlier panel for: whether a LATER task claims the
now-dead types. It grepped every remaining task file in phases 2-5 and found none. The four
declarations in `frontend/src/types/nodes.ts` — `NodeApiKey` (:46), `CreateNodeApiKeyRequest` (:67),
`CreateNodeApiKeyResponse` (:72), `MergeNodesResponse` (:78) — now form a closed cluster with zero
consumers elsewhere in `frontend/src`.

They break no gate (being `export`ed, neither `noUnusedLocals` nor ESLint's no-unused rules fire),
which is exactly why this would have shipped silently: the workstream would have closed fully green
while leaving dead code the spec's D3 intent says should be gone. Deleting them was correctly OUT of
202's scope (`types/nodes.ts` is not in its `files:`), so the fix is a new task, not a scope stretch.
**Task 204 created in THIS session** — no deferral (CLAUDE.md "No Deferred Remediation").

This is the second time an adversarial panel has caught something no gate could: task 104's panel
proved route reachability empirically, and this one found dead code that passes every check.

## Tasks 203 + 204 — architecture doc repoint, and the orphaned type cluster

**Both implementer ledgers: empty. Both panels: NO FINDINGS.** Phase 2 complete.

### Task 204 (created mid-run — see the task 202 entry for why)

Panel verified "truly orphaned" FOUR ways beyond `tsc`, which is the right standard for a deletion
whose whole justification is "nothing uses this":
- `grep -rn` across `frontend/src` → nothing.
- Whole `frontend/` tree → the only hit is `frontend/dist/assets/*.js.map`, a stale build artifact.
- Barrel/re-export check → the four importers of `types/nodes` import only `Node`/`NodeProject`; no
  `index.ts` re-exports the module.
- **ts-rs codegen check** → `grep 'NodeApiKey\|MergeNodesResponse' shared/types.ts` returns nothing,
  so none of the four was Rust-generated. There was no duplicate and no codegen copy is masked; the
  declarations were hand-written and genuinely dead.
SC7 intact: `remote-frontend/src/types/nodes.ts` still declares all four (lines 46, 67, 72, 78).

### Task 203 — and TWO defects the panel found in MY contract text

**C4 (found at implementation).** The task told the implementer to add a note reading "The node
server exposes no `/api/nodes/api-keys*` routes" while ALSO asserting that
`grep '/api/nodes/api-keys'` must return nothing. Both could not hold. The implementer reported the
hit verbatim rather than silently deleting the note or rewording it to dodge the grep — the correct
behaviour, and the reason the constrained-implementer role forbids improvisation. Fixed the
assertion, not the work: the real invariant is that no CITATION presents the path as a live node
endpoint.

**C5 (found by the panel's angle-7 check — the substantive one).** I asked this panel a question I
had not asked earlier ones: *are the repointed claims actually TRUE?* It found that my dictated
replacement text for the "Hard delete option" row was itself false. Verified chain:
- `NodeApiKeyRepository::delete` (`crates/remote/src/db/node_api_keys.rs:178`) has exactly one
  caller, `NodeServiceImpl::delete_api_key` (`crates/remote/src/nodes/service.rs:263-266`).
- `delete_api_key` has NO caller anywhere in `crates/`.
- The hive's `DELETE /v1/nodes/api-keys/{key_id}` (`routes/nodes.rs:57`) is bound to
  `revoke_api_key`, a SOFT revoke, and never reaches the hard delete.

`routes/nodes.rs` was never the "Used By" for that function — not on the node, not on the hive.
**Repointing a false citation to a differently-false citation would have shipped precisely the drift
this workstream was opened to remove.** The row now cites the real caller and states that no route
reaches it. Fixed in-session.

**C6.** `## Done when` said "the five that name a URL"; the table has four URL-bearing rows. An
arithmetic slip in my text, corrected.

**Pattern across this run, now unmistakable.** Every rejection and every contract defect has traced
to MY task text, never to an implementer improvising: `json!([])` mock bodies (102/103/104), a
vacuous+impossible `scope_test` (201, 202), a self-contradicting grep assertion (203 C4), and a
false citation (203 C5). All are properties of the surrounding system that decompose-time review
structurally cannot see. The two-stage design is doing exactly what it exists for — but the
signal is that the DECOMPOSER needs the pre-dispatch verification pass, not that the implementers
need more constraint.

## Task 301 — add ProjectWithStats and GET /api/projects/with-stats (additive)

Dispatched to **sonnet**, not haiku — the largest task in the run (new handler, two types, a
test-harness helper, route registration, ts-rs codegen).

**Implementer ledger: SIX declared items — the most useful ledger of this run.** Two mattered:

**Item 2 — the task's test block was DEFECTIVE and the implementer caught it.** My text showed
`h.seed_project("zeta", &[/* todo x3, in_progress x1, done x2 */])`. Those are comments inside an
EMPTY slice literal: it compiles, seeds ZERO tasks, and makes the `todo == 3` / `done == 2`
assertions vacuously false — the test would have failed confusingly, or worse, been "fixed" by
weakening the assertions. The implementer read the comments as the intended values, wrote the literal
`TaskStatus` arrays, and DECLARED the choice. The panel's experiment (c) vindicated it: seeding zero
tasks kills the test with `left: Number(0) right: 3`, proving the counts are genuinely DB-derived.

**Item 3 — `#[allow(dead_code)]` added to the SHARED harness.** The implementer added one to `Resp`'s
`content_type` field AND one to the whole `impl Resp` block. I challenged the impl-level one
specifically because `common/mod.rs` is used by every test binary, and a blanket allow there could
mask real dead code for all future tests. Removing it did NOT break clippy — it was unnecessary. The
field-level one is retained. The panel independently re-tested this (removed it, `touch`ed the file
to force a real 13.84s rebuild) and confirmed clippy exit 0 either way: redundant but harmless,
masking nothing, in a file the contract explicitly authorises.

**ORCHESTRATOR correction — `assert_registered()` was missing.** The task's test asserted only
`status == 200`. I ran the revert experiment, saw the test fail with a misleading message, and had
`res.assert_registered()` added for a diagnostic failure, matching the convention tasks 101-104
established.

**A reachability nuance worth recording.** With `/with-stats` unregistered, the request does NOT hit
the SPA catch-all — it falls into `.nest("/{id}", project_id_router)`, fails `Path<Uuid>` parsing,
and returns `400 text/plain`. So `assert_registered()` correctly PASSES in that case and the status
assertion is what catches it. Same phenomenon as `/api/nodes/api-keys` matching `/nodes/{node_id}`.
**`assert_registered()` is necessary but NOT sufficient for routes that have a dynamic sibling** —
relevant to any future task adding a static route beside a `{param}` route.

**Stage 2 — adversarial panel (opus): NO FINDINGS.** All FIVE hollow-test experiments killed it:

| # | Mutation | Result |
|---|---|---|
| a | unregister the route | KILLED (`:28` status assert) |
| b | reverse the sort comparator | KILLED (`left: "zeta" right: "alpha"`) |
| c | `seed_project` creates ZERO tasks | KILLED (`left: Number(0) right: 3`) |
| d | `seed_project` skips the TaskAttempt | KILLED (`last_attempt_at` null) |
| e | re-add `has_local` to struct + handler | KILLED (dead-field assertion) |

No vacuous assertions. Additivity proven at RUNTIME, not just by diff — the panel drove the same
seeded project through BOTH endpoints and showed every shared field identical, with
`/merged-projects` carrying exactly the three extras (`has_local`, `local_project_id`, `nodes`) and
still returning 200. Codegen integrity proven by re-running `npm run generate-types` and confirming
`git status` and `git diff` are both EMPTY afterwards, so `shared/types.ts` was not hand-edited.
Field-by-field: `ProjectWithStats` is `MergedProject` minus exactly those three, same order, same
types (`i32` per E1), same `#[ts(type=...)]` attributes.

**Known coverage gap, evidenced rather than fixed (a CONTRACT limitation, not implementer drift).**
The test asserts `task_counts.todo` and `.done` but not `.in_progress` or `.in_review`, because my
contract dictated those two assertions verbatim. The panel confirmed the `in_progress`/`in_review`
mapping is LINE-IDENTICAL to `merged.rs` (`with_stats.rs` vs `merged.rs`) and its runtime probe shows
all four counts serialised correctly (`{"todo":1,"in_progress":0,"in_review":0,"done":0}`). The
mapping is therefore proven correct by two independent means; only the automated regression net is
narrower than ideal. Recorded here rather than widened, to avoid unreviewed scope growth at this
stage — the frozen spec's required enrichment test is satisfied.

Whole-workspace gates green with REAL exit codes: `clippy --all` 0, `cargo test --workspace` 0,
`fmt` 0, `generate-types:check` up to date.

## Task 302 — repoint the board onto ProjectWithStats (IRREVERSIBLE, human-approved)

**Human gate.** Approved for 302 ONLY; the offered combined 302+303 approval was DECLINED, so 303
returns to the gate separately. Token `reviews/302.approved` records that explicitly.

**Implementer ledger: FIVE declared items, all verified by the panel as correct and necessary.**
Item 2 (`hasProjects` rewritten from `counts.total > 0` to `(projectsData?.projects.length ?? 0) > 0`)
was the one worth checking hardest — the panel read HEAD^ and confirmed `counts.total` was the
UNFILTERED array length, not a local-only subset, so the predicate is unchanged. Item 4 (removing
`Link2`/`LinkToLocalFolderDialog` imports) was confirmed genuinely orphaned: both were used ONLY
inside the `{!project.has_local && ...}` block, provably dead since `has_local: true` is hardcoded.

**Pre-dispatch amendments (ORCHESTRATOR).** F1 corrected an IMPOSSIBLE assertion: the task demanded
`grep 'local_project_id' frontend/src` return nothing, but that field legitimately exists on
`NodeProject` (`types/nodes.ts:36`), the swarm types, the task API type and the electric schema — it
would have failed on a perfect implementation. Same class as 201/202's vacuous `scope_test`. F2-F4
pre-verified the blast radius (`has_local` in exactly the 4 allowlisted files; `MergedProject` in
exactly 6, all allowlisted).

**ORCHESTRATOR error — I REPEATED the task-201 partial-commit mistake.** `git add` on an already
`git rm`'d path fails and aborts the ENTIRE add, so my first commit captured only the three
deletions, leaving the board mid-migration and broken in isolation. Caught by reading
`git show HEAD --stat`; amended before push. **The lesson was already in the ledger from 201 — the
failure was not knowing it, but not applying it.** Standing rule: after ANY commit involving a
deletion, read `git show HEAD --stat` and confirm every expected path is listed.

**Stage-1 gate fixed properly rather than disabled.** `scope_test: "frontend/src/components/projects"`
failed with `sh: 1: vitest: not found` (exit 127) because the gate runs the test command from the
REPO ROOT, where vitest is not installed. That is a harness path issue, not a code failure. Supplying
`WAI_TEST_CMD='(s={scope}; cd frontend && npx vitest run "${s#frontend/}")'` makes the gate pass on a
REAL test execution. Recorded as a plan convention for tasks 402/403. This also confirms 201/202/204
were right to use `N/A`: their scopes contained baseline-red files no command could fix.

**ORCHESTRATOR false alarm, self-corrected.** My first blank-board experiment appeared to PASS,
suggesting the regression guard was hollow. It was my own error: a single-line `sed` silently
no-opped against the two-line `projects: fixture,` mock. Re-run with a correct mutation, the test
fails as it should. I warned the panel about the trap; it hit the same two-line form and mutated
correctly via python.

**Stage 2 — adversarial panel (opus): NO FINDINGS**, on an 11-row rewrite-by-rewrite audit:
- Every dropped-field site rewritten with the DICTATED semantics — no condition inverted, no branch
  lost. Notably `UnifiedProjectCard.tsx:132` correctly KEPT the `!project.git_repo_path` guard while
  dropping only the `!has_local` conjunct, and the three `has_local && ` sites unwrapped their JSX
  without losing any menu item (openInIDE, openTerminal, github.settings, Edit, Delete all retained).
- **Endpoint URL verified against the ROUTER, not just the type system** — the a85f7d63 failure class
  that mocked tests structurally cannot catch. `routes/mod.rs:83 .nest("/api")` +
  `projects/mod.rs:149 .nest("/projects")` + `:136 .route("/with-stats")` = `/api/projects/with-stats`,
  byte-matching the literal in `lib/api/projects.ts:112`. Hyphen, not underscore.
- **Payload parity proven at the projection level:** `with_stats.rs` and `merged.rs` run the same
  query, map the same 12 shared fields with no hardcoding or zeroing, apply the identical sort, and
  neither adds a LIMIT or filter. No project and no field silently disappears from the board.
- Mutation matrix: zeroed `task_counts` KILLS the test; `projects: []` (the blank board) KILLS it.
  The a85f7d63 regression guard is real.

**PANEL-SURFACED ORPHAN — escalated to the user, NOT auto-deleted (F-2026-07-31-04).**
`frontend/src/components/dialogs/projects/LinkToLocalFolderDialog.tsx` now has ZERO consumers, since
its only mount point was the dead `!has_local` block. This is the same shape as task 202's discovery
that produced task 204 — but the disposition differs, deliberately:

204's orphans were dead TYPE declarations with no behavioural meaning, so deleting them was pure
cleanup. This dialog is a FEATURE with a live backing stack: `projectsApi.linkLocalFolder`
(`lib/api/projects.ts:117`), `useProjectMutations.linkLocalFolder`
(`hooks/useProjectMutations.ts:70,161`), and the server route `/api/projects/link-local`
(`crates/server/src/routes/projects/mod.rs`). Deleting it removes the ability to link a local folder
to a remote project — a PRODUCT decision outside this workstream's frozen spec (which covers
hive-proxy routes, the API-key surface, `ProjectWithStats`, and the hive-absent state).

It was already unreachable before this run (the `!has_local` guard has been permanently false since
node-foundations), so 302 did not remove working functionality — it removed the last reference to
something already dead. Filed as F-2026-07-31-04 and put to the user at the 303 gate rather than
resolved unilaterally.

## Task 303 — delete MergedProject, NodeLocation, /api/merged-projects (IRREVERSIBLE, human-approved)

**PHASE 3 COMPLETE. SC5 satisfied.**

**Human gate.** Approved for 303 only; the offered "303 + phase 4 upfront" was DECLINED. At the same
gate the user decided F-2026-07-31-04 (the orphaned `LinkToLocalFolderDialog`) as **"leave it —
backlog only"**, so the dialog, its API client, its hook entry and the `/api/projects/link-local`
route all REMAIN. That decision was written into the token and into the dispatch as an explicit
prohibition, and the panel verified compliance file-by-file.

**Implementer ledger: ONE declared item, and it was RIGHT.** It found a SECOND stale doc comment my
amendment G1 missed — `types.rs:123`, "Replaces `MergedProject`, whose merge fields are dead" — left
by task 301. Fixing it was not optional: the string `MergedProject` would have failed this task's own
`## Done when` grep, and would have left documentation describing deleted code (the very drift task
203 existed to fix). `types.rs` is in `files:`, so no boundary was crossed. **G1's silence on that
second comment was a gap in MY amendment, not a scope boundary** — and the implementer resolved it in
the direction the contract's own completeness criterion demanded, then declared it.

**Pre-dispatch amendments (ORCHESTRATOR).** G1 added `with_stats.rs` to `files:` because task 301's
doc comment named `MergedProject`/`merged.rs`/`/merged-projects` — the same impossible-assertion
defect corrected at 302 (F1) and 203 (C4). G2-G4 pre-verified the blast radius and protected
`impl From<Project> for RemoteNodeProject` (`types.rs:88-110`), which sits directly above the deleted
structs and constructs same-named fields — the easiest thing in this task to clip by accident.

**Stage 2 — adversarial panel (opus): NO FINDINGS**, with the strongest evidence set of the run:
- **Survivors verified individually:** `impl From<Project> for RemoteNodeProject` (`types.rs:88`),
  `RemoteNodeProject` (`:69`), `TaskCounts` (`:115`, used at `:149`). **`CachedNodeStatus` is NOT
  orphaned** — I asked specifically because it was a `NodeLocation` field type; it remains used at
  `types.rs:84`, `:163`, `generate_types.rs:37`, `services/node_cache.rs:268-274`, and
  `frontend/.../RemoteProjectCard.tsx:10`.
- **Codegen enumerated exactly**, not merely "check passes": `export type` symbols went 244 → 241,
  removing precisely `MergedProject`, `MergedProjectsResponse`, `NodeLocation` — 0 added, 0
  collaterally dropped. Re-running `generate-types` left `git status` empty, proving `shared/types.ts`
  was regenerated rather than hand-edited.
- **Runtime proof on an isolated port (9377):** `/api/merged-projects` → `200 text/html` (SPA
  fallback = route gone), `/api/projects/with-stats` → `200 application/json` with a real payload,
  and the decisive one — `/api/projects/link-local` POST → `422 "missing field remote_project_id"`,
  i.e. the extractor's rejection, which proves the handler was REACHED and is definitively not the
  catch-all. A control request to `/api/definitely-not-a-route` returned `200 text/html`, confirming
  the discriminator.
- **Board survival re-proven:** mutating `ProjectList.test.tsx`'s mock to `projects: []` (via python,
  with the mutation confirmed applied in-file first — the two-line trap that produced my own earlier
  false alarm) KILLS the test.
- Whole-workspace gates green: clippy 0, `cargo test --workspace` 0, fmt 0, build 0, frontend tsc 0,
  lint 0, vitest at the exact documented baseline with the 8 failing files enumerated BY NAME (counts
  alone could mask a swap) and no new entrant.

**Three panel observations, correctly NOT acted on, now filed:**
- **F-2026-07-31-05** — `useProjectMutations.ts:79` still invalidates `queryKey: ['mergedProjects']`,
  now a no-op since no query produces that key. The panel left it because touching that file would
  have violated the user's explicit keep-decision. Correct call: an explicit user instruction
  outranks tidiness.
- **F-2026-07-31-06** — stale doc comment at `crates/db/src/models/project/mod.rs:106` ("used in
  merged projects view"). Outside `files:`.
- **F-2026-07-31-07** — `remote-frontend/src/types/shared/types.ts` still declares the deleted types.
  **Pre-existing drift, NOT caused by this run**: it already differed from `shared/types.ts` by 20
  lines at HEAD^, is a hand-committed copy last touched by an unrelated hive PR (`b8c12d96`), is not
  written by `generate_types.rs` (which writes only `shared/`), and no `remote-frontend` source
  references the names. Filed as medium — a hand-maintained duplicate of a generated file is a
  standing drift hazard.

## Task 401 — HiveNotConfigured error variant (hive-absent is now 503, not 400)

**Implementer ledger: empty — verified genuinely empty** by the panel (diff is exactly the three
dictated edits; the only divergence from the contract's literal snippet is that rustfmt collapsed a
braced match arm to one line, which `cargo fmt --check` exit 0 confirms is canonical).

**The risk in this task was never the diff — it was RETRY BEHAVIOUR.** Changing 400 → 503 is one
line, but 5xx is the classic "retry me" signal: if anything in the frontend retried on 5xx, a quiet
hive-absent state would become a retry storm against an endpoint that can never succeed. That
regression passes every gate and surfaces only as mysterious load.

I analysed this MYSELF before the panel reported, and the panel reached the same conclusion
independently — deliberate corroboration rather than a single point of failure:
- `handleApiResponse` (`frontend/src/lib/api/utils.ts:125-148`) treats ALL non-2xx identically: it
  parses the body, lifts `errorData.message`, and throws `ApiError(message, status, response)`. There
  is no `>= 500` short-circuit, so the 503 body reaches callers intact and `ApiError.status === 503`
  is available as task 402's discriminator.
- The global `QueryClient` (`frontend/src/main.tsx:10-17`) sets only `staleTime` and
  `refetchOnWindowFocus` — **no `retry` override**. TanStack Query's default (`retry: 3`) retries on
  ANY thrown error without inspecting status, so the hive-proxy queries were ALREADY retried 3× under
  400.
- Every per-hook `retry:` override in the codebase is a plain number
  (`useTaskRelationships.ts:32,52`, `useBranchStatus.ts:12`, `useAuthStatus.ts:14`) — **zero
  status-predicate retry functions anywhere in `frontend/src`**.

**Conclusion: retry count per hive-proxy request is UNCHANGED at 3 under both 400 and 503.** No
regression. (Standing observation for task 403: a hive-absent node still retries every hive-proxy
query three times before settling — pre-existing, not introduced here.)

The panel additionally found the ONE 503 branch in the frontend — `ConfigProvider.tsx:128`
(`errorStatus === 503` inside `isProxyError`, which retries with backoff) — and proved it
UNREACHABLE from this change: it wraps `configApi.getConfig()` → `/api/config` only, and
`grep 'remote_client' crates/server/src/routes/config.rs` returns nothing, so that path can never
emit `HiveNotConfigured`.

**Blast radius is the declared one.** `RemoteClientNotConfigured` is constructed at exactly three
sites (`local-deployment/src/lib.rs:201,206`, `container.rs:181`) and reaches `ApiError` through the
single `From` impl. Every `?`-propagating caller moves 400 → 503, which the contract names
explicitly (including `/api/organizations*`). Non-`?` sites (`oauth.rs:177` `if let Ok`,
`labels.rs:70` `.ok()`, `.inspect_err`/`.map_err` sites) are unaffected by construction. Nothing in
`crates/` or the frontend matches the old literal `"Remote client not configured"` outside its two
`thiserror` definitions.

**No codegen impact — disproved rather than assumed.** `error.rs:30-31` carries
`#[derive(ts_rs::TS)]` with `#[ts(type = "string")]`, which collapses the entire enum to `string` in
TypeScript, so adding a unit variant CANNOT change generated output. Confirmed by
`generate-types:check` exit 0 and `grep 'HiveNotConfigured' shared/ frontend/src` returning nothing.

**Runtime proof (panel, isolated port 9477, `env -u` on the hive vars to defeat the `option_env!`
bake-in trap):** all four hive-proxy paths return `503 application/json` with
`"HiveNotConfigured: This node is not connected to a hive"`; `/api/projects/with-stats` and
`/api/health` stay 200; `/api/definitely-not-a-route` still returns the `text/html` catch-all. The
health endpoint reported `git_commit: b269b420`, proving the binary under test was HEAD.

**Task 105's evidence is NOT stale — checked, not assumed.** `reviews/105-reachability-evidence.md`
records 400s, but line 32 states the discriminator is CONTENT-TYPE, not status, and lines 37-38
explicitly anticipate "task 401 later changes it to 503". Every assertion it makes survives the
change. Self-superseding; left unedited.

**ORCHESTRATOR follow-up — corrected four now-misleading assertion messages.** The hive-absent tests
from tasks 101-104 asserted `assert_ne!(res.status, 500)` with the message *"absent hive is a
client-visible state, not a server error"*. That prose was written when the response was 400; 503 IS
formally in the 5xx server-error class, so on failure the message would now contradict the very
contract task 401 establishes. The assertion is still correct — what it guards against is an
UNHANDLED 500 — so I updated the message to say so:
`"hive-absent must be the specific HiveNotConfigured 503 (task 401), never an unhandled 500"`.
Comment-only, zero behavioural change, `cargo test -p server` green. This is the same
documentation-drift class tasks 203 and 303 existed to remove; leaving it would have been a small
instance of exactly what this workstream is about.

## Task 402 — render an explicit not-connected-to-a-hive state

**Implementer ledger: ONE item — the user-facing copy**, which the task deliberately left open. It
followed the sibling `Alert`/`AlertDescription` wrapper and the `t(key, default)` convention rather
than inventing a pattern. Correct, and correctly declared.

**Pre-dispatch amendments (ORCHESTRATOR).** H1 resolved a genuine coin-flip: `ApiError` carries BOTH
`.status` and `.statusCode` (`utils.ts:10-24`, `status` assigned FROM `statusCode`), so either would
have passed the unit test while only one matches the file's convention — dictated `.status`. H5
forbade matching on the message string, since
`"HiveNotConfigured: This node is not connected to a hive"` is a rendering detail, not a contract.

**The real risk was BRANCH ORDERING, which no test catches.** The hive-absent branch must sit AFTER
the loading branch (else a pending request flashes "not connected to a hive") and BEFORE the generic
error branch (else a hive-absent node still shows a destructive alert and the task's purpose is
defeated at that surface). I asked the panel for a per-file table across all six surfaces rather than
a spot check. Result — correct everywhere:

| Surface | loading | hive-absent | generic error |
|---|---|---|---|
| SwarmProjectsSection | :210 | :214 | :216 |
| SwarmLabelsSection | :206 | :210 | :212 |
| SwarmTemplatesSection | :193 | :197 | :199 |
| NodeTemplatesSection | :155 | :159 | :161 |
| NodeProjectsSection | :280 | :283 | :285 |
| pages/Nodes.tsx | :34 (+ `!orgId` :38) | :42 | :44 |

**No conflict with the other 503 handler.** `ConfigProvider.tsx:126-130` treats `errorStatus === 503`
as a retryable proxy error, but it is a self-contained `catch` doing structural duck-typing
(`'status' in err`) on `/api/config`, never calls `isHiveNotConfigured`, and is untouched here. Task
401's panel had already proven `/api/config` cannot emit `HiveNotConfigured`; this task did not widen
that surface.

**`NodeTemplatesSection.tsx`'s larger diff (13 lines vs 4) explained and verified:** it destructures a
previously-unused `error: swarmTemplatesError` at `:65`. The LOCAL query's `isLoading`/`error`
(`:55-56`) are unchanged and still drive the local "Failed to load local templates" branch — nothing
shadowed or dropped. The extra lines are prettier re-wrapping plus imports.

**Stage 2 — adversarial panel (opus): NO FINDINGS**, with mutations A (component returns `null`) and
B (detector always true) both correctly failing the test.

**ORCHESTRATOR follow-up — closed a REAL coverage gap the panel found (its mutation D).** Widening
the detector to `(err.status ?? 0) >= 500` **survived the test suite**: nothing pinned the match to
exactly 503, so a 500 or 502 from any hive-proxy route would have rendered "not connected to a hive"
— a wrong, user-visible diagnosis that every gate would have passed. I added three assertions to
`HiveNotConnected.test.tsx` (500, 502, 504 → false) and **re-ran the panel's exact mutation to prove
the gap is closed**: it now fails with `AssertionError: expected true to be false` where it
previously passed, and the detector restores byte-identical.

(The panel's mutation C — keying on `.statusCode` instead of `.status` — survives and is correctly
NOT a finding: the constructor populates both, so the two are interchangeable at runtime. H1's
dictate was about consistency, not correctness.)

**Known gap, filed not fixed: F-2026-07-31-08.** The i18n key `settings.swarm.hiveNotConnected` is
undefined in all four locales, so ja/ko/es fall back to the English default. Sibling keys such as
`settings.swarm.projects.title` ARE defined, so this is a genuine gap — but the locale files were
outside this task's `files:`, making `t(key, default)` the correct in-scope behaviour.

## Task 403 — harden the four remote stream hooks (PHASE 4 COMPLETE)

**The task text was WRONG about `useNodeLogStream`, and the real defect was worse than the one I
specified.** I assumed it needed 503/`isHiveNotConfigured` handling and forbade editing it. Reality:
`/v1/*` is the hive's namespace and is unregistered on the node server, so on a hive-less node the
request falls through to the SPA catch-all and returns `200 text/html`. Because the status is 200,
`if (!response.ok)` never fires and `response.json()` threw
`SyntaxError: Unexpected token '<', "<!doctype "... is not valid JSON` — surfaced to the user via
`console.error` and the error state, on every hive-less node whose logs were viewed.

**This is the SPA-catch-all trap in PRODUCTION code, not just tests.** It is the same root cause that
made `assert_ne!(status, 404)` vacuous across seven task files in phase 1 and that
`Resp::assert_registered()` exists to catch: **on this server a status code does not tell you whether
a route exists.** Content-type does. Phase 1 learned that lesson about test assertions; task 403
found the same bug shipped in a hook.

**Guard ORDERING matters as much as the guard**, and both directions are pinned:
- Guard AFTER `!response.ok` → only a SUCCESSFUL non-JSON response counts as "no stream"; genuine
  failures still throw.
- Deleting the guard → the SPA-fallback test fails with the exact `SyntaxError`.
- Moving the guard ABOVE `!response.ok` → the `500 text/plain` test fails
  (`expected null not to be null`), proving it would swallow real errors.
I ran both mutations myself, and the panel independently reproduced both.

**J6 — I OVERRODE A STOP TRIGGER, and the panel was right to demand it be recorded.** The contract
said `useNodeLogStream.ts` must be "reported back unmodified"; I authorised the edit in a SendMessage
review brief, which left the shipped code contradicting the contract's `## Done when` until J6 was
written. The override itself was justified (the STOP's stated reason — needing a task id from
`ProcessLogsViewer` — did not apply to a content-type guard inside the file's own boundary), but
issuing it out-of-band was a process defect on my part. **An orchestrator override belongs in the
task file, not only in a message.** Recorded as amendment J6.

**Implementer conduct was exemplary throughout:** it STOPped rather than improvising on the forbidden
file; it flagged that `useRemoteConnectionStatus` ALREADY computed a definite `'disconnected'`
(`:117-123`) so my task text implied a bug that did not exist; and it proactively added a negative
case (non-503 still errors) after being warned about task 402's over-broad-detector class.

**Stage 2 — adversarial panel (opus): NO FINDINGS**, with a per-hook pinning table and four
observations, all correctly scoped as observations rather than findings:
1. **F-2026-08-01-01** — `useDiffStream` and `useRemoteConnectionStatus` have NO test pinning their
   503 discrimination: replacing `isHiveNotConfigured(e)` with `if (true)` in both SURVIVED the whole
   72-test scope run. Their source is correct today; nothing stops a future edit from broadening them
   to swallow all errors silently. Filed.
2. **F-2026-08-01-02** — retry suppression is unpinned by construction: the test wrapper sets
   `retry: false`, so the suite structurally cannot observe that `useAvailableNodes` no longer
   retries on 503. Filed.
3. The J6 contract contradiction, now recorded above.
4. A genuine hive replying 200 with no content-type header would read as "no stream" — low
   likelihood, noted.

**Consumer compatibility verified by the panel, and one change is a strict IMPROVEMENT:**
`DiffsPanel.tsx:111` previously returned an error card when the hive-absent error was set, which
**blocked local diffs entirely**; with `error` now null it falls through and renders the local diff
stream. `CreateAttemptDialog` resolves immediately instead of after 3 retries plus backoff, so the
local-attempt path is less blocked than before, not more.

**Retry behaviour is correct by construction:** `useAvailableNodes` now RESOLVES `{ nodes: [] }` for
503 — a fulfilled promise, which TanStack cannot retry — while real errors still `throw`, leaving the
default `retry: 3` untouched for them.

**SC7 verified with the strongest available evidence.** `remote-frontend` is at exactly its
documented baseline (52 files / 405 tests passing; lint 0, tsc 0), and
`git diff $(git merge-base HEAD origin/main)..HEAD -- remote-frontend/ crates/remote/` is EMPTY —
the hive was not touched anywhere in this branch, not merely left passing.

## Reachability gate

Run-level gate, fired once at close (task 501). `change_kind: bugfix` → mandatory.
Full captures: `reviews/501-live-before-evidence.md`, `reviews/501-live-after-evidence.md`.

### (a) CALL-PATH TRACE

Traced against the **merged code as it actually exists**, not the spec's model of it.

1. **Production entry point.** `crates/server/src/routes/mod.rs` — `pub async fn router(deployment)`
   composes the app. Its `.merge(...)` chain registers `nodes::router()`, `swarm_projects::router()`,
   `swarm_labels::router()`, `swarm_templates::router()` under `/api`. The chain **ends** at
   `.route("/{*path}", get(frontend::serve_frontend))` (`mod.rs:76`).
2. **Why the bug was invisible.** `serve_frontend` returns `StatusCode::OK` with index.html
   (`frontend.rs:40-43`). An UNREGISTERED `/api` GET therefore returns **200 text/html**, never 404.
   This is the spec's one factual error: it described the symptom as a 404. It is not. Consequence,
   recorded at task 100/105 — `assert_ne!(status, 404)` is **vacuous** here and would have passed
   against the broken production server. Confirmed live in the before-capture. The spec was NOT
   edited (ADR-0001); the assertions were corrected instead, and `Resp::assert_registered()`
   (`crates/server/tests/common/mod.rs`) now fails when a response is the SPA fallback.
3. **The changed code is ON that path.** `routes/nodes.rs:61-65` declares
   `.route("/nodes", get(list_nodes))` etc. `list_nodes` (`routes/nodes.rs:22-28`) takes only
   `State` + `Query` — **no `.layer()`, no auth middleware anywhere in the router**. Body:
   `deployment.remote_client()?` then `client.list_nodes(query.organization_id).await?`.
4. **Confirmed executing in production, by output only the handler can produce.** Live on the
   deployed branch build, `GET /api/nodes` returns
   `Failed to deserialize query string: missing field 'organization_id'` — emitted by the
   `Query<ListNodesQuery>` extractor of the restored handler. The catch-all is
   `serve_frontend(uri: axum::extract::Path<String>)` (`frontend.rs:13`) — a wildcard `Path` capture
   that always succeeds, with **no `Query` extractor**, so it cannot name that field; a nonexistent
   route cannot reject a query string. (Corrected: an earlier revision said the catch-all "has no
   extractor" full stop. It has a `Path` one. Conclusion unchanged — found by working panel attack
   vector 6, "claims that overstate what the captured output shows", against my own docs.)
5. **The proxy traverses end-to-end.** With a well-formed uuid, all four return `401` in the
   `ApiResponse` envelope. Two sites share that string — `error.rs:261`
   (`RemoteClient(RemoteClientError::Auth)`) and `error.rs:309` (`ApiError::Unauthorized`). The
   absence of any middleware layer (step 3) rules out the latter: nothing can 401 before the handler
   body runs. So the node built a remote client, **called the hive, and propagated the hive's
   rejection**. Live end-to-end proxy traversal.

   **A GLOBAL `/api` auth layer would defeat that argument** — it would 401 via `error.rs:309`
   without any per-router `.layer()`, and step 3 alone does not exclude it. The same live capture
   excludes it empirically: **`GET /api/projects/with-stats` returned `200 application/json` with
   real project rows on the SAME unauthenticated curl**, in the same sweep. `/api` is therefore not
   blanket-guarded, so the four 401s cannot come from a global guard. Cited explicitly so a future
   session need not re-derive it.
6. **Corollary, stated rather than implied.** `remote_client()?` returned `Ok`, so task 401's
   `From<RemoteClientNotConfigured>` → `503 HiveNotConfigured` correctly did NOT fire: this host IS
   hive-configured. SC4's 503 path is consequently **not live-observable on this host by
   construction**, and remains covered by in-process tests only. Recorded as a known limit of this
   evidence, not papered over.

### (b) REAL-SEAM TEST

`crates/server/tests/common/mod.rs:109` and `:162` build the app via
`server::routes::router(deployment.clone()).await` — **the same function the production binary
calls**, catch-all included. Tests issue real HTTP through that router; none call a handler
directly. This satisfies the gate's explicit failure mode ("a task whose only test calls the
changed unit directly FAILS"): no test in this run does.

Seam tests: `nodes_routes.rs`, `swarm_projects_routes.rs`, `swarm_labels_routes.rs`,
`swarm_templates_routes.rs`, `projects_with_stats.rs`, `harness_smoke.rs`.

`harness_smoke.rs` is load-bearing in a way worth naming: it proves the harness can DETECT the SPA
fallback. Without it, `assert_registered()` could itself be vacuous — a test asserting a test.

### (c) INCIDENT-SYMPTOM ASSERTION

The incident symptom is *"node Nodes/swarm screens receive HTML where they expect JSON"* — not
"a helper returns X". Asserted at both levels:

- **Live, before:** all four routes `200 text/html` on `main`/`feff74be` (before-capture).
- **Live, after:** all four reach the handler and return JSON/extractor errors; `/api/merged-projects`
  inverted to the SPA (deleted); `/api/projects/with-stats` serves real project rows (after-capture).
- **In-suite:** `assert_registered()` fails precisely on the content-type signature of the symptom,
  so a regression re-introducing it turns the suite red rather than green.

(a), (b) and (c) all hold.

VERDICT: PASS

## Deploy verification

Feature-branch build deployed by the user to the live node at `http://NODE_HOST`
(hive: `https://HIVE_HOST`). Merge is NOT a prerequisite for deploying a branch.
Build identity was verified BEFORE any probe was trusted — the deployed commit is byte-identical to
the branch HEAD under review:

```text
$ curl -s http://NODE_HOST/api/health
{"status":"ok","version":"0.0.125","git_commit":"374598a7","git_branch":"feat/vk-swarm-node-ui-localize","build_timestamp":"2026-08-03T20:22:33Z","database_ready":true}

$ git rev-parse --short=8 HEAD
374598a7
$ git branch --contains 374598a7
* feat/vk-swarm-node-ui-localize

$ for p in /api/nodes /api/swarm/projects /api/swarm/labels /api/swarm/templates /api/merged-projects /api/projects/with-stats; do
    printf '%-42s -> ' "$p"; curl -s -o /dev/null -w '%{http_code} %{content_type}\n' "http://NODE_HOST$p"; done
/api/nodes                                 -> 400 text/plain; charset=utf-8
/api/swarm/projects                        -> 400 text/plain; charset=utf-8
/api/swarm/labels                          -> 400 text/plain; charset=utf-8
/api/swarm/templates                       -> 400 text/plain; charset=utf-8
/api/merged-projects                       -> 200 text/html
/api/projects/with-stats                   -> 200 application/json

$ curl -s http://NODE_HOST/api/nodes
Failed to deserialize query string: missing field `organization_id`

$ curl -s "http://NODE_HOST/api/nodes?organization_id=00000000-0000-0000-0000-000000000000"
{"success":false,"data":null,"error_data":null,"message":"Unauthorized. Please sign in again."}

$ curl -s http://NODE_HOST/api/merged-projects | head -c 120
<!DOCTYPE html>
<html><head><title>Build frontend first</title></head>
<body><h1>Please build the frontend</h1></body></

$ curl -s http://NODE_HOST/api/projects/with-stats | head -c 200
{"success":true,"data":{"projects":[{"id":"c8809147-3066-439e-9f2b-9477cb3e8bec","name":"vibe-kanban","git_repo_path":"/home/david/Code/vibe-kanban","created_at":"2025-11-28T03:41:40.239Z","remote_pro
```

**Observed SC outcomes:** SC1/SC2 (four node-surface routes registered and proxying to the hive —
proven by handler-specific output, not by status code); SC5/ADR-0014 (`/api/merged-projects` gone,
`/api/projects/with-stats` serving real rows).

**Not observable on this host:** SC4's `503 HiveNotConfigured`, because this node IS hive-configured
(`remote_client()` returned `Ok`). In-process tests cover it. Recorded as a limit, not claimed.

## Task 502 — the hive-absent 503 was never actually pinned (found at 501 close)

**LEDGER: one orchestrator-initiated correction, recorded in full.**

Found by the orchestrator working attack vector 4 independently while the Stage-2 panel was
running. The four hive-absent tests asserted `assert_ne!(res.status, 500)` under a message that
*claimed* to pin `503`. The assertion passes for 200/400/401/404 — anything but 500.

**This invalidated a claim already written into task 501's ledger.** The `## Deploy verification`
section states SC4's 503 is *"not observable on this host ... In-process tests cover it."* That was
FALSE as written: no in-process test pinned 503. Corrected at the source (the tests), not by
softening the ledger sentence — the sentence is now true.

Fourth instance of one defect class in this run — `assert_ne!(status, 404)` (phase 1),
`status >= 500` (402 mutation D), the over-broad content-type guard (403), and now this. All four
were **invisible to Stage 1** (mechanical) and to the suite (green either way). The through-line
holds: every one originated in ORCHESTRATOR-authored task text, not in implementer improvisation.

**The product was already correct.** Behaviour is genuinely 503; only the test was hollow. No
product code changed.

**Mutation evidence — the assertion is real, and all four kill it:**

```text
$ # mutate crates/server/src/error.rs:201 SERVICE_UNAVAILABLE -> BAD_GATEWAY
$ cargo test -p server --no-fail-fast --test nodes_routes --test swarm_projects_routes \
      --test swarm_labels_routes --test swarm_templates_routes
EXIT=101
failure blocks: 4
  left: 502
 right: 503

$ # revert error.rs, re-run
$ git diff --stat crates/server/src/error.rs
(empty)
test result: ok  x4
```

`--no-fail-fast` was required: the first run stopped after ONE failure block, which would have left
three of the four unproven. Checking only the aggregate exit code would have hidden that.

**Full gates after the fix (real exit codes captured, not paraphrased):**

```text
fmt=0        (nightly imports_granularity/group_imports warnings only; exit code read directly)
clippy=0     cargo clippy --all --all-targets --all-features -- -D warnings
test=0       cargo test --workspace  -> 57 "test result: ok" blocks
```

## Stage-2 panel FAILURE on tasks 501 and 502 — recorded, not papered over

**Tasks 501 and 502 closed WITHOUT an independent Stage-2 adversarial panel.** A panel
(`panel-501-a`, Opus) was dispatched with the standard 7 attack vectors. It never returned findings
despite four explicit requests, cycling to `idle_notification ... "idleReason":"available"` four
times (2026-08-03 20:42, 2026-08-04 01:31, and two mid-run) without producing a single line of
output. No partial result, no error.

Every other task in this run (099–403) received a real panel. These two did not.

**What was done instead.** The orchestrator worked the panel's attack vectors directly. This is
explicitly WEAKER than an independent check — it shares the orchestrator's blind spots, which is the
entire reason Stage 2 exists as a separate rung. It is recorded as a substitution, not an
equivalent.

**It was not empty theatre — self-review caught two real defects:**

1. **Vector 4 → task 502 (BLOCKING).** Four hive-absent tests asserted `assert_ne!(res.status, 500)`
   under a message claiming to pin `503`. This falsified 501's own ledger claim that in-process
   tests cover SC4's 503 path. Fixed; mutation-verified across all four.
2. **Vector 6 → the registration proof's justification (MINOR).** The evidence docs asserted the SPA
   catch-all "has no extractor". It has one: `serve_frontend(uri: axum::extract::Path<String>)`
   (`frontend.rs:13`). The conclusion held (no `Query` extractor; a wildcard `Path<String>` always
   succeeds) but the stated reason was wrong. Corrected in both the ledger and the after-evidence
   doc.

**Vectors NOT independently covered**, and therefore the residual risk a reviewer should weigh:
1 (call-path trace re-derivation), 3 (real-seam test genuineness), 5 (incident-symptom mapping),
7 (SC7 hive-untouched) — all were verified by the orchestrator, none by a second party. Vector 7 has
the strongest independent backing regardless, being a mechanical empty-diff check.

**Recommendation for `/wai:close` or `/wai:ship`:** if an independent adversarial review of 501/502
is cheap to obtain, run it before merge. The deploy evidence itself is live-captured and
reproducible from the recorded commands, so re-verification does not depend on trusting this ledger.

## Post-review known issues

Non-actionable findings from the `/dr:code-review` pre-graduation gate (round 1). Recorded so they
do not resurface as fresh blockers in a later round, per SC3a/SC3b.

1. **`crates/services/tests/normalize_sync_test.rs:359-368` — `test_fast_execution_no_lost_logs`
   flakes under full-workspace runs.** The `tokio::time::timeout` result is discarded (`let _ =`),
   so under contention the assertion runs before the normalizer writes and reads 0 patches. NOT
   caused by this branch — `git diff feff74be..HEAD -- crates/services/ crates/executors/` is empty,
   and it reproduces with this run's own edit stashed. Promoted to a tracked workstream created in
   THIS session (`dev-docs/workstreams/services-normalize-flaky-test/`), filed as `F-2026-08-04-02`.
   Not suppressed — no `#[ignore]`, the test stays live.

2. **`~/.config/pnpm/rc` sets a global `virtual-store-dir` to a temp DAG worktree.** Outside the
   repo and outside the diff, but it made the whole frontend suite report "no tests, 37 errors"
   after `/tmp` cleanup gutted the store. Worked around locally
   (`--virtual-store-dir=node_modules/.pnpm`); suite restored to 37 files / 433 tests. Surfaced to
   the user — it affects every pnpm project on that machine, so the fix is theirs to make, not a
   repo change.

3. **Subagent dispatch failed for all three code-review finders** (`cr-rust`, `cr-tests`,
   `cr-frontend`), as it did earlier for the Stage-2 panel `panel-501-a` on tasks 501/502. Appears
   systemic to this session. Consequence: round 1's findings come from the orchestrator's own
   inline review, which is weaker than independent review. The uncovered areas are enumerated in
   `reviews/code-review-round-1.md` under "NOT independently covered" — that list, not this ledger,
   is the place to start a re-review.
