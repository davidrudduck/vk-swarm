# Decisions Ledger

## Submission
Plan accepted from submit envelope.

## Decomposition tournament closure

Tournament 1 closed on 2026-08-21. Durable report: `reviews/tournament-1.md`.

- Codex: 5 validated issues + 5 validated fixes = 10.
- Claude: 9 validated issues + 4 validated fixes = 13.
- OpenCode/GLM: 5 validated issues + 4 validated fixes = 9.
- All peer-validated findings were repaired through the strict plan envelope and resubmitted; no generated task was hand-edited.
- Two independently verified defects from an invalid partial seat (task-007 base64 token fixtures and nested optional node context) were also repaired rather than deferred.
- The restart-port supplemental finding was disproved against the final plan: task 006 already awaits old-server completion and explicitly permits port reuse.

## Review-time decisions

1. Keep task 013 as one cross-layer vertical slice and keep `execution_process_id` required. An optional legacy unscoped connection-token fallback would contradict frozen D7 exact resource scoping.
2. Keep the WAI framework convention that append-only plan-ledger writes do not need every task to list the ledger in `files:`. `task-gate.sh` explicitly exempts `docs/plans/$TOPIC/*` and separately validates append-only ledger writes.
3. Keep task 003/004 persisted-state concurrency proofs. sqlx-sqlite 0.8.6 applies its default 5-second busy timeout to every established connection, disproving the alleged arbitrary unconfigured pooled-connection failure mode.
4. Use stable unversioned `$HOME/.agents/wai/scripts` commands everywhere. The active WAI submitter was corrected before strict resubmission so all 21 generated Done-when footers use that path; no plugin cache version is embedded in the deliverable.
5. General Hive outage continuity belongs to task 015; task 018 scans concrete browser-visible token surfaces and uses a separate valid session so Hive disconnect actually executes.

## Sibling-alignment advisory acknowledgements

`wai-plan-lint.sh` reports one rotating alphabetical neighbour for new files. Adding each arbitrary neighbour only advances the advisory to another file, so the plan lists actual pattern siblings and records why the reported neighbours are not patterns:

- Task 001 — `crates/db/migrations/20250621120000_relate_activities_to_execution_processes.sql` destructively drops/recreates a table and uses `DATETIME DEFAULT datetime('now')`; task 001 is strictly additive and uses caller-bound integer epoch milliseconds.
- Task 008 — `crates/server/src/routes/breakdown.rs` is a domain router, not top-level public/protected composition or API fallback. `crates/server/tests/projects_with_stats.rs` tests ordering/counts/response fields, not authorization or fallback.
- Tasks 009, 011, 013, 014, 015 and 018 — `crates/server/tests/nodes_routes.rs` checks the ordinary `/api/nodes` response contract. It contains no browser OAuth, cookies, protocol upgrade, proxy audience, restart continuity, or sentinel-disclosure pattern.
- Task 013 — `frontend/src/hooks/useActivityDismiss.ts` is a REST mutation/cache-invalidation hook, not a stream or token-lifecycle sibling.
- Task 016 — `frontend/src/lib/api/breakdown.test.ts` is a downstream `makeRequest` consumer, not an auth-boundary or centralized unauthorized-event pattern.
- Task 019 — `scripts/dev-swarm-setup.sh` is interactive and state-mutating; the verifier and its fixture test are read-only deterministic checks.
- Task 020 — `docs/configuration-customisation/database-performance.mdx` shares only generic MDX structure. `network-access.mdx`, already named by the task, is the topical trusted-LAN sibling.

## SQL-anchor advisory

No live SQLite schema checker is configured for WAI, so plan lint cannot dynamically validate new SQL literals. This is acknowledged rather than silently treated as green: task 001 owns the only new schema, and all persistence tests use the repository's migrated `create_test_pool()` rather than duplicated test DDL.

## Execution boundary

Task 001 is the sole irreversible task. The operator explicitly approved it, and the approval was recorded at `docs/plans/local-node-browser-oauth/reviews/001.approved` at `2026-08-22T01:32:37+00:00` before the migration was created.

## Task 001 — additive browser-auth migration

- TDD RED observed before the migration existed: `cargo test -p db test_utils` failed only `browser_auth_migration_creates_owner_handoff_and_session_tables` with `migration did not create node_owner`; 2 existing tests passed.
- TDD GREEN after adding `20260821000000_add_browser_auth.sql`: the focused run passed all 3 `test_utils` tests.
- `cargo test -p db` passed all eight targets: 286 unit tests, 8 bulk-operation tests, 1 emission-conformance test, 6 SQLite pragma tests, 8 task-timestamp tests, 8 variable-inheritance tests, 5 visibility tests, and 11 doctests; zero failures.
- Stage-1 deterministic gate output:

```text
WAI gate: topic=local-node-browser-oauth task=001 commit=HEAD allowed_change=mixed
  - irreversible: approval token present
  - file-set: only declared files changed (3 paths)
  - mixed: structural check relaxed — relies on adversarial panel
WAI gate: typecheck (override): cargo fmt --all -- --check ...
  - typecheck: override command exit 0
WAI gate: running tests for scope 'crates/db/src/test_utils.rs' ...
  - tests: scope 'crates/db/src/test_utils.rs' green
CONFORMS: task 001 passed all deterministic gates
GATE_FAIL_CHECK=none
```

- Stage-2 Opus adversarial review verdict: `CONFORMS`. It verified the migration SQL is byte-identical to the task contract, additive-only, highest-versioned, free of existing table-name collisions, structurally enforces the owner singleton and handoff states, stores integer epoch-millisecond timestamps without timestamp defaults, gives sessions no expiry column, and changes no undeclared production file.
- No undictated implementation choice was made. The test was appended at the actual end of the existing test module after `test_template_reuse`; the task's stale line anchor said the module ended after `test_create_test_pool`, but its required behavior and exact test remained unambiguous.

