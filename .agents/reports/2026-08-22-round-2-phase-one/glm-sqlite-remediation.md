# GLM seat — SQLite calibration remediation recheck (round 2, phase 1)

Panelist: GLM (opencode). Read-only review of `ae5ee15f..fa0612ee` in the detached worktree at
`/home/david/.cache/dr-panel-tmp/dr-panel-gentle-mongoose-fa0612ee-150627` (HEAD verified at
`fa0612ee4ad32bc7f5dbf49d232c29dd24f645a2`, working tree clean). Shared prompt:
`.agents/reports/2026-08-22-round-2-phase-one/remediation-recheck-prompt.md`.

Scope emphasized per dispatch: the two rewritten calibration controls in
`crates/db/src/models/execution_process/lifecycle.rs` and `queries.rs` — whether the new schedule
genuinely creates a WAL read snapshot, commits from another connection, and deterministically
yields extended code 517 — plus pooled-connection assumptions, false positives, transaction
cleanup, unchanged live write-first stress tests, ignore/gate suppression, workstream and ledger
accuracy. Keychain finding spot-checked secondarily.

## 1. The deterministic schedule is mechanically sound

Both controls (`control_prior_status_read_reproduces_busy_snapshot`, lifecycle.rs:1110-1149;
`control_read_then_write_shape_reproduces_busy_snapshot`, queries.rs:1369-1414) now construct the
exact conflicting schedule with no background writer, no sleeps, and no scheduler dependence:

1. **Read snapshot** — `pool.begin()` (sqlx 0.8.6 issues a deferred `BEGIN`), then the SELECT via
   `fetch_optional(&mut *tx)` / `fetch_all(&mut *tx)` (lifecycle.rs:1120-1129,
   queries.rs:1381-1391). The first read statement inside the open explicit transaction acquires
   the WAL read snapshot; within an explicit transaction the read txn persists after the statement
   is reset, until COMMIT/ROLLBACK.
2. **Intervening commit from another connection** — `.execute(&pool).await.unwrap()` on the decoy
   row (lifecycle.rs:1131-1136, queries.rs:1393-1398) while the `Transaction` pins one pooled
   connection. The pool (`build_contention_pool`, lifecycle.rs:1005-1025 / queries.rs:1282-1302:
   real temp file, `SqliteJournalMode::Wal`, `busy_timeout(5s)`, `min_connections(2)`,
   `max_connections(10)`, `acquire_timeout(10s)`) necessarily hands the second acquire a distinct
   connection — structurally guaranteed since a `PoolConnection`/`Transaction` cannot be shared,
   and even a lazy pool would open a fresh connection under max 10. The `.await` guarantees the
   autocommit write is fully committed (write lock released) before the next statement runs.
3. **Write upgrade rejected** — the UPDATE on the stale snapshot (lifecycle.rs:1139-1143,
   queries.rs:1401-1408) must fail with `SQLITE_BUSY_SNAPSHOT`. WAL upgrade rejection is
   snapshot-based, not page-based, so committing the *decoy* row (non-overlapping with the UPDATE
   target) still invalidates the snapshot — the decoy usage is correct. `SQLITE_BUSY_SNAPSHOT` is
   not retried by `busy_timeout`, and no other writer holds the write lock at that instant (the
   intervening commit completed), so plain `SQLITE_BUSY` (5) cannot appear either. The entire
   sequence is await-ordered on one task: there is no timing window left to race.

## 2. Extended code 517 is asserted exactly — no false-positive channel

`is_busy_snapshot` (lifecycle.rs:995-1000, queries.rs:1273-1278) matches `e.code() == "517"`. In
sqlx 0.8.6 (Cargo.lock:5461) `SqliteError::code()` returns `sqlite3_extended_errcode` formatted as
a string (verified in
`~/.cargo/registry/src/*/sqlx-sqlite-0.8.6/src/error.rs`, `DatabaseError for SqliteError::code`),
so the match is against the extended code only — 517 is uniquely `SQLITE_BUSY_SNAPSHOT`, distinct
from primary BUSY (5). Failure modes that would void the control (UPDATE succeeds → `expect_err`
panics; different code → the final `assert!` fails with the full error text) both fail loudly. A
pass therefore proves the schedule fired. The controls share `build_contention_pool` with the live
stress tests, so they calibrate the same harness those tests depend on.

## 3. Empirical results (clean-room target dir)

- Both controls: **15/15 consecutive runs green** (`cargo test -p db --lib control_`, 2 passed, 0
  failed, 0 ignored each run) — reproducing and exceeding the ledger's "ten consecutive runs".
- Live write-first stress tests unchanged and green:
  `update_completion_does_not_read_then_upgrade` (lifecycle.rs:1034) and
  `mark_orphaned_as_failed_does_not_read_then_upgrade` (queries.rs:1312) both ran live, scoring
  **0/200 BUSY_SNAPSHOT, 0 other errors** — the control/stress pairing is meaningful in both
  directions.
- Full `cargo test -p db` on a fresh `CARGO_TARGET_DIR`: 302 passed/7 ignored (lib), 8 passed/3
  ignored (bulk ops), 1 emission conformance, 6 pragma, 8 execution-timestamp, 8
  variable-inheritance, 5 visibility, and **11 doctests passed / 2 ignored, 0 failures** — matching
  the ledger's claim (decisions-ledger.md:275-277) number for number.
