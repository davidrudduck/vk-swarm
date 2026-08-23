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

Stage-2 remediation (plan commit eb999901) applied to browser_oauth.rs: the two-browser test now
mounts two mocks and asserts both statuses 200 plus both cookies (`expect`) before `assert_ne`;
added `initiation_persists_the_handoff_behind_the_epoch_fence`. Mutation evidence (both mutations
temporary, both restored, oauth.rs byte-identical afterward — `git diff` empty):
(a) reverted two-browser test to a single mounted `mock_hive_oauth` →
`assertion left == right failed: browser B initiation failed: {"success":false,...,"message":"Remote service error. Please try again."}  left: 404  right: 200` — RED as required (the pre-remediation test passed vacuously here);
(b) moved `create_handoff(...)` above `deployment.browser_auth_epoch().lock().await` in
crates/server/src/routes/oauth.rs →
`panicked at crates/server/tests/browser_oauth.rs:146:5: handoff row appeared while the epoch fence was held` — RED as required. Green after restore: 3/3 passed; fence test
stable across 5 consecutive runs (5/5 passed); fmt --check, clippy -D warnings, git diff --check
all clean.

## Task 009 closure — gates, mutation evidence, panel verdicts

Source provenance: implementation `e58946c3` (task(009): bind oauth initiation to a browser; RED captured: both plan tests failed — "no binding cookie issued" and two-browser None/None), remediation `07ad0dac` (test(auth): make binding tests discriminating). Plan amendments: `eb999901` (corrected two-browser test + epoch-fence test + mutation requirements), `a6204f31` (stale "2 tests green" count → 3).

Stage-1 gate transcripts (per-commit, plan commits interleave):
- `e58946c3` vs base `91ac6483`: WAI_TYPECHECK_CMD="cargo fmt --all -- --check" WAI_TEST_CMD="cargo test -p server --test browser_oauth && cargo test -p server --test browser_auth_routes" → "file-set: only declared files changed (3 paths)… typecheck: override command exit 0… tests: scope 'crates/server/tests/browser_oauth.rs' green… CONFORMS: task 009 passed all deterministic gates. GATE_FAIL_CHECK=none".
- `07ad0dac` vs base `eb999901`: same typecheck, WAI_TEST_CMD="cargo test -p server --test browser_oauth" → "(2 paths)… exit 0… green… CONFORMS… GATE_FAIL_CHECK=none". (First attempt of this gate spuriously failed on a missing `.cargo-tmp/` scratch dir — mktemp ENOENT cascaded into an empty typecheck log; recreating the untracked dir and re-running produced the recorded CONFORMS. No source or gate semantics changed.)

Stage-2 round 1 (parallel, range 91ac6483..e58946c3): subagent-kimi CONFORMS (file scope exact; all six STOP triggers cleared with citations; guard-across-Hive-I/O disproved by await-order trace; deviations adjudicated mechanical). subagent-gpt DEVIATES with two valid findings: [BLOCKING] two-browser test false-green — `mock_hive_oauth` mounts `/v1/oauth/web/init` with `up_to_n_times(1)` (common/mod.rs:826-839), so the second initiation failed closed and `assert_ne!(Some, None)` passed vacuously; [SHOULD-FIX] no observable discriminated the create_handoff-under-epoch-fence ordering.