## Task 002 — execution-time plan correction

The task added `base64` to `crates/server/Cargo.toml` but omitted the repository lockfile from `files:` and allowed moves. Running the specified Cargo checks correctly regenerated the existing `server` package entry in `Cargo.lock` by adding only `"base64"`; no package version or resolution changed. The task contract was amended before validating implementation so the required generated lockfile change is explicit and gated rather than silently left dirty or deferred.

## Task 002 — deterministic browser-auth seams

- Constrained implementer followed the specified test-first sequence and reported the five colocated tests RED before completing the interfaces, then GREEN after the minimal implementation.
- Focused verification passed: `cargo test -p server auth::seams` ran all five task tests successfully.
- `cargo clippy -p server --all-targets --all-features -- -D warnings` passed.
- Stage-1 deterministic gates passed independently for source commit `6425a16c2d1a201b51654dd7c5deab50fb98ed06` and generated-lockfile commit `2d8b3aba`:

```text
WAI gate: topic=local-node-browser-oauth task=002 commit=6425a16c2d1a201b51654dd7c5deab50fb98ed06 allowed_change=mixed
  - file-set: only declared files changed (4 paths)
  - mixed: structural check relaxed — relies on adversarial panel
WAI gate: typecheck (override): cargo fmt --all -- --check ...
  - typecheck: override command exit 0
WAI gate: running tests for scope 'crates/server/src/auth/seams.rs' ...
  - tests: scope 'crates/server/src/auth/seams.rs' green
CONFORMS: task 002 passed all deterministic gates
GATE_FAIL_CHECK=none
WAI gate: topic=local-node-browser-oauth task=002 commit=2d8b3aba allowed_change=mixed
  - file-set: only declared files changed (1 paths)
  - mixed: structural check relaxed — relies on adversarial panel
WAI gate: typecheck (override): cargo fmt --all -- --check ...
  - typecheck: override command exit 0
WAI gate: running tests for scope 'crates/server/src/auth/seams.rs' ...
  - tests: scope 'crates/server/src/auth/seams.rs' green
CONFORMS: task 002 passed all deterministic gates
GATE_FAIL_CHECK=none
```

- Stage-2 Opus adversarial review verdict: `CONFORMS`. It independently reran format, focused tests, and clippy; verified exact public interfaces, CSPRNG/base64url behavior, atomics and mutex soundness, byte-identical SHA-256 encoding versus `routes/oauth.rs`, integration-test visibility, and the one-line lockfile update.
- No undictated implementation choice was made beyond the explicit lockfile plan correction recorded above.

## Task 003 — atomic owner pin-or-compare

- Constrained implementer followed the task's test-first sequence and implemented only the exact owner model/export surface.
- The focused four-test suite passed three consecutive runs; the concurrent first-pin test remained stable.
- Runtime SQLx macro scan returned no matches under `crates/db/src/models/browser_auth/`.
- Stage-1 deterministic gate output:

```text
WAI gate: topic=local-node-browser-oauth task=003 commit=54936598c5cafa8cd425bd92d2b04126d0d1e5ca allowed_change=mixed
  - file-set: only declared files changed (3 paths)
  - mixed: structural check relaxed — relies on adversarial panel
WAI gate: typecheck (override): cargo fmt --all -- --check ...
  - typecheck: override command exit 0
WAI gate: running tests for scope 'crates/db/src/models/browser_auth/owner.rs' ...
  - tests: scope 'crates/db/src/models/browser_auth/owner.rs' green
CONFORMS: task 003 passed all deterministic gates
GATE_FAIL_CHECK=none
```

- Stage-2 Opus adversarial review verdict: `CONFORMS`. It independently verified one-statement atomicity, incumbent-owner return without value mutation, mismatch side-effect freedom, BLOB UUID codec compatibility, SQLite race behavior, exact public exports, all three sibling patterns, no macro SQLx forms, and no undeclared files. It additionally ran the full DB suite and workspace clippy clean.
- No undictated implementation choice was made.

## Task 004 — durable atomic handoff claim

- Constrained implementer followed the prescribed test-first sequence: the colocated tests failed because `create_handoff`/`claim_handoff` did not exist, then passed after the exact runtime-SQLx implementation landed in commit `acfbf691ff91af0e258187783cb595e2e80806da`.
- `cargo test -p db browser_auth` passed three consecutive runs (10 tests per run), including all five handoff tests and the prior owner/migration coverage. `cargo clippy -p db --all-targets --all-features -- -D warnings` passed. Macro-form SQLx and `Utc::now` scans returned no matches.
- Stage-1 deterministic gate output:

```text
WAI gate: topic=local-node-browser-oauth task=004 commit=HEAD allowed_change=mixed
  - file-set: only declared files changed (2 paths)
  - mixed: structural check relaxed — relies on adversarial panel
WAI gate: typecheck (override): cargo fmt --all -- --check ...
  - typecheck: override command exit 0
WAI gate: running tests for scope 'crates/db/src/models/browser_auth/handoff.rs' ...
  - tests: scope 'crates/db/src/models/browser_auth/handoff.rs' green
CONFORMS: task 004 passed all deterministic gates
GATE_FAIL_CHECK=none
```

