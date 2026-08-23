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
  `crates/db/src/models/execution_process/queries.rs:1437` with 0/200
  `SQLITE_BUSY_SNAPSHOT` observations. It is tracked at
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