- `cargo clippy -p db --all-targets --all-features -- -D warnings`: green (the exact command the
  workstream README cites).
- Focused keychain tests: `services::oauth_credentials::tests::explicit_file_backend_is_path_scoped`
  green; both edited local-deployment fixtures
  (`configured_startup_sync_is_installed_before_constructor_returns`,
  `raw_api_base_remains_available_when_share_sync_config_is_unavailable`) green with
  `DISABLE_WORKTREE_ORPHAN_CLEANUP=1`.

Environment note for the record: the first full-suite attempt failed to *compile* (rustc
`Disk quota exceeded` writing `/tmp/rustc*`), and a follow-up run in the partially poisoned shared
`target/` showed 11 doctest E0460 "possibly newer version of crate `utils`" failures — stale-artifact
fallout from the quota-aborted compile, not a source defect. A clean `CARGO_TARGET_DIR` +
root-filesystem `TMPDIR` run (as the shared prompt prescribes) is fully green. Any panelist seeing
red doctests in this worktree should re-run clean before reading it as a finding.

## 4. Transaction cleanup, ignores, gates

- Cleanup: after `expect_err`, the still-open read txn is dropped → sqlx rolls back on drop when
  the connection returns to the pool; each test owns a private temp-file DB and pool; the old
  leaked-writer task and per-iteration commit/drop matching are gone with the rewrite. No residue.
- `#[ignore]` audit: `git grep -c "#\[ignore"` in `crates/db/src` returns identical counts at
  `ae5ee15f` and `fa0612ee` (7 in `models/project/sync.rs`; plus the 2 pre-existing doc-ignores) —
  all pre-existing, none added, removed, or repurposed. The diff contains no `cfg_attr`, no
  `Cargo.toml`/gate/config changes, no test-list changes. Nothing is suppressed.

## 5. Workstream and ledger accuracy

- `dev-docs/workstreams/sqlite-busy-snapshot-calibration-stability/README.md` exists (new in
  range), `status: active`, with an honest unchecked completion box ("branch merged; mark shipped
  then") and completion criteria that match what I independently verified above. Its origin note
  correctly states the browser-auth diff does not touch the execution-process subsystem.
- The ledger's round-2 entry (decisions-ledger.md:241-277) is accurate against my measurements:
  test counts exact, "ten consecutive runs" reproduced (15/15), "no test was ignored" confirmed,
  and the cited pre-remediation failure line `queries.rs:1437` refers to the pre-remediation file
  (1458 lines), which is correct for the state it describes.

## 6. Keychain finding (spot check) and no-silent-completion checks

- `OAuthCredentials::new_file_backed()` constructs `Backend::File(FileBackend { path })` directly
  (oauth_credentials.rs:55-60); `new()`/`Backend::detect()` are byte-unchanged (macOS env override
  with `cfg!(debug_assertions)` file default, file on other OSes). Both task-022 direct-constructor
  fixtures use it (local-deployment/src/lib.rs:1320, 1370). The only remaining `new(` sites are
  `for_test()` (lib.rs:559, temp path, never saves in fixture flows — the only `.save(` callers are
  the production `services/auth.rs:32` and the new test) and production `Deployment::new`
  (lib.rs:601). The regression test is platform-independent and never mutates
  `OAUTH_CREDENTIALS_BACKEND`. Task 022's plan file was amended coherently (file-set now declares
  `oauth_credentials.rs`; gate command includes the services test) and remains `status: passed`
  with matching gate evidence.
- Tasks 009–012 all still `status: ready`; spec `status: active`; the O8 crash-window residual is
  still recorded as accepted (ledger:190) and the new entry re-affirms "Route wiring in tasks
  009–012 is still required before SC8 is complete." Nothing was silently declared complete.

## 7. Minor, non-blocking observations

1. `explicit_file_backend_is_path_scoped` pins the path-scoped file contract, but on non-macOS
   (and macOS debug default) a regression back to `Backend::detect()` would still pass it, since
   detect selects File there anyway; its discriminating power against keychain selection exists
   only under macOS + `OAUTH_CREDENTIALS_BACKEND=keychain`/release. The stated requirements
   (explicit constructor, unchanged detection, fixtures converted) are nonetheless fully met, and
   the fixture-level guarantee no longer depends on the test.
2. The ledger's pasted task-022 gate log excerpt shows only the handoff-scope test line; the full
   `WAI_TEST_CMD` chain (services + three local-deployment tests) is attested in prose and I
   verified those tests green directly. Cosmetic only.

## Conclusion

The remediation converts both probabilistic controls into an await-sequenced, deterministic
WAL-snapshot schedule whose pass condition is exactly extended code 517; the live write-first
stress tests are untouched and still exercise the real production functions; no ignore, gate, or
config suppression exists anywhere in the range; the workstream and ledger claims are accurate and
independently reproduced; the keychain fix is explicit, path-scoped, and leaves production
detection unchanged; pending work (009–012, SC8, O8 residual) remains honestly open. No blocking or
should-fix finding remains.

VERDICT: APPROVE