- Stage-2 Opus challenger verdict: `CONFORMS`. It verified the exact two-file scope, API and SQL text, one-statement terminal claim, strict expiry boundary, wrong-browser non-consumption, replay rejection, explicit caller time, raw verifier handling, runtime query forms, sibling alignment, and non-hollow mutation behavior.
- Stage-2 GPT challenger returned `DEVIATES` with two proposed test changes. Both were independently dismissed as contract/spec false positives:
  1. It objected that the concurrent test permits the losing future to return a database error. The authoritative task itself prescribes `r1.as_ref().ok().map_or(...)` and explicitly says, “Persisted state is the real proof, however the loser failed” (`004-...md:89-97`). The implementation is byte-faithful; changing this would contradict the frozen task rather than remediate a deviation. SQLx supplies a five-second default SQLite busy timeout, and three repeated runs produced two successful claim futures with exactly one winner.
  2. It requested a model-layer raw-binding-token persistence test. `create_handoff` accepts only `binding_hash`, so raw-cookie absence cannot be meaningfully established at this seam; such a test would never feed the raw secret to production code and would be hollow. The real hash-only production path is already decision-locked and tested by task 009: `initiation_issues_a_binding_cookie_and_persists_only_its_hash` compares the stored value to `hash_token(&raw)` and rejects equality with the raw cookie (`009-...md:27-52`). Task 004 proves the persistence/claim semantics of the supplied hash; task 009 proves that only a hash is supplied. This is the TS1 split encoded by the plan, not deferred remediation.
- No undictated implementation choice was made.

## Task 005 — persistent browser sessions and revocation

- Constrained implementer followed the exact test-first sequence and changed only `browser_auth/session.rs` plus its module export in commit `7c8cf142`.
- The five focused session tests and the complete 15-test `browser_auth` group passed. `cargo clippy -p db --all-targets --all-features -- -D warnings` passed. Macro-form SQLx and `expires_at` scans returned no matches.
- Stage-1 deterministic gate output:

```text
WAI gate: topic=local-node-browser-oauth task=005 commit=HEAD allowed_change=mixed
  - file-set: only declared files changed (2 paths)
  - mixed: structural check relaxed — relies on adversarial panel
WAI gate: typecheck (override): cargo fmt --all -- --check ...
  - typecheck: override command exit 0
WAI gate: running tests for scope 'crates/db/src/models/browser_auth/session.rs' ...
  - tests: scope 'crates/db/src/models/browser_auth/session.rs' green
CONFORMS: task 005 passed all deterministic gates
GATE_FAIL_CHECK=none
```

- Both independent Stage-2 challengers returned `CONFORMS`. They verified the exact API/export and two-file scope; local live-session authorization without time, expiry or Hive state; token-hash-scoped idempotent revocation preserving the first timestamp; live-only all-session revocation and exact count; uniqueness; runtime SQLx; no DELETE/owner/credential/sync writes; sibling guard alignment; and discriminating tests.
- `create_session` uses `INSERT ... RETURNING` to satisfy its required `Result<BrowserSession, sqlx::Error>` signature. The task fixes the signature but does not dictate insert SQL; `RETURNING` is the existing browser-auth house pattern in owner and handoff models. This is recorded as an explicit, reversible implementation choice rather than silently claiming there was none.

## Integrated phase-1 review — disconnect/login serialization

- Cross-model review over `41f55c4b..8fd674a8` produced two `CONFORMS` verdicts and one cited
  `DEVIATES`. The cited finding was independently reproduced: revoking all rows cannot revoke a
  session inserted afterward (`revoke_all_rows=1 live_after_disconnect=['after']`). Under the
  original locked tasks, a callback paused in Hive I/O could recreate credentials, session and
  sync after explicit disconnect returned, violating SC8.
- Investigation found two coupled real races: detached `spawn_remote_sync` can install after
  disconnect observes an empty slot, and in-flight refresh can save credentials after clear.
- Remediation is corrective task 022 plus amended tasks 009–012: a per-deployment in-memory commit
  epoch, durable invalidation of pending handoffs using existing terminal `claimed`, synchronous
  login-path sync installation, and precise use of `AuthContext::refresh_guard` around credential
  commit/clear. No new schema is required.
- Initiation linearizes at `create_handoff`. A handoff inserted before disconnect is invalidated;
  one inserted after disconnect is a legitimate fresh login. Claim plus epoch capture is one short
  guarded DB section. Callback Hive I/O is deliberately unlocked; callback commit re-checks the
  epoch before any daemon/session side effect.
- Durable generation was rejected because no approved schema can store it; adding a table/column
  would exceed task 001's irreversible approval. The in-memory epoch is sufficient while one node
  process owns the SQLite file, and durable pending-handoff invalidation covers restart.
- O8 residual accepted and explicit: a process crash between SQLite revoke-all and file/Keychain
  credential clear can leave an over-locked-out node with credentials present. The operator retries
  disconnect. A durable crash-recovery state would require a separately approved migration.
- Full report: `.agents/reports/2026-08-22-round-1-cross-model-phase-one.md`.

## Task 022 — disconnect fence primitives and startup linearization

- Task 022 added durable pending-handoff invalidation, a per-deployment browser-auth epoch, and a
  synchronous `install_remote_sync` path. Source commits are `6eece603`, formatting follow-up
  `a32804bc`, startup linearization fix `cc70f9d7`, and Stage-2 compatibility/safety remediation
  `94e5aecc`; plan corrections are `fed8958d`, `3e10fd1f`, and `5922e7b3`.
- The initial Stage-2 observation that overwriting a `RemoteSyncHandle` necessarily leaves an
  unreachable live task was disproved at `crates/services/src/services/share.rs:682-689`:
  `RemoteSyncHandleInner::drop` sends shutdown and aborts the join handle. A distinct race was real:
  detached configured startup could return before installing its handle, allowing disconnect to
  observe an empty slot before the late install. `LocalDeployment::from_parts` now awaits
  `install_remote_sync` before returning, and the current-thread constructor test proves the handle
  is observable immediately.
