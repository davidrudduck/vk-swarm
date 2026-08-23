# Phase-1 Remediation Recheck — `local-node-browser-oauth`

- **Scope reviewed:** `ae5ee15f..fa0612ee` (4 commits) in the detached worktree at `fa0612ee`
- **Worktree state:** clean throughout (`git status --porcelain` empty before and after all test runs; `git rev-parse HEAD` = `fa0612ee4ad32bc7f5dbf49d232c29dd24f645a2`). Nothing was edited, restored, reset, stashed, or removed.
- **Commits in range:**
  1. `183da92f` `plan(wai): isolate task credentials from keychain` — task-022 plan file only
  2. `594d531c` `fix(auth): isolate test credentials from keychain` — `oauth_credentials.rs`, `local-deployment/src/lib.rs`
  3. `7b5d6eff` `test(db): stabilize busy snapshot calibration` — `lifecycle.rs`, `queries.rs`, new workstream README, ledger
  4. `fa0612ee` `docs(wai): close phase one review findings` — ledger + task-022 gate line

---

## Summary

- Two should-fix findings remain; both are test-integrity/claim-accuracy defects, not production-code defects.
  - **F-R2-01 (should-fix, lead):** the calibration repair removed the property the two production write-first tests depend on, and left a now-false docstring in shipped code. Neither the ledger nor the new workstream README discloses this.
  - **F-R2-02 (should-fix):** the new `explicit_file_backend_is_path_scoped` regression test cannot fail under the repository's documented validation command if the fix is reverted.
- Everything else in the brief verified clean, including both explicit "has this been silently declared complete?" checks.

---

## Finding F-R2-01 — calibration repair silently de-fangs the two production write-first tests (should-fix)

- **What the controls existed for**
  - `crates/db/src/models/execution_process/queries.rs:1364-1367` (docstring, unchanged by this range):
    - "reconstructs attempt 1's REJECTED shape … **against the IDENTICAL harness**, to prove it is capable of reproducing F17B-1's finding rather than being silently toothless."
  - The "harness" was `build_contention_pool()` **plus** the background writer committing every ~200 µs **plus** the 200-iteration loop — the exact harness used by the production tests it calibrates.
- **What `7b5d6eff` changed**
  - Both controls dropped the background writer and the iteration loop, keeping only `build_contention_pool()`:
    - `crates/db/src/models/execution_process/queries.rs:1381-1414` — single deferred tx, one forced external commit, one `expect_err`
    - `crates/db/src/models/execution_process/lifecycle.rs:1116-1146` — same shape
  - The production tests were **not** changed and still use the probabilistic harness:
    - `crates/db/src/models/execution_process/queries.rs:1312` `mark_orphaned_as_failed_does_not_read_then_upgrade` — background writer at `queries.rs:1323-1334`, `assert_eq!(busy_snapshot_errors, 0, …)` at `queries.rs:1354`
    - `crates/db/src/models/execution_process/lifecycle.rs:1034` `update_completion_does_not_read_then_upgrade` — background writer at `lifecycle.rs:1049-1060`, `assert_eq!(busy_snapshot_errors, 0, …)` at `lifecycle.rs:1084`
- **Why this matters — the failure scenario**
  - The control's `0/200` result recorded in `docs/plans/local-node-browser-oauth/decisions-ledger.md:255-256` is a **direct measurement that the background-writer harness does not interleave on this host**.
  - After the repair, the controls exercise a hand-built schedule that shares only the pool config with the production tests. Nothing in the tree still demonstrates that the background-writer generator can produce a conflicting commit at the right moment.
  - Consequently `assert_eq!(busy_snapshot_errors, 0)` in both production tests now passes **vacuously** on this host, and no remaining test can detect that. If someone reintroduced a read-then-upgrade shape into `update_completion` or `mark_orphaned_as_failed`, those two tests would still report `0/200` and pass. Panel 18's measured `15/200` injection result (cited in the `lifecycle.rs:1027-1039` docstring) was done in a detached worktree and is not a shipped test — the in-tree control was the only live substitute, and it no longer plays that role.
  - The remediation therefore converted a *failing calibration signal* into *silence*, which is the precise hazard the original docstring was written to prevent.