Remediation: plan `eb999901` + test-only source `07ad0dac`. Two-browser test now mounts two mocks, asserts both statuses 200, expects both cookies. New `initiation_persists_the_handoff_behind_the_epoch_fence` holds `browser_auth_epoch` from outside via `h.deployment()`, proves Hive I/O completed (`hive_request_count("POST","/v1/oauth/web/init") >= 1`), asserts the row cannot appear while the fence is held, then opens the fence and asserts 200 + state pending. Mutation evidence (ledger ## Task 009 decisions, decisions-ledger.md:629-639): (a) single-mock revert → two-browser FAILED "browser B initiation failed … left: 404 right: 200"; (b) create_handoff moved above the epoch lock → fence FAILED "handoff row appeared while the epoch fence was held". Both restored; oauth.rs byte-identical after (git checkout --). Fence test 5/5 stable; browser_oauth 3 green.

Focused re-review by the dissenting seat (subagent-gpt) over eb999901..07ad0dac: both prior findings verified closed, no new functional issues, flakiness hazards checked (current-thread scheduling safe; 200ms window 5/5 stable); one SHOULD-FIX doc defect — plan line 247 still said "2 tests green". Fixed in `a6204f31` (plan-lint PASS). All remediation converged in-session.

## Task 010 decisions

Mechanical deviation (sanctioned by the dispatch instruction): the plan's After block binds
`let epoch_at_claim = *epoch_guard;`, which is unused until task 011. rustc emitted
`warning: unused variable: epoch_at_claim` (crates/server/src/routes/oauth.rs:152), which
`cargo clippy -p server --all-targets --all-features -- -D warnings` promotes to an error — a
deadlock between the plan text and the clippy gate. Applied the underscore binding
`let _epoch_at_claim = *epoch_guard;`. Same value, same guard scope; task 011 renames it back
when it consumes the epoch. No other deviation from the plan text; imports were extended in
place (`browser_auth::{claim_handoff, create_handoff}`, `cookies::{BINDING_COOKIE,
binding_set_cookie, read_cookie}`) rather than duplicated.

RED evidence (observed, verbatim mechanism): the dispatch brief predicted "wrong-browser
completion currently succeeds via the in-memory take_oauth_handoff which ignores binding
cookies"; the observed baseline differs — since task 009, nothing calls `store_oauth_handoff`,
so the in-memory map is always empty and EVERY callback fail-closed with 400 "OAuth handoff not
found or already completed". Observed RED run: 5 passed / 2 failed.
- `a_copied_callback_url_cannot_be_completed_in_another_browser` FAILED at
  crates/server/tests/browser_oauth.rs:219 — rightful browser A got 400 (body: "OAuth handoff
  not found or already completed") instead of 200.
- `replaying_a_completed_callback_is_rejected` FAILED at crates/server/tests/browser_oauth.rs:253
  — first (rightful) completion got 400 instead of 200.
The two 400-expecting tests (`a_forged_binding_cookie_does_not_consume_the_handoff`,
`an_expired_handoff_cannot_be_completed`) passed vacuously in RED because everything 400'd;
they are discriminating in GREEN (a claim that ignored the binding hash or expires_at would
return 200 and flip the row to 'claimed', failing them).

Verification run (dispatch instructed NOT to run task-gate.sh; equivalent gates run directly):
`cargo test -p server --test browser_oauth` → 7 passed / 0 failed (3 from task 009 + 4 new);
`cargo fmt --all` then `-- --check` clean; `cargo clippy -p server --all-targets --all-features
-- -D warnings` clean; `git grep -n 'take_oauth_handoff' crates/server/` empty (exit 1, no
matches); `git diff --check` clean.

SC3/SC4 walk-through (manual-verification item 4):
- SC3 copied URL: `a_copied_callback_url_cannot_be_completed_in_another_browser` proves browser
  B (no binding cookie) gets 400, no `vks_browser_session=` is minted, the row stays 'pending',
  and rightful browser A still completes 200. Single consumer: task 004's
  `concurrent_claim_has_exactly_one_consumer` proves exactly one claimant wins the conditional
  UPDATE.
- SC4 expiry: `an_expired_handoff_cannot_be_completed` (row DB-aged past TTL → 400).
- SC4 replay: `replaying_a_completed_callback_is_rejected` (a claimed row is terminal → second
  callback 400).

### Task 010 round-1 remediation — panel-strengthened assertions (test-only)

Test-only remediation per the plan's "Panel-strengthened assertions (round-1 remediation)"
section, on top of `0f5064d1` and plan amendment `c34c5eb2`. `crates/server/src/routes/oauth.rs`
is UNTOUCHED (git diff on the file is empty at commit).

Strengthened/new assertions in `crates/server/tests/browser_oauth.rs`:
- `a_copied_callback_url_cannot_be_completed_in_another_browser` records
  `redeems = hive_request_count("POST", "/v1/oauth/web/redeem")` after start_login; the count
  must be unchanged after browser B's cookieless 400 AND after a second stolen attempt whose URL
  appends browser A's RAW binding token as `&vks_browser_binding=<raw>` (extracted from
  `a.header_value()`; the token is URL-safe base64 without padding, so raw embedding is exact);
  the row must still be 'pending' after the smuggle; the rightful completion then takes the
  count to exactly `redeems + 1`.
- `a_forged_binding_cookie_does_not_consume_the_handoff` and
  `an_expired_handoff_cannot_be_completed` assert the redeem count is unchanged by the 400.
- `replaying_a_completed_callback_is_rejected` asserts the count is 1 after the successful
  completion and still 1 after the rejected replay.
- NEW `completion_drops_the_epoch_fence_before_hive_redemption`: `mock_hive_oauth` +
  `start_login` in jar A, priority-1 `mock_hive_delayed("POST", "/v1/oauth/web/redeem")`, the
  callback GET spawned via raw reqwest carrying jar A's Cookie header, the arrival oneshot
  awaited under a 2s timeout (redemption provably in flight), then
  `h.deployment().browser_auth_epoch().try_lock()` must succeed — the claim guard was dropped
  before Hive I/O; the guard is dropped and the spawned request task aborted and awaited.

Mutation evidence (each mutant applied to `crates/server/src/routes/oauth.rs` temporarily,
focused run, then `git checkout --` restore; final `git diff` on the file verified empty):
- (a) `claim_handoff` moved to AFTER `handoff_redeem` (a non-consuming SELECT feeds the redeem
  its `app_verifier` — the only way the relocation compiles): 3 tests FAILED —
  `a_forged_binding_cookie_does_not_consume_the_handoff` at browser_oauth.rs:282 "a forged
  binding cookie must not burn the one-time Hive code: left 1, right 0";
  `an_expired_handoff_cannot_be_completed` at browser_oauth.rs:341 "an expired handoff must not
  reach Hive redemption: left 1, right 0"; `replaying_a_completed_callback_is_rejected` at
  browser_oauth.rs:317 "a replayed callback must not redeem again: left 2, right 1". The
  cookieless wrong-browser attempts in the copied-callback test still exit at the pre-claim
  no-cookie branch and never reach Hive, so the forged/expired/replay redeem-count assertions
  are the claim-order discriminators, exactly as designed.
- (b) `drop(epoch_guard)` deleted (guard held across redemption to end of handler scope):
  `completion_drops_the_epoch_fence_before_hive_redemption` FAILED at browser_oauth.rs:389
  "epoch fence is still held while Hive redemption is in flight: TryLockError(())".
- (c) query-parameter fallback accepted for `vks_browser_binding` (read from the query params
  in addition to headers): `a_copied_callback_url_cannot_be_completed_in_another_browser`
  FAILED at browser_oauth.rs:238 "a query-parameter binding token must not complete the
  handoff: left 200, right 400" — the smuggled raw token completed the login ("Signed in with
  github"), the exact copied-URL escalation the headers-only rule exists to prevent.

Panel finding-1 adjudication (no code change): the no-binding-cookie branch is a separate
pre-claim exit performing ZERO DB access — it reveals nothing about any handoff row, and the
browser itself knows whether it holds a cookie, so its distinct "start again" guidance message
is not an oracle. All four claim-failure paths (unknown id / wrong cookie / expired / replay)
share the single message "OAuth handoff not found, expired, or already completed" prescribed by
the After block. The plan's self-contradiction (a STOP trigger that read as forbidding the
distinct no-cookie message) was resolved by amending the STOP-trigger wording in commit
`c34c5eb2` — source unchanged.