- Stage-2 remediation preserved the pre-task client boundary by injecting raw `api_base` separately
  from parsed `ShareConfig`. `raw_api_base_remains_available_when_share_sync_config_is_unavailable`
  proves a parseable raw base still configures `RemoteClient` even when sync configuration is absent.
- The direct constructor tests now call the same process-wide orphan-cleanup guard as `for_test()`.
  Review also reproduced a pre-existing exposure in `LocalContainerService::new_for_drain_test()`;
  it is explicitly split and tracked in
  `dev-docs/workstreams/local-deployment-test-orphan-cleanup-safety/README.md` rather than deferred
  silently inside the browser-auth task.
- The final source commits were gated explicitly rather than relying on plan-only `HEAD`: the
  behavioral remediation `94e5aecc` and comment-only follow-up `53a962b8` each passed the task gate.
  Final gate transcript for `53a962b8`:

```text
WAI gate: topic=local-node-browser-oauth task=022 commit=53a962b8 allowed_change=edit
  - file-set: only declared files changed (1 paths)
  - edit: structural check relaxed — relies on adversarial panel
WAI gate: typecheck (override): cargo fmt --all -- --check ...
  - typecheck: override command exit 0
WAI gate: running tests for scope 'crates/db/src/models/browser_auth/handoff.rs' ...
  - tests: scope 'crates/db/src/models/browser_auth/handoff.rs' green
CONFORMS: task 022 passed all deterministic gates
GATE_FAIL_CHECK=none
```

- `cargo test -p db browser_auth` passed all 17 tests. The epoch clone, configured startup install,
  and raw API-base compatibility tests each passed focused runs. `cargo clippy -p db -p deployment
  -p local-deployment --all-targets --all-features -- -D warnings` and
  `cargo fmt --all -- --check` passed with repo-local `TMPDIR` used to avoid the host `/tmp` quota.
- Final independent Stage-2 convergence returned two `CONFORMS` verdicts. The panel verified all
  prior deviations closed, the startup linearization remained intact, the four declared source
  files and STOP triggers remained respected, and the pre-existing cleanup hazard had a legitimate
  tracked scope split. The only final observation was a displaced test-helper doc comment; commit
  `53a962b8` moved it back onto `for_test()` without changing behavior and was re-gated above.

## Integrated phase-1 review round 2 remediation

- The eight-seat review of `41f55c4b..ae5ee15f` confirmed the phase-1 browser-auth state machines,
  epoch, startup sync linearization, raw API-base compatibility, task scopes, and STOP triggers.
  Route wiring in tasks 009–012 is still required before SC8 is complete.
- The review found that task 022's startup fixture could select the production macOS Keychain
  backend before saving `test-refresh-token`. Commit `594d531c` added the explicit
  `OAuthCredentials::new_file_backed()` constructor and changed both direct constructor fixtures to
  use isolated temporary paths. `OAuthCredentials::new()` and backend detection are unchanged.
  RED was E0599 for the missing constructor; the focused service test, both constructor tests, and
  focused services/local-deployment clippy were green after implementation.
- The review also enforced the previously promised scope split
  `sqlite-busy-snapshot-calibration-stability`. The pre-existing calibration failure reproduced at
  `crates/db/src/models/execution_process/queries.rs` in
  `control_read_then_write_shape_reproduces_busy_snapshot`, with 0/200
  `SQLITE_BUSY_SNAPSHOT` observations in the old scheduler-sensitive control. It is tracked at
  `dev-docs/workstreams/sqlite-busy-snapshot-calibration-stability/README.md` and fixed in this
  branch by forcing the read-snapshot/intervening-commit/write-upgrade schedule in both calibration
  controls. The two controls passed together in ten consecutive runs; no test was ignored.
- Task 022's post-review source commit `594d531c` passed its explicit deterministic gate:

```text
WAI gate: topic=local-node-browser-oauth task=022 commit=594d531c allowed_change=edit
  - file-set: only declared files changed (2 paths)
  - edit: structural check relaxed — relies on adversarial panel
WAI gate: typecheck (override): cargo fmt --all -- --check ...
  - typecheck: override command exit 0
WAI gate: running tests for scope 'crates/db/src/models/browser_auth/handoff.rs' ...
  - tests: scope 'crates/db/src/models/browser_auth/handoff.rs' green
CONFORMS: task 022 passed all deterministic gates
GATE_FAIL_CHECK=none
```

- After deterministic calibration repair `cargo test -p db` passed: 302 unit tests, 8 bulk
  operation tests, the emission conformance test, 6 pragma tests, 8 execution-timestamp tests, 8
  variable-inheritance tests, 5 visibility tests, and 11 live doctests; no failures. Existing
  selectively ignored deprecated remote-cache and environment-dependent doctests were unchanged.
- Remediation re-review confirmed both production fixes but rejected two evidence claims. The
  SQLite controls force the hazard directly; they do not use or calibrate the unchanged,
  scheduler-sensitive background-writer generators in the real-function stress tests. The test
  comments and tracked workstream now state that limitation explicitly. The credential regression
  test now inspects `OAuthCredentials`' private backend before saving, requires `Backend::File`, and
  verifies the exact configured path; on macOS a Keychain selection therefore fails without a
  Keychain write.
- The strengthened credential test passed task 022's deterministic gate on commit `9d705438`:

```text
WAI gate: topic=local-node-browser-oauth task=022 commit=HEAD allowed_change=edit
  - file-set: only declared files changed (1 paths)
  - edit: structural check relaxed — relies on adversarial panel
WAI gate: typecheck (override): cargo fmt --all -- --check ...
  - typecheck: override command exit 0
WAI gate: running tests for scope 'crates/db/src/models/browser_auth/handoff.rs' ...
  - tests: scope 'crates/db/src/models/browser_auth/handoff.rs' green
CONFORMS: task 022 passed all deterministic gates
GATE_FAIL_CHECK=none
```

## Integrated phase-1 informational-finding closure

- Startup diagnostics now derive `has_shared_api` from the already-resolved raw API-base
  dependency. This keeps injected tests and compile-time fallback configuration aligned with the
  remote client that was actually constructed. The orphan-cleanup SAFETY comment no longer claims
  that `from_parts` parses `ShareConfig`; production parsing moved to `new()` during task 022.
- The task-022 examples now destructure `create_test_pool()`'s real `(pool, TempDir)` return value,
  and its manual clippy command includes `services`, which task 022 added to its file set.
- The contention-pool comment now distinguishes its explicit five-second busy timeout and ten
  connections from production's thirty-second timeout and the shared helpers' SQLx-default
  five-second timeout and five connections. A busy timeout retries plain `SQLITE_BUSY`; it does
  not prevent extended code 517 after an invalid read-snapshot/write-upgrade.
- Durable terminal handoff rows and revoked session rows are intentionally preserved by phase 1;
  tasks 004, 005 and 022 explicitly prohibit deletion. Their eventual retention policy requires a
  product/storage decision and is tracked as the legitimate scope split
  `browser-auth-terminal-row-retention` at
  `dev-docs/workstreams/browser-auth-terminal-row-retention/README.md`.
- The task-022 `siblings:` concern is non-actionable: it edits existing modules, introduces no new
  sibling file, and current plan lint emits no task-022 sibling advisory. The synchronous
  `RemoteSync::spawn` URL-parse panic concern is non-actionable because `ShareConfig.api_base` is
  already a parsed `Url`; serializing that value yields a parseable URL for the spawn path. The raw
  client base and parsed sync config are independent injected dependencies, so no stronger
  same-bytes invariant is claimed.
- The phase-1 tests now prove the persistence semantics that the STOP triggers require. Commit
  `16be3a50` asserts that invalidation preserves the handoff row in terminal `claimed` state;
  commit `4c80e19e` asserts that revoke-all preserves both session rows, sets the live row's
  timestamp, and leaves the already-revoked timestamp unchanged. DELETE mutants no longer satisfy
  these tests.
- The configured-startup test's node-cache task is runtime-scoped test hygiene, not leaked process
  state: it uses a private temporary database, the Tokio test runtime aborts remaining tasks at
  teardown, and no deployment-level node-cache stop API exists. The test explicitly shuts down the
  RemoteSync handle and event bus; adding a production shutdown surface solely for this isolated
  fixture would exceed the browser-auth contract.
- Final focused review found the sibling `queries.rs` contention-pool comment still overstated
  production timeout parity and its assertion still said “calibration control.” Both DB controls
  now use the same accurate five-second-test/thirty-second-production wording and “deterministic
  hazard control” terminology.

## Task 006 structured node-cache lifecycle correction

- The earlier phase-1 dismissal at lines 328-332 is superseded. At that point node-cache shutdown
  was only test hygiene for an isolated startup fixture. Task 006's restart/outage harness and task
  012's explicit Hive-disconnect outcome make lifecycle ownership production-relevant: node-cache
  is authenticated Hive synchronization and must not survive a completed explicit disconnect or a
  planned deployment restart.
- Final task-006 review disproved the request-count quiescence heuristic in commit `3de79b63`.
  Replacement `RemoteSync` migrates unlinked projects through `GET /v1/organizations`, the same
  path used by node-cache startup, so a `baseline + 2` barrier can count one request from each
  service and release before node-cache's immediate interval tick. The detached old-generation
  node-cache task also has no join/stop owner and can outlive `HiveHarness::restart()`.
- User approved structured lifecycle ownership rather than more scheduler instrumentation.
  `NodeCacheSyncService::spawn()` therefore returns a cancellation-plus-join
  `NodeCacheSyncHandle`; `LocalDeployment` retains it in a clone-shared optional slot, exposes an
  awaited shutdown seam, and takes the slot before awaiting. Cancellation interrupts both
  in-flight sync I/O and the five-minute interval wait; handle drop aborts only as a final safety
  net. Existing `run()` and `stop()` behavior remains available to existing callers.
- Task 006 now proves refresh-request provenance after seeding an unlinked project and restarting:
  the harness awaits both current-generation RemoteSync and node-cache shutdown, writes
  refresh-only credentials, observes exactly the explicit caller's first refresh attempt, then
  aborts/awaits its retrying task. Request counts remain observations, never lifecycle barriers.
- Task 012 now interprets SC8 “stop synchronization” as both RemoteSync and node-cache. Browser
  logout leaves both running; explicit Hive disconnect awaits both; a fresh same-owner login can
  start both again. Task 015 uses the same owned shutdown seam after its seeded restart.
- The implementation awaits `start_node_cache_sync()` inside `from_parts` instead of detaching the
  handle-slot installation. `NodeCacheSyncService::spawn()` itself returns without awaiting network
  work, so startup latency is unchanged in substance; the ordering guarantees a completed
  constructor cannot later install a node-cache handle after restart/disconnect already observed an
  empty slot. This is the node-cache analogue of task 022's synchronous RemoteSync startup install.

## Task 006 closure — gates, mutation evidence, panel verdicts