- **False claim left in shipped code**
  - `crates/db/src/models/execution_process/queries.rs:1364-1367` still asserts the control runs "against the IDENTICAL harness"; it does not. This is a misleading in-code claim about the code directly beneath it.
- **Undisclosed in the remediation artifacts**
  - `dev-docs/workstreams/sqlite-busy-snapshot-calibration-stability/README.md:32-34` states "The production write-first stress tests remain unchanged" — true, but presented as reassurance when it is the problem: they remain unchanged *and* uncalibrated.
  - `docs/plans/local-node-browser-oauth/decisions-ledger.md:253-259` describes the fix as "forcing the read-snapshot/intervening-commit/write-upgrade schedule in both calibration controls" with no mention that the production tests' `== 0` assertions lost their evidential backing.
- **Suggested remediation**
  - Either (a) keep a probabilistic control on the *same* harness alongside the deterministic one, so the generator itself stays calibrated, or (b) drive the production functions under the same forced schedule, or (c) at minimum correct the `queries.rs:1364-1367` docstring and record in the ledger/README that the two `assert_eq!(…, 0)` assertions are now structurally-justified rather than harness-calibrated.

---

## Finding F-R2-02 — new regression test cannot fail under the documented validation command (should-fix)

- **The fix itself is correct.** `OAuthCredentials::new_file_backed()` at `crates/services/src/services/oauth_credentials.rs:55-60` constructs `Backend::File(FileBackend { path })` directly, bypassing detection. `OAuthCredentials::new()` (`oauth_credentials.rs:47-52`) and `Backend::detect` (`oauth_credentials.rs:100-123`) are byte-for-byte unchanged (`git diff ae5ee15f..fa0612ee` shows additions only). Both direct-constructor fixtures were repointed:
  - `crates/local-deployment/src/lib.rs:1320-1322` (the one that saves `test-refresh-token`)
  - `crates/local-deployment/src/lib.rs:1371-1373`
- **The regression test does not lock it.** `crates/services/src/services/oauth_credentials.rs:259-274` asserts only `assert!(path.exists())` at line 273. Replacing `new_file_backed` with `new` on line 262 would leave that assertion passing in every configuration the suite actually runs:
  - `oauth_credentials.rs:117-121` — under `#[cfg(not(target_os = "macos"))]`, `detect` returns `Backend::File` unconditionally. On Linux (this host and CI) the two constructors are **behaviourally identical**; no assertion can discriminate, because there is no difference to detect.
  - `oauth_credentials.rs:104-108` — on macOS the fallback arm is `cfg!(debug_assertions)`, which is **true** under `cargo test`. So even on macOS the default invocation picks `File`.
  - The only configuration where the test discriminates is `cargo test --release` on macOS with `OAUTH_CREDENTIALS_BACKEND` unset — i.e. exactly the configuration in which the reverted bug has *already written the production Keychain slot*. The guard fires after the damage, and CLAUDE.md §9 documents `cargo test --workspace` (debug) as the validation command.
- **Suggested remediation:** assert the backend variant structurally rather than via the filesystem side effect — e.g. `assert!(matches!(credentials.backend, Backend::File(_)))` (the test is in the same module, so the private field is reachable). That is meaningful on macOS at any opt level. It remains vacuous on Linux by nature; that limitation should be stated in the task file rather than implied away.

---

## Brief item 1 — task 022 keychain isolation: verified except F-R2-02