Verification: `cargo test -p server --test browser_oauth` 8 passed; `--test
browser_auth_routes` 4 passed; `--test harness_smoke` 11 passed; `cargo clippy -p server
--all-targets --all-features -- -D warnings` clean; `cargo fmt --all` then `-- --check` clean;
`git diff --check` clean; `git diff crates/server/src/routes/oauth.rs` empty.

## Task 010 closure — gates, panel verdicts, remediation provenance

- Preflight: freshness PASS (spec 680587143dbb2f1cbe74d1f7ef78a250705d5686); plan lint PASS;
  stale "6 tests" count corrected to 7 in `fe896a7b` (lint re-run PASS).
- Implementation `0f5064d1` (oauth.rs + browser_oauth.rs + ledger; RED 5 passed/2 failed with
  the two 200-expecting tests failing and the two 400-expecting tests vacuously green — RED
  mechanism recorded; GREEN 7 passed).
- Stage-1 gate `0f5064d1` vs `fe896a7b` (fmt override; browser_oauth scope): CONFORMS,
  GATE_FAIL_CHECK=none.
- Stage-2 round 1: kimi CONFORMS (epoch-guard await-order trace, single-decider UPDATE,
  post-claim region byte-identical, 2 INFO: epoch-capture-consumed-in-011 by design;
  take_oauth_handoff inert outside scope). gpt DEVIATES: 1 BLOCKING (distinct no-cookie
  message) adjudicated FALSE POSITIVE as source defect — pre-claim exit performs zero DB
  access, all four claim failures share the one message; plan self-contradiction fixed by
  STOP-trigger rewording in `c34c5eb2` (lint PASS). 3 REAL test-strength findings:
  claim-after-redeem, epoch-held-across-redeem, query-param binding fallback — all
  mutation-false-green.
- Remediation plan `c34c5eb2`; remediation source `0fca152c` (test-only + ledger adjudication
  note; oauth.rs byte-identical, hash 9c3b4e6f…61376). Mutation evidence recorded above:
  (a) claim-after-redeem → forged/expired/replay redeem-count assertions FAIL (:282, :341,
  :317); copied-callback test itself stays green under (a) because cookieless attempts exit at
  the pre-claim no-cookie branch — the count assertions are the claim-order discriminators;
  (b) epoch guard held across redeem → try_lock test FAILS (:389 TryLockError(())); (c)
  query-param fallback → stolen-raw-token attempt FAILS (:238 left 200 right 400).