Source provenance (plan commits interleave, so gates ran per source commit against the plan state
of record): `a62162ab`/`3fc19fbf` gated against `c5762b63`, `9d8fbf23` gated against `743ee1bc`,
`3de79b63` gated against `13fcf4d1`, `364a0e47` gated against `8af96e52`, `737f01ee` gated against
`364a0e47` — every run `CONFORMS`, `GATE_FAIL_CHECK=none`. Final two gates (verbatim):

```
WAI gate: topic=local-node-browser-oauth task=006 commit=364a0e47 allowed_change=edit
  - file-set: only declared files changed (4 paths)
WAI gate: typecheck (override): cargo fmt --all -- --check ...
  - typecheck: override command exit 0
WAI gate: running tests for scope 'crates/server/tests/harness_smoke.rs' ...
  - tests: scope 'crates/server/tests/harness_smoke.rs' green
CONFORMS: task 006 passed all deterministic gates
GATE_FAIL_CHECK=none
```

```
WAI gate: topic=local-node-browser-oauth task=006 commit=737f01ee allowed_change=edit
  - file-set: only declared files changed (1 paths)
WAI gate: typecheck (override): cargo fmt --all -- --check ...
  - typecheck: override command exit 0
WAI gate: running tests for scope 'crates/server/tests/harness_smoke.rs' ...
  - tests: scope 'crates/server/tests/harness_smoke.rs' green
CONFORMS: task 006 passed all deterministic gates
GATE_FAIL_CHECK=none
```

Idle-interval mutation evidence for `737f01ee` (strengthened
`shutdown_interrupts_the_idle_interval_wait` observes the compound DB state — second node cached,
first node remove_stale-deleted — proving both startup passes completed and the service is parked
in the five-minute wait before the shutdown timeout): with the interval-wait cancellation arm
replaced by a plain `interval.tick().await`, the idle test failed with `shutdown must cancel the
idle interval wait instead of the next tick: Elapsed(())` while
`shutdown_interrupts_an_in_flight_sync` still passed; restoring the arm made all three lifecycle
tests green and the idle test green 10/10 consecutive runs. The two tests are mutually
discriminating: the in-flight test catches the do_sync-arm mutant, the idle test catches the
tick-arm mutant.

Stage-2 adversarial panel (two independent families) both returned `VERDICT: CONFORMS` over the
full source range `c0ddf51a..737f01ee`: concurrency soundness verified (biased selects cannot miss
cancellation; `run()` holding the sender preserves never-cancelled semantics including the
immediate first tick and stop-flag path; slot-fill vs shutdown race closed by the slot mutex;
`shutdown_node_cache_sync` drops the guard before awaiting; no lock cycles among the three
mutexes); strengthened idle test reasoning verified sound (no await between the terminal
`remove_stale` commit and the interval select); `from_parts` awaited start adds no network wait;
harness/restart/helper ordering matches the amended contract; request-count heuristics fully
removed; consumer repointing intact (14/14 COOKIE builders, `delete_with` everywhere, zero
assertion drift); full regression sweep green (services node_cache 3/3, local-deployment 43/43,
harness_smoke 11/11, full server, clippy `-D warnings`, fmt, diff-check). One pre-existing
observation retained as a non-finding: the harness share-sync `if let Some(h) = …take()` pattern
holds the share-sync slot guard across `shutdown().await` — no cycle exists because the RemoteSync
task never re-acquires that lock, and the plan's take-before-await constraint targets the
node-cache method.

## Task 007 — cookie helpers, session resolver, scoped auth middlewares

Cross-class evidence (manual verification 4). The two cross-audience assertions and the
wrong-resource assertion from
`crates/server/src/auth/node_token.rs::tests::each_predicate_requires_its_own_audience_node_and_resource_scope`:

```rust
assert!(!connection_token_is_valid_for_resource(
    &v, Some(&proxy), expected_node, resource));   // proxy aud never opens the connection surface
assert!(!proxy_token_is_valid_for_node(&v, Some(&conn), expected_node)); // connection aud never opens the proxy surface
assert!(!connection_token_is_valid_for_resource(
    &v, Some(&conn), expected_node, other));       // right aud+node, wrong resource: rejected
```

Audiences are set by the validator at
`crates/services/src/services/connection_token.rs:106` (`set_audience(&["connection"])` in
`validate()`) and `crates/services/src/services/connection_token.rs:201`
(`set_audience(&["node_proxy"])` in `validate_proxy_token()`). The receiving middlewares call
only the strict `validate_for_resource` (connection_token.rs:157) and `validate_proxy_for_node`
(connection_token.rs:227); the loose audience-decoding methods are never used receiver-side.
`git grep -n 'Secure' crates/server/src/auth/cookies.rs` shows the D9 doc comment and the test's
negative assertion only — no emitted `Secure` attribute.

Choices the task did not fully dictate:

- The task's node_token test sketch used undeclared `secret()`/`SECRET` placeholders; bound them
  to the task's own `test_secret()` fixture (`let secret = test_secret();` passed by reference),
  per the task's "no undeclared secret()/SECRET placeholder remains" rule.
- Appended two positive-control service tests (`test_validate_for_resource_accepts_exact_node_and_resource`,
  `test_validate_proxy_for_node_accepts_exact_target`) alongside the mandated rejection tests:
  rejection-only tests cannot distinguish a strict validator from one that always errors.
- `resolve_browser_session` maps a DB error to `None` (fail closed) with a `tracing::warn!`
  carrying only the error, never the presented token.

### Task 007 closure — gates, mutation evidence, panel verdicts