| Check | Result | Evidence |
|---|---|---|
| `new_file_backed()` is explicit/path-scoped | PASS | `oauth_credentials.rs:55-60` — direct `Backend::File(FileBackend { path })`, no `detect` call |
| Production detection unchanged | PASS | `git diff ae5ee15f..fa0612ee -- crates/services/…` is additions only; `detect` body identical at `oauth_credentials.rs:100-123` |
| Every task-022 fixture that **saves** credentials uses it | PASS | Only saving fixture is `local-deployment/src/lib.rs:1320-1331`; repointed. Second direct constructor at `:1371` also repointed |
| Residual `OAuthCredentials::new` in test code is inert | PASS (checked) | `for_test` at `local-deployment/src/lib.rs:559` still uses `new`, but `from_parts` performs **no** credential I/O — it only calls `oauth_credentials.get().await` (in-memory read) at `local-deployment/src/lib.rs:186-192` and in the `share_sync_config` gate. No `load()`/`save()`/`clear()`. `browser_auth_epoch_is_shared_by_deployment_clones` (a task-022 test) routes through `for_test` and therefore cannot touch a production Keychain entry |
| Test does not mutate `OAUTH_CREDENTIALS_BACKEND` | PASS | No `set_var` anywhere in `oauth_credentials.rs`; `grep` over `crates/` shows the var read only at `oauth_credentials.rs:104` |
| Regression test is **discriminating** | **FAIL** | See F-R2-02 |
| Plan/status/gate evidence coherent | PASS | Task-022 frontmatter now declares 5 files including `crates/services/src/services/oauth_credentials.rs` (`022-…md:9-14`); ledger gate transcript for `594d531c` reports "only declared files changed (2 paths)" and `git show 594d531c --stat` confirms exactly those 2 declared files; `status: passed` at `022-…md:5`; manual-verification step 1 and "Done when" both updated to include `cargo test -p services explicit_file_backend_is_path_scoped` |

Command evidence:

```
$ cargo test -p services --lib -- explicit_file_backend_is_path_scoped --nocapture
running 1 test
test services::oauth_credentials::tests::explicit_file_backend_is_path_scoped ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 315 filtered out

$ DISABLE_WORKTREE_ORPHAN_CLEANUP=1 cargo test -p local-deployment --lib -- \
    browser_auth_epoch_is_shared_by_deployment_clones \
    configured_startup_sync_is_installed_before_constructor_returns \
    raw_api_base_remains_available_when_share_sync_config_is_unavailable
running 3 tests
test tests::browser_auth_epoch_is_shared_by_deployment_clones ... ok
test tests::raw_api_base_remains_available_when_share_sync_config_is_unavailable ... ok
test tests::configured_startup_sync_is_installed_before_constructor_returns ... ok
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 40 filtered out
```

---

## Brief item 2 — busy-snapshot scope split: literal criteria all pass, but see F-R2-01

| Check | Result | Evidence |
|---|---|---|
| Workstream exists and is tracked | PASS | `dev-docs/workstreams/sqlite-busy-snapshot-calibration-stability/README.md` (47 lines, frontmatter `workstream:`/`status: active`/`staging_pointers:`), created in `7b5d6eff`; referenced from `decisions-ledger.md:254,257` |
| Both controls force a genuine WAL read-snapshot → intervening-commit → write-upgrade schedule | PASS | `queries.rs:1381-1414` and `lifecycle.rs:1116-1146`: `pool.begin()` → `SELECT` (opens WAL read snapshot) → independent-row `UPDATE` via `execute(&pool)` on a **different** pooled connection (guaranteed: the tx holds its connection checked out; pool is `min_connections(2)`/`max_connections(10)` at `queries.rs:1293-1296` and `lifecycle.rs:1016-1019`) → write upgrade on the stale snapshot |
| Decoy row is genuinely independent (proves snapshot invalidation, not row conflict) | PASS | `queries.rs` decoy has `server_instance_id = "current-instance"`, excluded from both the SELECT and the UPDATE `WHERE` (`queries.rs:1375-1376`, `1383-1386`, `1403-1408`). `lifecycle.rs` decoy is a different `execution_process` row entirely (`lifecycle.rs:1120-1121`) |
| Require extended code 517 | PASS | `expect_err(…)` at `queries.rs:1413` / `lifecycle.rs:1145` followed by `assert!(is_busy_snapshot(&error), …)`. `is_busy_snapshot` compares `e.code() == "517"` at `lifecycle.rs:995-1000` and `queries.rs:1273-1278` (sqlx surfaces the SQLite **extended** result code). The error is asserted on, not swallowed |
| Live production write-first tests retained | PASS | `queries.rs:1312` and `lifecycle.rs:1034` unchanged, still 200 iterations, still `assert_eq!(…, 0)` |
| No ignores or gate changes hiding failures | PASS | No `#[ignore]` added anywhere in the range; `git diff ae5ee15f..fa0612ee` touches no `Cargo.toml`, no CI config, no `doctest =` setting. Full `cargo test -p db` reports the same pre-existing 7 lib / 3 bulk / 2 doctest ignores |
| Calibration relationship to the production tests preserved | **FAIL** | See F-R2-01 |

