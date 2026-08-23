# Remediation re-review: local-node-browser-oauth phase 1 (keychain isolation + busy-snapshot calibration)

- **Repo under review:** `/home/david/.cache/dr-panel-tmp/dr-panel-gentle-mongoose-fa0612ee-150627` at `fa0612ee` (detached)
- **Range reviewed:** `ae5ee15f..fa0612ee` (4 commits: `183da92f` plan, `594d531c` fix(auth), `7b5d6eff` test(db), `fa0612ee` docs close-out)
- **Reviewer seat:** kimi (OpenCode) — read-only; no source/git state modified
- **Date:** 2026-08-23

## Finding 1 — Task-022 fixtures could select the production macOS Keychain backend

**Remediation verified. No blocking issue.**

1. **Explicit constructor is genuinely detection-bypassing and path-scoped.**
   `OAuthCredentials::new_file_backed(path)` at `crates/services/src/services/oauth_credentials.rs:55-60`
   constructs `Backend::File(FileBackend { path })` directly — it never calls `Backend::detect`
   and never consults `OAUTH_CREDENTIALS_BACKEND`. `OAuthCredentials::new` (lines 48-53) and the
   entire `Backend::detect` body (lines 100-123, including the macOS `debug_assertions` default and
   the Keychain arm) are byte-identical to `ae5ee15f`; the commit `594d531c` diff is purely additive
   in this file. Production behavior unchanged, satisfying the task's STOP trigger
   (`docs/plans/local-node-browser-oauth/phase-1/022-...md:219`).

2. **Every task-022 fixture that saves credentials uses the explicit constructor.**
   The only test that saves is `configured_startup_sync_is_installed_before_constructor_returns`
   (`crates/local-deployment/src/lib.rs:1316-1359`): it builds
   `OAuthCredentials::new_file_backed(temp_dir.path().join("credentials.json"))` (line 1320) and
   then `credentials.save(...)` (lines 1323-1330) into a `create_test_pool_with_migrations` tempdir —
   no keychain reach. The second direct-constructor fixture
   (`raw_api_base_remains_available_when_share_sync_config_is_unavailable`, lines 1361-1387) also
   uses `new_file_backed` (line 1371) though it never saves. A workspace-wide grep for
   `OAuthCredentials::new` finds exactly one remaining detection-based caller in test code:
   `for_test()` at `crates/local-deployment/src/lib.rs:559`. That fixture is safe: `from_parts`
   touches the credentials object only via the in-memory `.get()` (lib.rs:240); no `load`/`save`/`clear`
   is ever invoked on it (grep over the crate confirms), so even on macOS release-profile test runs
   where `detect` would select `Backend::Keychain`, no Keychain I/O occurs. The doc comment at
   lib.rs:530-533 states this contract ("unwritten temp path (never loaded...)"). The production
   call site `LocalDeployment::new()` (lib.rs:601) correctly keeps `OAuthCredentials::new`.

3. **Regression test is discriminating.**
   `explicit_file_backend_is_path_scoped` (`oauth_credentials.rs:258-274`) saves a refresh token via
   the new constructor and asserts the *supplied path* exists. On macOS — the platform of the
   original finding — a regression to `Backend::detect` in a release test binary would select the
   fixed Keychain slot and the assertion would fail. On Linux it cannot distinguish detect-vs-explicit
   (both yield `File`), but it still pins the path-scoping contract. Passed:
   `cargo test -p services explicit_file_backend_is_path_scoped` → `1 passed` (run evidence below).

4. **Plan/status/gate coherence.**
   The task reopened before the fix (`183da92f`: `status: passed → ready`, added
   `crates/services/src/services/oauth_credentials.rs` to `files:`, added the constructor to the
   Change section, allowed-moves line 208, and STOP trigger line 219), the fix landed in `594d531c`
   touching exactly the 2 newly declared files, and `fa0612ee` re-marks `status: passed` with a gate
   transcript in `decisions-ledger.md:262-276` recording `commit=594d531c`, `file-set: only declared
   files changed (2 paths)`, typecheck override exit 0, scope tests green, `GATE_FAIL_CHECK=none`.
   The recorded transcript is abridged (ellipses) but internally consistent with the actual commit
   contents. Ledger entry `decisions-ledger.md:248-253` accurately describes the change
   ("`OAuthCredentials::new()` and backend detection are unchanged" — confirmed by diff).
   Focused tests all pass on this host:
   - `configured_startup_sync_is_installed_before_constructor_returns` → ok (0.70s)
   - `raw_api_base_remains_available_when_share_sync_config_is_unavailable` → ok (0.64s)
   - `browser_auth_epoch_is_shared_by_deployment_clones` → ok (1.10s)
   (run with `TMPDIR=/home/david/.cache/dr-panel-tmp/kimi-tmpdir DISABLE_WORKTREE_ORPHAN_CLEANUP=1`).

## Finding 2 — `sqlite-busy-snapshot-calibration-stability` scope split and probabilistic `cargo test -p db`

**Remediation verified. No blocking issue.**

1. **Workstream exists and is properly tracked.**
   `dev-docs/workstreams/sqlite-busy-snapshot-calibration-stability/README.md` (new in `7b5d6eff`)
   has valid frontmatter (`status: active`, named workstream), an honest finding statement (0/200
   `SQLITE_BUSY_SNAPSHOT` on this host), the resolution description, and completion criteria with the
   only unchecked box being the post-merge `shipped` flip. It is documented in the decisions-ledger
   (`decisions-ledger.md:254-260`), satisfying the three scope-split requirements (named, tracked
   README, ledger entry). Minor observation: the generated tracker `dev-docs/MASTER.md` has not been
   regenerated to list it (last touched by `1f2caaea`); MASTER.md is produced by `/wai:status`, so
   this is a tracker-regeneration chore, not a rule violation.