- Stage-1 gate `0fca152c` vs `c34c5eb2`: CONFORMS, GATE_FAIL_CHECK=none (transcript:
  file-set 2 paths; typecheck override exit 0; browser_oauth scope green).
- Stage-2 focused re-review (dissenting seat, gpt): all three test-strength findings CLOSED,
  scope exact, oauth.rs byte-identical, no new issues; VERDICT: APPROVE.
- Final verification at `0fca152c`: browser_oauth 8 passed; browser_auth_routes 4;
  harness_smoke 11; clippy -p server --all-targets --all-features -D warnings clean; fmt
  clean; git diff --check clean.
- Status flipped ready → passed.

## Task 011 decisions

### RED evidence (tests written first; suite `cargo test -p server --test browser_oauth`, 8 passed / 4 failed)

- `successful_login_mints_a_hash_only_persistent_session_cookie` FAILED at browser_oauth.rs:363
  `no session cookie` — the pre-task callback completed 200 with NO session cookie (no mint path
  existed).
- `the_same_owner_may_authorize_a_second_browser` FAILED at browser_oauth.rs:422
  `first session survived: left 401, right 200` — first browser had no session.
- `a_different_subject_is_rejected_without_replacing_credentials_or_sessions` FAILED at
  browser_oauth.rs:458 `left: 200, right: 400`, body "Signed in with github" — the EXACT ordering
  defect this task fixes: pre-task code saved the intruder's credentials before any subject
  check and happily completed the login.