Determinism verified empirically (10 consecutive runs, both controls, tree clean at `fa0612ee`):

```
run1:  test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 307 filtered out; finished in 0.89s
run2:  test result: ok. 2 passed; ...  1.29s
run3:  ok. 2 passed  1.22s      run4:  ok. 2 passed  1.34s
run5:  ok. 2 passed  0.97s      run6:  ok. 2 passed  1.14s
run7:  ok. 2 passed  2.43s      run8:  ok. 2 passed  0.83s
run9:  ok. 2 passed  0.90s      run10: ok. 2 passed  0.77s
=== git status ===        (empty)
fa0612ee4ad32bc7f5dbf49d232c29dd24f645a2
```

---

## Ledger / report claim verification

- **`cargo test -p db` counts — claim verified exactly.** `decisions-ledger.md:275-278` claims "302 unit tests, 8 bulk operation tests, the emission conformance test, 6 pragma tests, 8 execution-timestamp tests, 8 variable-inheritance tests, 5 visibility tests, and 11 live doctests; no failures." Measured:

```
$ DISABLE_WORKTREE_ORPHAN_CLEANUP=1 cargo test -p db
  db-…            running 309 tests → ok. 302 passed; 0 failed; 7 ignored
  bulk_operations running  11 tests → ok.   8 passed; 0 failed; 3 ignored
  emission_conformance      1 test  → ok.   1 passed
  sqlite_pragmas            6 tests → ok.   6 passed
  task_execution_timestamps 8 tests → ok.   8 passed
  task_variable_inheritance 8 tests → ok.   8 passed
  task_visibility_discriminator 5 tests → ok. 5 passed
  Doc-tests db             13 tests → ok.  11 passed; 0 failed; 2 ignored
  [exited with code 0]
```

- **Ledger line-citation accurate.** `decisions-ledger.md:255-256` cites the pre-existing failure at `crates/db/src/models/execution_process/queries.rs:1437`. At `ae5ee15f`, line 1437 is the `assert!(` of the failing `busy_snapshot_errors > 0` check (`git show ae5ee15f:crates/db/src/models/execution_process/queries.rs | sed -n '1425,1445p'`). Correct.
- **Gate transcript coherent.** `decisions-ledger.md:262-271` records `commit=594d531c … only declared files changed (2 paths)`; `git show 594d531c --stat` shows exactly 2 files, both in the task's `files:` list.
- **Round-1 report exists and matches.** `.agents/reports/2026-08-22-round-1-cross-model-phase-one.md:76-80` records the calibration failure and pre-commits to the `sqlite-busy-snapshot-calibration-stability` scope split. The split was in fact created in this range.
- **Overstated framing (part of F-R2-01).** `decisions-ledger.md:258-259` and README `:32-34` present the calibration repair as complete without disclosing the lost calibration of the two production tests.

---

## Regressions / scope / side-effect checks