Source provenance (plan commits interleave, gated per-commit):
- `06fa46d6` (implementation) vs base `9daad75a`: 6 paths (5 declared files + ledger).
  Gate transcript: `WAI gate: topic=local-node-browser-oauth task=007 commit=HEAD allowed_change=mixed
  - file-set: only declared files changed (6 paths)
  - mixed: structural check relaxed — relies on adversarial panel
  - typecheck (override): cargo fmt --all -- --check ... exit 0
  - tests: scope 'crates/server/src/auth/session.rs' green
  CONFORMS: task 007 passed all deterministic gates / GATE_FAIL_CHECK=none`
  (WAI_TEST_CMD="cargo test -p server auth:: && cargo test -p services connection_token":
  server 11 passed / services 9 passed at review time).
- `6e760955` (test-strength remediation) vs base `46fa0e83` (plan amendment): 2 paths.
  Same gate command shape → CONFORMS, GATE_FAIL_CHECK=none (server 12 passed).

Stage-2 panel (over 9daad75a..06fa46d6):
- subagent-gpt: DEVIATES with two SHOULD-FIX test-strength findings — (1) clear-cookie test
  substring-only, a `; Secure` mutant would pass; (2) no discriminating test for the resolver's
  DB-error fail-closed branch.
- subagent-kimi: CONFORMS — cookie bytes byte-exact, resolver hashes-then-lookups with stored-hash
  replay craft covered, revocation enforced in SQL, privilege separation verified (predicates call
  only validate_for_resource/validate_proxy_for_node; cross-audience rejection asserted both
  directions), middleware mechanics match the compile-order contract, no secrets in any warn!/error
  text, mutant table for every STOP trigger; 3 INFO (single Cookie header field read — fail-closed;
  middleware body unit tests deferred to tasks 008/013/014 by contract line 23; duplicate cookie
  name first-wins benign). All three INFO items are plan-sanctioned or fail-closed; none carried
  forward.

Remediation (plan `46fa0e83`, source `6e760955`, only cookies.rs + session.rs tests):
- `clear_cookie_expires_immediately` now byte-exact
  (`vks_browser_session=; HttpOnly; SameSite=Lax; Path=/; Max-Age=0`) + `!contains("Secure")`.
  Mutation evidence: injecting `; Secure` → FAILED assert_eq (left showed injected attribute);
  implementation unchanged, test passed on arrival once aligned.
- New `resolver_fails_closed_when_the_database_errors` (pool.close().await → is_none()).
  Mutation A `.expect("db")` → panic `db: PoolClosed` (test FAILED). Mutation B fabricated
  `Some(BrowserSessionCtx{..})` on error → `is_none()` FAILED. Restored → GREEN. No
  production-code changes in the remediation commit (verified by reviewer diff).
- Focused re-review by the dissenting seat (subagent-gpt) over 46fa0e83..6e760955:
  VERDICT: APPROVE — both findings closed, nothing new.

Final state: server auth:: 12 passed / 0 failed; services connection_token 9 passed / 0 failed;
clippy `-p server -p services --all-targets --all-features -D warnings` clean; fmt clean;
git diff --check clean at 6e760955. Task 007 frontmatter status: passed.

## Task 008 — public/protected router split, API-boundary 404, and two undictated corrections

Two choices the task file did not dictate, both forced:

1. **Test compile fix (mechanical).** The spec's `oauth_initiation_and_callback_stay_public` calls
   `h.get("/api/auth/handoff/complete?handoff_id="`.to_string() + &uuid…)` but
   `HiveHarness::get` takes `&str` (common/mod.rs:366). Applied the compiler's minimal fix —
   wrap the expression in `&(...)` (common/mod.rs:550 `get_with` has the same signature). No
   assertion or semantic change; identical URL reaches the server.

2. **`api_not_found` is registered as a catch-all ROUTE inside the nest, not only as the nested
   fallback.** Plan-verbatim `.fallback(api_not_found)` on `base_routes` leaves
   `unknown_api_paths_terminate_inside_the_api_boundary` RED (observed: `unknown /api/* fell
   through to SPA HTML (status 200, ct Some("text/html"))`). Mechanism, axum 0.8.8 source:
   `Router::nest` (routing/mod.rs:225-228) files a nested custom fallback under the PARENT's
   `fallback_router`, which `PathRouter::call_with_state` consults only when the MAIN matchit
   tree misses — and the outer `/{*path}` real route (routes/mod.rs SPA catch-all) matches every
   path, so the nested fallback is unreachable. Fix: keep the plan's `.fallback(api_not_found)`
   AND add `.route("/{*path}", any(api_not_found))` on `base_routes`, registering
   `/api/{*path}` in the main tree where the `/api` static prefix outranks the root catch-all.
   `any` (not `get`) so non-GET methods on unknown `/api/*` paths also terminate as JSON 404
   inside the boundary instead of 405-ing the SPA route. No route conflicts: the only other
   `/{*…}` catch-alls are `/{id}/files/{*file_path}` in projects/task_attempts (deeper,
   param-prefixed) and the outer SPA route itself.

Manual verification item 5 — explicit contents of `public_routes` (routes/mod.rs:51-54):
- `GET /health` (health::health_check)
- `POST /auth/handoff/init`, `GET /auth/handoff/complete` (oauth::public_router)
- `GET /auth/state` (browser_auth::public_router)

Nothing else is in it. Every other merge entry from the original chain survives once, in the same
relative order, in `protected_routes` behind `require_browser_session`; `oauth::router()` became
`oauth::protected_router()` in place (position between task_variables and organizations unchanged).

`npm run generate-types` diff on shared/: `shared/types.ts` +2 lines — exactly
`export type BrowserAuthState = { authorized: boolean, oauth_available: boolean, };` inserted
after `CheckAgentAvailabilityQuery`. No other type changed, no schema changes, no drift.

Gates at commit: browser_auth_routes 4/4 green (RED first: 3 failed exactly as the task predicted —
auth/state SPA-fall-through, protected routes 200, unknown /api SPA fallback); `cargo test -p
server` full suite green (17 ok suites, 0 failures; only consumer change = the plan-dictated
harness_smoke inversion); `npm run generate-types:check` exit 0; `cargo fmt --all -- --check`
clean; `cargo clippy -p server --all-targets --all-features -- -D warnings` clean;
`git diff --check` clean.

### Task 008 closure — gates, mutation evidence, panel verdicts

Source commits: `a97eb6d6` (implementation, vs base `0e947ad9`) and `9bab70be` (test-only
remediation, vs base `d4f6a65e`). Plan amendments: `e5090bf8` (axum fallback correction —
JSON 404 registered as a real `/{*path}` route inside the nest because the outer SPA catch-all
shadows fallback-only registration on axum 0.8.8) and `d4f6a65e` (oauth publicness test pinned to
handler-specific outcomes). Plan lint PASS after both.

Stage-1 gates (verbatim transcripts of the two final runs):

```
WAI gate: topic=local-node-browser-oauth task=008 commit=a97eb6d6 allowed_change=mixed
  - file-set: only declared files changed (8 paths)
  - mixed: structural check relaxed — relies on adversarial panel
WAI gate: typecheck (override): cargo fmt --all -- --check ...
  - typecheck: override command exit 0
WAI gate: running tests for scope 'crates/server/tests/browser_auth_routes.rs' ...
  - tests: scope 'crates/server/tests/browser_auth_routes.rs' green
CONFORMS: task 008 passed all deterministic gates
GATE_FAIL_CHECK=none
```

```
WAI gate: topic=local-node-browser-oauth task=008 commit=9bab70be allowed_change=mixed
  - file-set: only declared files changed (1 paths)
  - mixed: structural check relaxed — relies on adversarial panel
WAI gate: typecheck (override): cargo fmt --all -- --check ...
  - typecheck: override command exit 0
WAI gate: running tests for scope 'crates/server/tests/browser_auth_routes.rs' ...
  - tests: scope 'crates/server/tests/browser_auth_routes.rs' green
CONFORMS: task 008 passed all deterministic gates
GATE_FAIL_CHECK=none
```

Orchestrator mutation check (fallback correction): removing only the
`.route("/{*path}", any(api_not_found))` line from routes/mod.rs made
`unknown_api_paths_terminate_inside_the_api_boundary` FAIL (SPA HTML fallback answered);
restored, 4/4 green. This independently reproduced the implementer's axum finding.

Stage-2 panel (parallel, `0e947ad9..a97eb6d6` against amended plan):
- subagent-kimi: VERDICT: CONFORMS. 32-entry merge-chain before/after comparison — identical
  order, no duplicates, oauth swap in place; catch-all public (layer applied before merge);
  auth-state clean (no hive call, no D8 leaks); all downstream consumers green; mutation
  reasoning table per STOP trigger; handler-body semantic-edit coverage gap noted as
  pre-existing, not introduced (diff verifiably touches no handler body).
- subagent-gpt: VERDICT: DEVIATES with one BLOCKING finding: the plan's own oauth-publicness
  test snippet used `assert_ne!(status, 401)` (and callback had no assert_registered at all) —
  false-green to route drops because the JSON-404 catch-all satisfies both. This violated the
  task's own STOP trigger (status-code-alone proves routing). Plan defect, not implementer drift.

Remediation (`9bab70be`, test-only, subagent-glm): init pinned to `assert_registered()` +
`assert_eq!(status, 200)` + body contains `handoff_id`; callback pinned to
`assert_eq!(status, 400)` + body contains `Missing app_code` (no `assert_registered` — the
handler answers HTML which `is_spa_fallback()` would misread). Mutation evidence recorded:
dropping `/auth/handoff/init` → init RED (`left: 404, right: 200`); dropping
`/auth/handoff/complete` → callback RED (`left: 404, right: 400`); both restored → 4/4 green,
harness_smoke 11/11 unchanged, fmt/clippy/diff-check clean. One mechanical deviation: `&` borrow
on the String path (E0308 with `&str` parameter), matching the original test's shape.

Focused re-review by the dissenting seat (subagent-gpt, `d4f6a65e..9bab70be`, including its own
independent mutation spot-check): VERDICT: APPROVE.

Final state: 4/4 browser_auth_routes green, full server suite green, generate-types:check green.
SC1 delivered: public/protected subtrees, deny-by-default layer, API-terminating JSON-404
boundary, minimal `/api/auth/state`. Cross-node proxy subtree breakage behind the browser layer
is INTENDED and is undone by tasks 013/014 (STOP trigger honored).

## Task 009 decisions

Mechanical compile fix beyond the plan text: the plan's new return type
`Result<axum::response::Response, ApiError>` no longer pins `ApiResponse`'s second generic
parameter (`ApiResponse<T, E = T>`, crates/utils/src/response.rs:5), so the plan-verbatim
`ApiResponse::success(body)` failed E0282 (cannot infer `E`). Fixed with the turbofish
`ApiResponse::<HandoffInitResponseBody>::success(body)`, which restores exactly the type the
handler returned before this task (`E` defaults to `T`). No behavioral or serialization change.

Formatting-only deviations: `IntoResponse` was inlined into the existing `axum::{...}` use tree
(`response::{IntoResponse, Json as ResponseJson}`) instead of a separate top-level use; and the
mandatory `cargo fmt --all` gate reflowed the plan-verbatim test snippet's line breaks
(whitespace only, no semantic change). Test logic is byte-equivalent to the plan modulo fmt.