- `an_invalid_candidate_token_yields_a_sanitized_generic_400_with_no_writes`, two RED stages:
  - Stage 1 (redeem override mounted AFTER `mock_hive_oauth`, per the dispatch note's "wiremock
    is LIFO" claim): FAILED at :503 `left: 200, right: 400` — the override never fired; wiremock
    0.6.5 resolves EQUAL-priority matches by FIRST registration, not last (the harness's own
    overrides — `mock_hive_failure`, `delay_hive_profile`, `mock_hive_delayed` — all use
    `.with_priority(1)` for exactly this reason). Empirically proven on this repo.
  - Stage 2 (override mounted BEFORE `mock_hive_oauth`, no body matcher so it shadows the
    code-1-specific redeem for the whole test): FAILED at :513 `must be the generic
    browser-login failure message: {"success":false,...,"message":"Invalid access token: failed
    to decode JWT: InvalidToken"}` — the pre-task path leaked the decode-error detail into the
    HTTP body instead of the sanitized generic message.

### GREEN evidence

`cargo test -p server --test browser_oauth` → 12 passed, 0 failed (8 carried over from
009/010 + the three plan tests at :350/:403/:432 + the invalid-candidate-token test at :485).

### Ordering satisfied (login.rs `complete_browser_login`, crates/server/src/auth/login.rs)

1. redeem → candidate `Credentials` built in memory, never saved;
2. `extract_expiration(&redeem.access_token)?` → `BrowserLoginError::InvalidToken` via
   `#[from] utils::jwt::TokenClaimsError]` (static Display "candidate access token is invalid";
   never mapped to OwnerMismatch/Remote);
3. `profile_with_token(&redeem.access_token)` (new `RemoteClient` method; AuthMode::ApiKey
   passes the CANDIDATE token straight to `bearer_auth`, never the saved daemon creds, never
   `get_login_status()`/cached profile);
4. `pin_or_verify_owner(pool, profile.user_id, now)` with `BrowserAuthError::OwnerMismatch` →
   `BrowserLoginError::OwnerMismatch` and `BrowserAuthError::Database` → `Database` remapping
   (db error type differs, as instructed); runs BEFORE any credential/session write;
5. fenced commit: `browser_auth_epoch` guard acquired ONLY here (never across redemption or
   profile I/O), `*guard != epoch_at_claim` → sanitized static-Display `Disconnected` with
   nothing saved/minted; then `auth_context.refresh_guard()` acquired, `save_credentials`,
   `create_session` (hash-only), `install_remote_sync(config).await` when `share_config()` is
   Some, then both guards dropped. Raw token returned to the caller for exactly one
   destination: the Set-Cookie header (never a body, redirect or query param).

Route (`routes/oauth.rs`): `_epoch_at_claim` renamed to `epoch_at_claim` and passed through;
the redeem/save/get_login_status/detached-spawn_remote_sync region replaced by the
`complete_browser_login` match (OwnerMismatch → warn + specific 400; everything else →
`error = %e` Display-only log + generic 400 "Sign-in could not be completed. Please start
again." — no `?e`/Debug anywhere); node-cache-sync block kept; success response is
`close_window_response` + one inserted `Set-Cookie: vks_browser_session=…` header.

### Deviations from plan/dispatch text (all mechanical, evidenced)

- Redeem-override mount order in the invalid-candidate-token test: dispatch said mount AFTER
  `mock_hive_oauth` ("wiremock is LIFO"); empirically false for wiremock 0.6.5 (stage-1 RED
  above), so the override is mounted BEFORE with no body matcher — same intent (override
  redeem with a malformed token), correct resolution semantics. No harness change needed.
- `BrowserLoginError::NotConfigured` variant added: `deployment.remote_client()` returns
  `Result<_, RemoteClientNotConfigured>`, which cannot flow through `#[from]
  RemoteClientError`. Unreachable in practice (a claimable handoff implies `handoff_init`
  already required a configured client); static sanitized Display, folds into the generic 400.
- `Clock` trait imported in login.rs (`SystemClock.now_millis()` is a trait method) —
  mechanical import fix.
- `pub mod login;` inserted alphabetically in auth/mod.rs.

### Manual verification

- `cargo test -p server --test browser_auth_routes` → 4 passed; `--test harness_smoke` →
  11 passed; `cargo test -p services` → 318+12+5+6+5 passed, 0 failed (all green);
  `cargo clippy -p server -p services --all-targets --all-features -- -D warnings` clean;
  `cargo fmt --all` then `-- --check` clean; `git diff --check` clean. task-gate.sh NOT run
  by the implementer per explicit dispatch instruction; the gate's constituent commands
  (fmt --check, browser_oauth suite) were run directly and are green.
- TS2 walk-through: public/protected routing → 008's `browser_auth_routes` suite
  (public_surface…, protected_api_is_denied_by_default, unknown_api_paths…,
  oauth_initiation_and_callback_stay_public); browser-A isolation, callback copying/replay →
  010's suite (a_copied_callback…, a_forged_binding_cookie…, replaying_a_completed_callback…,
  an_expired_handoff…); cookie attributes, hash-only storage, same-owner and different-owner
  redemption → this suite (:350, :403, :432) plus invalid-candidate-token (:485); browser
  logout and explicit disconnect → 012's suite (not yet written).
- SC5 restart clause (survival across a planned idle node restart) is proven by task 015's
  TS4 suite against the same migrated SQLite/assets directory — NOT re-proven here.

### Round-1 panel remediation (post-implementation; kimi CONFORMS; gpt found the items below)

Provenance: round-1 adversarial panel — kimi seat returned CONFORMS (no findings); gpt seat
found four items, all assigned to this owning task: (1) upstream-body log leak via the
transparent `Remote` Display (task 018's sentinel obligation assigned here per
no-deferred-remediation), (2) a logout slot-guard-across-await deadlock cycle newly reachable
because this task's fenced commit holds `browser_auth_epoch` + `refresh_guard` across
`install_remote_sync` (which locks the share-sync slot) while `logout` held that same slot
across `handle.shutdown().await` and the RemoteSync task can be blocked on `refresh_guard`, and
(3)+(4) the fence and save-failure orderings were mutation-false-green — no test existed that
could turn red under either mutation.

Changes applied (plan section "Panel-strengthened corrections (round-1 remediation)"):

- `BrowserLoginError::Remote` Display made static: `#[error("remote service error")]`
  (`#[from]` retained; only Display changes). Pinned by same-file unit test
  `remote_variant_display_is_sanitized` wrapping `RemoteClientError::Http { status: 500,
  body: "SENTINEL-ACCESS-8f31c0d2" }`.
- `logout` take-before-await: the share-sync handle is `.take()`n inside a scoped block so the
  slot guard drops BEFORE `handle.shutdown().await` — the pattern used everywhere else. Cycle
  removed; full disconnect semantics remain task 012's.
- New additive harness helper `mock_hive_delayed_json(method, path, delay_ms, body)` — same
  signal-on-arrival shape as `mock_hive_delayed`, `.with_priority(1)`, `record_hive_mock`, but
  answers `set_body_json(body)` after `delay_ms`. One mechanical fix: the `Respond` closure
  must be `Fn` (wiremock calls it per request) but `set_body_json` consumes its value, so the
  closure clones `body` per invocation.
- Test A `a_stale_callback_cannot_commit_after_the_epoch_moves`: deterministic "stale"-label
  JWT obtained before mounting; priority-1 delayed-json redeem (300ms) mounted BEFORE
  `mock_hive_oauth("code-1", "stale", "ref", owner)`; completion GET spawned via raw reqwest
  (addr + Cookie header cloned before the spawn); arrival awaited under 2s; epoch bumped from
  the test; asserts 400 + generic body + NOT owner-mismatch wording + 0 browser_sessions rows
  + credentials bytes unchanged.
- Test B `a_credential_save_failure_mints_no_session`: after `start_login`,
  `credentials.json.tmp` sabotaged as a directory (EISDIR inside `FileBackend::save`'s temp
  open); asserts 400 generic body, no `vks_browser_session` Set-Cookie, 0 session rows.

Mutation evidence (each mutant applied to the source, focused test run, source restored
byte-identical — sha256-verified against a snapshot of the remediated file; note
`git checkout -- login.rs` alone could not serve as the restore because the remediation is not
yet committed, so restores used the snapshot copy and `cmp`):

- (a) Deleted the `if *epoch_guard != epoch_at_claim { return Err(Disconnected); }` block →
  Test A FAILED at browser_oauth.rs:649 `assertion left == right failed ... left: 200
  right: 400` (body: "Signed in with github. You can return to the app." — the mutant committed
  the stale login and, per the plan's prediction, the session-count assertion path follows).
- (b) Moved `create_session` before `save_credentials` → Test B FAILED at
  browser_oauth.rs:706 `assertion left == right failed: no session row may exist when the save
  failed — left: 1 right: 0`.
- (c) Reverted `Remote` Display to `#[error(transparent)]` → the sentinel unit test FAILED at
  login.rs:151 `assertion left == right failed — left: "http 500: SENTINEL-ACCESS-8f31c0d2"
  right: "remote service error"` (the exact log-leak the panel flagged).

Verification: `cargo test -p server --test browser_oauth` → 14 passed (12 prior + A + B);
`--lib auth::login` → 1 passed (103 filtered); `--test browser_auth_routes` → 4 passed;
`--test harness_smoke` → 11 passed; `cargo clippy -p server --all-targets --all-features --
-D warnings` clean; `cargo fmt --all` + `-- --check` clean (stable-channel warnings about
nightly-only rustfmt.toml options are pre-existing and cosmetic); `git diff --check` clean;
`git diff` on login.rs + oauth.rs shows ONLY the intended Display and take-before-await
changes. task-gate.sh NOT run per dispatch instruction.

## Task 011 closure — gates, panel verdicts, remediation provenance

- Preflight: freshness PASS (spec 680587143dbb2f1cbe74d1f7ef78a250705d5686); plan lint PASS;
  stale "9 tests" count corrected in `dbab7d08`.
- Implementation `5391555f` (remote_client.rs + auth/login.rs + auth/mod.rs + oauth.rs +
  browser_oauth.rs + ledger; RED 8 passed/4 failed incl. the exact ordering defect — intruder
  creds saved + 200; GREEN 12). Ledgered deviations: wiremock 0.6.5 equal-priority resolution is
  first-registered-wins (dispatch's LIFO guidance corrected empirically — malformed-redeem
  override mounts BEFORE mock_hive_oauth); BrowserLoginError::NotConfigured added (RemoteClientNotConfigured cannot flow #[from] RemoteClientError); mechanical Clock import.
- Stage-1 gate `5391555f` vs `dbab7d08`: CONFORMS, GATE_FAIL_CHECK=none (6 paths; fmt override;
  browser_oauth scope).
- Stage-2 round 1: kimi CONFORMS (full ordering trace; edition-2024 cycle claim WRONG — see
  below; 4 INFO: Disconnected unreachable until 012 wires the epoch bump, Http-body Display
  note, get_login_status warmup removal equivalent, pre-existing logout ?e io::Error). gpt
  DEVIATES — all four findings verified against source and adjudicated REAL:
  1. [BLOCKING] upstream-body leak: RemoteClientError::Http Display `http {status}: {body}` +
     transparent Remote + route `%e` → sentinel-bearing 5xx body reaches logs; task 018
     explicitly assigns the fix to the owning task.
  2. [BLOCKING] reachable deadlock: fenced commit holds epoch+refresh across
     install_remote_sync's share-sync slot lock; logout held that slot across
     shutdown().await (join); RemoteSync task can block on refresh_guard mid-authed-call.
     (kimi's edition-2024 disproof was wrong: the if-let scrutinee-temporary tightening
     affects the ELSE block, not the THEN block — guard was held across the await.)
  3. [BLOCKING] epoch-fence mutation-false-green (no claim→commit mismatch test).
  4. [SHOULD-FIX] save-failure-before-mint untested.
- Remediation plan `32155a84` ("fence commit fence and sanitize remote display" amended with
  harness conflict pair): files += common/mod.rs; conflicts_with 006↔011 linked bidirectionally
  (frontmatters + plan.md); static Remote Display keeping #[from]; logout take-before-await;
  additive mock_hive_delayed_json(method, path, delay_ms, body) helper; Test A
  a_stale_callback_cannot_commit_after_the_epoch_moves (delayed-json redeem, arrival signal,
  external epoch bump, 400 generic + zero sessions + creds unchanged); Test B
  a_credential_save_failure_mints_no_session (dir at credentials.tmp forces EISDIR). Lint PASS.
- Remediation source `13107ce0`: all four changes + sentinel Display unit test. Mutation
  evidence (all restored byte-identical, sha256-verified): (a) epoch check deleted → Test A RED
  `left: 200 right: 400` (stale login committed); (b) create_session moved before save → Test B
  RED `left: 1 right: 0`; (c) Display reverted to transparent → sentinel test RED `left: "http
  500: SENTINEL-ACCESS-8f31c0d2"`.
- Stage-1 gate `13107ce0` vs `32155a84`: CONFORMS, GATE_FAIL_CHECK=none (5 paths).
- Stage-2 focused re-review (dissenting seat, gpt): all four findings CLOSED; logout shutdown
  still idempotent; Test A deterministic (claim guard dropped before redeem; priority-1
  shadowing correct; no current-thread starvation); VERDICT: APPROVE.
- Final verification at `13107ce0`: browser_oauth 14; auth::login lib 1; browser_auth_routes 4;
  harness_smoke 11; clippy -p server --all-targets --all-features -D warnings clean; fmt clean;
  git diff --check clean. GPT INFO-5 (Location/log full-header scanning) remains owned by task
  018 per its plan.
- Status flipped ready → passed.

## Task 012 decisions

### Mechanical deviations (compile/format only)

- The dispatch grounding path `services::services::share::config::ShareConfig` does not compile:
  `mod config` is private (crates/services/src/services/share.rs:14) and the struct is
  re-exported as `services::services::share::ShareConfig` (the compiler's own suggestion).
  Applied the re-export path in both tests; no behavior change.
- Test-file imports added at top (`deployment::Deployment`, `serde_json::json`, `std::time::Duration`,
  `uuid::Uuid`) so the plan-verbatim snippets (`Uuid::new_v4()`, `json!({})`) compile as written.
- `cargo fmt --all` reflowed the plan-verbatim snippets' long lines (login helper's
  `mock_hive_oauth(...)` call, test 1's `raw_a`/set-cookie lines, the `browser_logout` handler's
  `revoke_session(...)` call, and `tracing::info!(invalidated, revoked, ...)`) — whitespace only.
- The existing `let auth_context = deployment.auth_context();` binding was kept (existing code
  preserved verbatim); the plan's `deployment.auth_context().refresh_guard().await` line is used
  as written immediately after `client.logout()`.

### Dispatch fixture correction (plan amendment `636cba61`)

Race 4's delayed refresh response was corrected from the valid labeled JWT to an INVALID
`"not-a-jwt"` access token: the original valid-token body was mutation-false-green for (c)
because the refresher saves a valid far-future token inside the guard before releasing it, so
the mutant's queued acquisition lands on valid credentials and never re-enters the guard — the
deadlock the dispatch predicted cannot occur with that body. The invalid-token body makes the
refresher error without saving, forcing the mutant's `client.logout()` to re-enter the
non-reentrant `refresh_guard` on the same task.

### RED evidence (tests written first; `cargo test -p server --test browser_auth_routes`, 6 passed / 6 failed)

- `browser_logout_revokes_the_presented_raw_token_only_and_keeps_real_sync` FAILED at :195
  `matches!(out.status, 200 | 204)` — POST /api/auth/browser/logout answered 404 (route absent).
- `anonymous_browser_logout_is_rejected_before_the_handler` FAILED at :313 `left: 404 right: 401`
  (body `{"success":false,"message":"unknown api route"}` — route absent).
- `hive_disconnect_revokes_all_sessions_stops_real_sync_and_keeps_owner` FAILED at :243
  "explicit Hive disconnect must stop every Hive synchronization task" — pre-task logout left
  node-cache running.
- `disconnect_during_an_in_flight_callback_leaves_no_session_credentials_or_sync` FAILED at :380
  `left: 200 right: 400` (body "Signed in with github") — pre-task logout never bumped the epoch,
  so the mid-flight callback committed.
- `a_pending_callback_from_before_disconnect_is_durably_invalidated` FAILED at :447
  `left: 200 right: 400` (body "Signed in with github") — no durable pending-handoff
  invalidation existed.
- `a_fresh_login_after_disconnect_still_succeeds` FAILED at :478 `left: 2 right: 1` — pre-task
  logout revoked no sessions, so B's stale session plus C's new one were both live.
- The six passing were the four task-008 tests, the different-subject test (task-011's owner
  pin already enforces it — regression guard), and race 4 (the pre-task logout takes no
  refresh_guard at all, so it cannot deadlock — regression guard).

### GREEN evidence

`cargo test -p server --test browser_auth_routes` → 12 passed / 0 failed.

### Mutation evidence

Each mutant was applied to the uncommitted source, focused-run, then restored from a
pre-mutation snapshot (task-011 precedent: `git checkout --` cannot serve as restore for
uncommitted work); byte-identical restore verified by sha256 against the snapshot.

- (a) Deleted the `if *epoch_guard != epoch_at_claim { return Err(Disconnected); }` re-check in
  `crates/server/src/auth/login.rs` → race 1 FAILED at browser_auth_routes.rs:380
  `left: 200 right: 400` (body "Signed in with github") — the stale callback committed exactly
  as the plan predicted. Restored (sha256 dac4378a…9bca8 match).
- (b) Removed the `invalidate_pending_handoffs(...)` call from `logout` → race 2 FAILED at
  browser_auth_routes.rs:447 `left: 200 right: 400` (body "Signed in with github") — the
  pre-disconnect pending handoff stayed claimable. Restored (sha256 885530…09cc2 match).
- (c) MOVED `refresh_guard` ACQUISITION BEFORE `client.logout()` → with the ORIGINAL
  dispatch-dictated valid-token body, race 4 PASSED (mutation-false-green) in both mutant
  placements (before the call, and first statement of the handler; 1.6–3.0s). Mechanism: the
  refresher acquires the guard BEFORE its HTTP request (remote_client.rs:262), and its delayed
  response carried a VALID far-future JWT whose credentials `refresh_credentials` saves INSIDE
  the guard scope (remote_client.rs:261-327) before releasing it — the mutant's queued
  acquisition landed on valid credentials, so `client.logout()` → `require_token()` returned
  the saved token without re-entering the refresh path (remote_client.rs:248-252): no deadlock,
  identical observable state. STOP was invoked (no commit) and escalated. After plan amendment
  `636cba61` corrected the body to an INVALID `"not-a-jwt"` access token, the re-run of
  mutation (c) FAILED exactly as required: race 4 panicked at browser_auth_routes.rs:538
  `disconnect during in-flight token refresh timed out (lock regression?): Elapsed(())`,
  finished in 30.01s (the 30s wrapper fired — the refresher errors with
  RemoteClientError::Token without saving, so the mutant's `client.logout()` re-enters the
  non-reentrant guard on the same task and deadlocks). Correct implementation under the
  corrected body passed in 3.13s. oauth.rs restored byte-identical (sha256 885530…09cc2 match
  against the pre-mutation snapshot).

### Verification (all with TMPDIR=$PWD/.cargo-tmp DISABLE_WORKTREE_ORPHAN_CLEANUP=1)

- `cargo test -p server --test browser_auth_routes` → 12 passed.
- `cargo test -p server --test browser_oauth` → 14 passed (regression).
- `cargo test -p server --test harness_smoke` → 11 passed (regression).
- `cargo clippy -p server --all-targets --all-features -- -D warnings` → clean.
- `cargo fmt --all` then `cargo fmt --all -- --check` → clean (pre-existing nightly-only
  rustfmt.toml warnings unchanged).
- `git diff --check` → clean.
- task-gate.sh NOT run per dispatch instruction.

### SC7/SC8 walk-through (plan manual-verification item 3)

- SC7 scope of revocation: only the presenting browser — raw token captured before logout,
  replayed from a fresh jar → 401, `revoked_at` set by hash, browser B still 200
  (`browser_logout_revokes_the_presented_raw_token_only_and_keeps_real_sync`).
- SC7 credentials/sync untouched: `share_sync_handle().lock().await.is_some()` and
  `node_cache_sync_is_running()` both asserted true after browser logout (same test).
- SC8 revoke every session: `live_session_count == 0` plus A and B both 401 afterwards
  (`hive_disconnect_revokes_all_sessions_stops_real_sync_and_keeps_owner`).
- SC8 credentials removed: `!h.credentials_path().exists()` (same test, and race 4 asserts it
  against an in-flight refresh).
- SC8 sync stopped: sync slot `None` and `!node_cache_sync_is_running()` (same test).
- SC8/D4 owner retained: `stored_owner_uuid == owner` after disconnect (same test, race 1, and
  the different-subject test pins that a different subject cannot replace it).
