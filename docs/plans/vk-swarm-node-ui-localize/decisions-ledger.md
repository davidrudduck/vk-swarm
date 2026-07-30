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