2. **Both negative controls now force a genuine read-snapshot / intervening-commit / write-upgrade
   schedule.**
   - `control_read_then_write_shape_reproduces_busy_snapshot`
     (`crates/db/src/models/execution_process/queries.rs:1368-1414`): begins a deferred transaction,
     issues the attempt-1 SELECT (opening a WAL read snapshot, asserted non-empty at line 1391),
     performs and awaits an intervening committed `UPDATE` on a *different* pooled connection
     (lines 1393-1398; pool is `min_connections(2)`, so the write cannot reuse the tx's connection),
     then `expect_err` on the write upgrade (line 1408) and asserts extended code 517 via
     `is_busy_snapshot` (lines 1410-1413; helper at queries.rs:1273-1278 compares `code() == "517"`).
   - `control_prior_status_read_reproduces_busy_snapshot`
     (`crates/db/src/models/execution_process/lifecycle.rs:1109-1149`): identical schedule for the
     17A-remediation shape.
   The schedule is deterministic by SQLite WAL semantics (any commit from another connection after a
   deferred reader's snapshot makes its write upgrade fail with `SQLITE_BUSY_SNAPSHOT`), and the
   controls cannot pass hollowly: if the snapshot were not invalidated the UPDATE would succeed and
   `expect_err` would panic; if the wrong error occurred the 517 assertion would fail. The flaky
   200-iteration background-writer harness was removed from both controls (no timing dependence
   remains).

3. **Extended code 517 required.** Confirmed at queries.rs:1273-1278 and lifecycle.rs:995-1000
   (`c == "517"`), asserted in both controls.

4. **Production write-first tests retained, live, and unchanged in this range.**
   `mark_orphaned_as_failed_does_not_read_then_upgrade` (queries.rs:1311-1362) and
   `update_completion_does_not_read_then_upgrade` (lifecycle.rs, asserts 0/200 at lines 1086-1093)
   keep their 200-iteration contention harness and zero-tolerance assertions; the `7b5d6eff` diff
   touches only the two control tests. Both pass:
   `cargo test -p db --lib read_then_upgrade` → `2 passed`.

5. **No hidden failures.** No `#[ignore]` added anywhere in the range (grep over both modified db
   files: none); no gate/config change; the range touches 5 code/doc files plus the new workstream
   README only.

6. **Stability evidence.** `cargo test -p db --lib busy_snapshot` (both controls) → 10/10
   consecutive `ok. 2 passed`. Full `cargo test -p db` → exit 0 with
   `302 passed / 7 ignored` unit, `8/3`, `1/0`, `6/0`, `8/0`, `8/0`, `5/0` integration suites, and
   `Doc-tests db: 11 passed; 2 ignored` — exactly matching the ledger claim at
   `decisions-ledger.md:272-276`. `cargo clippy -p db -p services -p local-deployment --all-targets
   --all-features -- -D warnings` → clean. `cargo fmt --all -- --check` → clean (only pre-existing
   nightly-option warnings from rustfmt).

## Regression sweep of `ae5ee15f..fa0612ee`

- Range is 7 files: 4 code (2 of which are test-only edits), 2 docs, 1 new workstream README.
  No production logic changes anywhere; `Backend::detect`, `OAuthCredentials::new`, route code, and
  migrations untouched.
- No contract/scope contradictions: task 022's `files:` list was amended *before* the source commit,
  so the gate's file-set check was legitimate, not retrofitted.
- No unsafe test side effects: the startup fixture invokes `disable_orphan_cleanup_for_tests()`
  (lib.rs:1318) before `from_parts`, per the task's allowed move; credentials land in per-test
  tempdirs.
- **O8 crash window:** still explicitly accepted, not silently closed — `decisions-ledger.md:190`
  ("O8 residual accepted and explicit: a process crash between SQLite revoke-all and file/Keychain…").
- **Pending route work 009–012:** not declared complete. 009–011 are `status: ready`
  (phase-2 fronts), 012 is `status: ready` (phase-3), and the ledger states plainly at
  `decisions-ledger.md:246`: "Route wiring in tasks 009–012 is still required before SC8 is
  complete." Task 022's manual-verification item 4 (task file line 226) records the same.

## Observations (non-blocking)

1. **Transient doctest harness flake, not attributable to this range.** During verification, two
   runs of `cargo test -p db --doc` executed in a tight loop immediately after recompilation failed
   with `0 passed; 11 failed` in <0.5s (harness-level abend, "test exited abnormally"), then 12+
   subsequent runs — including the full-suite `cargo test -p db` at exit 0 and forced-rebuild
   repetitions — all passed. The failure mode (all 11 doctests failing in under half a second) is
   consistent with a merged-doctest binary/artifact race in this host's cargo environment, not with
   the branch's content; the range modifies no doctest, no `crates/db` production file, and no test
   configuration. Worth noting only so a future red doctest run is investigated rather than blindly
   re-run.
2. `dev-docs/MASTER.md` has not been regenerated to list the new workstream (it is a generated
   artifact from `/wai:status`); the scope-split rule's substantive requirements (named split,
   README, ledger entry) are all met.
3. The service regression test cannot distinguish explicit-vs-detected file backend on Linux (both
   are `Backend::File`); its discriminating power is macOS-specific. Acceptable, since the hazard it
   guards is macOS-specific and the test does pin path-scoping on every platform.

## Conclusion

Both confirmed panel findings are remediated with discriminating, deterministic tests; production
behavior is unchanged; plan/status/gate/ledger evidence is coherent; the accepted O8 residual and
the pending 009–012 route work are still honestly tracked; and no regressions, hollow assertions,
or gate-weakening were found in `ae5ee15f..fa0612ee`.

VERDICT: APPROVE