- **No production-code change in range.** The only non-test source change is the additive `new_file_backed` constructor. `crates/local-deployment/src/lib.rs` changes are inside `#[cfg(test)] mod tests`. `crates/db/…` changes are inside `mod lifecycle_event_tests`.
- **No contract or scope contradiction.** Task 022's `STOP` list was amended coherently: `"Any route edit or credential operation in this primitive task."` → `"Any route edit or production credential-backend behavior change; the explicit file-backed test constructor must bypass detection without changing OAuthCredentials::new."` (`022-…md:219`). The new bullet in `spec_summary` (`022-…md:208`) matches what shipped. No route file was touched.
- **No unsafe test side effects introduced.** The repaired controls use `tempfile::TempDir`-backed SQLite files only. The new services test uses `tempfile::tempdir()`. No `set_var`, no process-global mutation, no writes outside temp dirs. The orphan-cleanup guard (`disable_orphan_cleanup_for_tests`) remains called by both direct-constructor fixtures (`local-deployment/src/lib.rs:1319`, `:1364`).
- **Removed `eprintln!` diagnostics.** Both controls dropped their `no_read_then_upgrade(control, …)` output lines. Neutral for correctness; the disambiguation work recorded at `022`/F19-1 (making the two controls' output strings distinguishable) is now moot since neither prints. Worth noting only because that disambiguation was itself a prior review finding.
- **Gates green on the touched crates:**

```
$ cargo clippy -p db -p services -p local-deployment -p deployment --all-targets --all-features -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3m 56s        # no warnings

$ cargo fmt --all -- --check
FMT OK
```

---

## Confirmed NOT silently declared complete

- **O8 accepted crash window — intact.** `docs/plans/local-node-browser-oauth/decisions-ledger.md:190-192` still reads: "O8 residual accepted and explicit: a process crash between SQLite revoke-all and file/Keychain credential clear can leave an over-locked-out node with credentials present. The operator retries disconnect. A durable crash-recovery state would require a separately approved migration." The ledger diff in this range is **append-only** (`git diff ae5ee15f..fa0612ee -- docs/plans/…/decisions-ledger.md` shows `+37` lines, `-0`). The corresponding in-code notes at `phase-1/005-…md:145` and `phase-3/012-…md:177` are untouched.
- **Route work 009–012 — still `ready`, not complete.**

```
006-…: status: ready   007-…: status: ready   008-…: status: ready
009-bind-oauth-initiation-…            : status: ready
010-claim-the-handoff-atomically-…     : status: ready
011-validate-the-candidate-profile-…   : status: ready
012-add-browser-scoped-logout-…        : status: ready
013-…021-…: status: ready
```

  Only `001`–`005` and `022` are `passed`. The ledger states this explicitly at `:249-250`: "Route wiring in tasks 009–012 is still required before SC8 is complete", and task 022's manual-verification step 4 repeats it. No claim of SC8 completion appears anywhere in the range.

---

## Informational (not blocking, pre-existing)

- **`dev-docs/MASTER.md` is stale relative to the workstream tree.** The generated table (`MASTER.md:12-40`) omits `sqlite-busy-snapshot-calibration-stability`, and also omits `local-node-browser-oauth` itself, `local-deployment-test-orphan-cleanup-safety` (the previously-accepted scope split), and `local-node-first-use`. Since the umbrella workstream of this whole effort is already missing, the drift predates this range. CLAUDE.md's no-deferred-remediation rule is satisfied literally — it requires `dev-docs/workstreams/<name>/README.md`, which exists. Regenerating with `wai-status.sh` before the PR would close the gap.
- **Concurrent activity in the worktree.** A `ps` snapshot during my run showed another process executing `cargo test -p services explicit_file_backend_is_path_scoped -- --nocapture` — presumably a sibling review seat. It caused only cargo lock waits; `git status` remained clean and `HEAD` unchanged across all my runs.

---

## Verdict rationale

- Both remediations do the right thing to the production surface: the constructor fix is real and correctly scoped, and the calibration controls are now genuinely deterministic (10/10 verified). Every ledger number I could check held exactly.
- What remains are two integrity defects in the *evidence layer* the WAI process depends on: a shipped docstring that now describes code that no longer exists, two regression assertions that lost their calibration without disclosure, and a regression test that cannot fail under the documented validation command. All three are fixable in-session with small, non-behavioural edits.

VERDICT: REJECT