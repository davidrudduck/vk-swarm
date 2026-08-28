# Independent remediation re-review — kimi fidelity seat

**Target:** detached worktree at `fa0612ee`, remediation range `ae5ee15f..fa0612ee`
**Prompt:** `.agents/reports/2026-08-22-round-2-phase-one/remediation-recheck-prompt.md`
**Posture:** read-only. No source or git mutations (verified `git status --short` empty at end of review; only `target/` build artifacts were touched, including one rebuild to clear stale-artifact corruption caused by a /tmp quota failure in this review session).

## Range summary

Four commits: `183da92f` (task-022 plan update), `594d531c` (explicit file-backed credentials constructor + fixture swap), `7b5d6eff` (deterministic busy-snapshot calibration controls), `fa0612ee` (workstream README + decisions-ledger entry). Diff: 7 files, +202/−126.

## Finding 1 — task 022 Keychain fixture (CLOSED)

- `OAuthCredentials::new_file_backed()` is explicit and path-scoped: `crates/services/src/services/oauth_credentials.rs:55-60` constructs `Backend::File(FileBackend { path })` directly, bypassing `Backend::detect`.
- Production detection is unchanged: `new()` (`oauth_credentials.rs:48-53`) and `Backend::detect` (`:101-122`, including the `OAUTH_CREDENTIALS_BACKEND` env handling and the macOS `debug_assertions` default) are byte-identical to `ae5ee15f` — confirmed by the range diff, which touches nothing above line 55 of that file.
- Every task-022 fixture that saves credentials uses the explicit constructor: `crates/local-deployment/src/lib.rs:1320` (saves `test-refresh-token` at `:1323-1330`) and `:1371`. The two remaining `OAuthCredentials::new` call sites are fine: `:601` is production `new()` (must keep detection); `:559` is `for_test()`, whose doc comment (`:531-533`) states the credentials are never loaded — and grep confirms nothing in the `for_test`/`from_parts` path calls `save`/`clear` (only `crates/services/src/services/auth.rs:32` calls `.save()`, unreachable from those fixtures without a login flow).
- The regression test is discriminating: `explicit_file_backend_is_path_scoped` (`oauth_credentials.rs:259-274`) saves through the explicit constructor and asserts the supplied path exists; had a Keychain backend been selected (the original hazard on macOS), the path would not exist and the test would fail. It does not mutate `OAUTH_CREDENTIALS_BACKEND`, matching the task-022 constraint.
- Task 022 plan/status/gate coherence: frontmatter `files:` now includes `crates/services/src/services/oauth_credentials.rs`; the gate transcript in the ledger claims "file-set: only declared files changed (2 paths)" for `594d531c`, and `git show 594d531c --stat` confirms exactly those 2 declared paths. `status: passed` is backed by the recorded gate run. The WAI_TEST_CMD in both "Manual verification" and "Done when" includes `cargo test -p services explicit_file_backend_is_path_scoped`.

## Finding 2 — sqlite-busy-snapshot-calibration-stability (CLOSED)

- Workstream exists: `dev-docs/workstreams/sqlite-busy-snapshot-calibration-stability/README.md` with frontmatter (`status: active`), finding description, resolution, and completion criteria; the ledger links it.
- Both negative controls now force the schedule deterministically instead of racing a background writer:
  - `crates/db/src/models/execution_process/queries.rs:1369-1414` (`control_read_then_write_shape_reproduces_busy_snapshot`): begin deferred tx → `SELECT` orphaned rows (opens WAL read snapshot, `:1383-1391`) → awaited intervening commit from a separate pooled connection (`:1394-1398`; `min_connections(2)` at `:1294` guarantees a second connection) → tx `UPDATE` must fail (`:1408`).
  - `crates/db/src/models/execution_process/lifecycle.rs:1110-1149` (`control_prior_status_read_reproduces_busy_snapshot`): same three-step schedule at `:1120-1143`.
  - Both require extended code 517 via `is_busy_snapshot` (`queries.rs:1273-1278`, `lifecycle.rs:995-1000`), `.expect_err` + explicit assert — no hollow assertions.
- Live production write-first tests retained and unchanged: `mark_orphaned_as_failed_does_not_read_then_upgrade` (`queries.rs:1311-1362`) and `update_completion_does_not_read_then_upgrade` (`lifecycle.rs:1033-1094`) are untouched by the range diff, still 200-iteration contention tests asserting 0 busy-snapshot errors, not ignored.
- No hidden ignores or gate changes: the range diff adds no `#[ignore]`, no `skip`, no config change; the doctest-ignore population observed in the test run (7 unit, 3 bulk, 2 doc) is pre-existing.

## Command evidence (this session, `TMPDIR=/home/david/.cache/dr-panel-tmp/rust-tmp`)

- `cargo test -p db` — full suite green, counts matching the ledger exactly: 302 passed/7 ignored unit tests; 8 passed/3 ignored bulk_operations; 1 emission_conformance; 6 sqlite_pragmas; 8 task_execution_timestamps; 8 task_variable_inheritance; 5 task_visibility_discriminator; doctests 11 passed/2 ignored. (An initial doctest E0460 "rlib format" failure was stale-artifact corruption from a /tmp-quota-killed build earlier in *this* review session — cleared by rebuilding `utils`; not a code defect.)
- `cargo test -p db control_` — both controls green in 6 consecutive runs (1 + a 5-run loop, all "2 passed; 0 failed").
- `cargo test -p services explicit_file_backend_is_path_scoped` — 1 passed.
- `DISABLE_WORKTREE_ORPHAN_CLEANUP=1 cargo test -p local-deployment configured_startup_sync_is_installed_before_constructor_returns` — 1 passed.
- `DISABLE_WORKTREE_ORPHAN_CLEANUP=1 cargo test -p local-deployment raw_api_base_remains_available_when_share_sync_config_is_unavailable` — 1 passed.
- `cargo clippy -p db -p services -p local-deployment --all-targets --all-features -- -D warnings` — clean.

## Fidelity checks

- **SC8 not silently completed:** ledger line 246 states "Route wiring in tasks 009–012 is still required before SC8 is complete"; task 022 manual-verification item 4 repeats it. Tasks 009–012 have no files in `docs/plans/local-node-browser-oauth/phase-1/` (only 001–005, 022), so no status flip occurred.
- **O8 remains explicit:** ledger lines 190–193 retain the accepted crash-window residual ("O8 residual accepted and explicit … operator retries disconnect"), unchanged by the remediation.
- **Ledger citations accurate:** the cited pre-remediation failure location `queries.rs:1437` matches the old probabilistic assert block at `ae5ee15f` (verified via `git show ae5ee15f:...queries.rs` lines 1430–1445). Gate transcript, commit hashes (`594d531c`, `7b5d6eff`), and test-count claims all verified above.
- **No scope/contract contradictions or unsafe test side effects:** the remediation diff is confined to the two control tests, the additive constructor, two fixture call sites, and documentation; `from_parts`, backend detection, and all production query shapes are untouched.

## Residual observations (non-blocking)

- The workstream README's "ten consecutive focused runs" claim was sampled, not fully reproduced here (6 consecutive green runs in this session, and the controls are now deterministic by construction — the intervening commit is awaited rather than timed, so scheduler luck is eliminated by design).
- The ledger's `queries.rs:1437` citation points into the pre-remediation file; accurate as a historical reference but not resolvable against the current tree (now 1415 lines). This is standard for failure citations and not misleading.

VERDICT: APPROVE
