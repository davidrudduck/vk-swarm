Full report content:

# Phase-1 remediation recheck

## Findings

No blocking or should-fix findings remain in `ae5ee15f..fa0612ee`.

## Standards

No standards violations found.

A possible concern was the compact task-022 gate transcript at [decisions-ledger.md](</home/david/.cache/dr-panel-tmp/dr-panel-gentle-mongoose-fa0612ee-150627/docs/plans/local-node-browser-oauth/decisions-ledger.md:260>). This is not a finding: the gate runner executes the complete `WAI_TEST_CMD` but redirects its output and reports the generic scope as green (`/home/david/.agents/wai/scripts/task-gate.sh:167-191,702-714`). The updated command includes the service and local-deployment regressions at [022-fence-browser-login-commit-against-explicit-disconnect.md](</home/david/.cache/dr-panel-tmp/dr-panel-gentle-mongoose-fa0612ee-150627/docs/plans/local-node-browser-oauth/phase-1/022-fence-browser-login-commit-against-explicit-disconnect.md:222>), while the ledger separately records their successful execution and focused clippy at [decisions-ledger.md](</home/david/.cache/dr-panel-tmp/dr-panel-gentle-mongoose-fa0612ee-150627/docs/plans/local-node-browser-oauth/decisions-ledger.md:247>).

No ignored tests, disabled doctests, quality-gate changes, migrations, route edits, or undeclared production behavior changes occur in the remediation range. `git diff --check ae5ee15f..fa0612ee` exited successfully with no output.

## Spec

### 1. Task-022 credential isolation

Verified:

- `OAuthCredentials::new_file_backed(path)` constructs `Backend::File(FileBackend { path })` directly at [oauth_credentials.rs](</home/david/.cache/dr-panel-tmp/dr-panel-gentle-mongoose-fa0612ee-150627/crates/services/src/services/oauth_credentials.rs:55>).
- Production `OAuthCredentials::new()` still delegates to `Backend::detect` at [oauth_credentials.rs](</home/david/.cache/dr-panel-tmp/dr-panel-gentle-mongoose-fa0612ee-150627/crates/services/src/services/oauth_credentials.rs:48>). Comparison with `git show ae5ee15f:crates/services/src/services/oauth_credentials.rs` confirms that `new()` and detection at [oauth_credentials.rs](</home/david/.cache/dr-panel-tmp/dr-panel-gentle-mongoose-fa0612ee-150627/crates/services/src/services/oauth_credentials.rs:100>) are unchanged.
- The only task-022 fixture that saves credentials constructs them explicitly at [local-deployment/src/lib.rs](</home/david/.cache/dr-panel-tmp/dr-panel-gentle-mongoose-fa0612ee-150627/crates/local-deployment/src/lib.rs:1317>) before saving the token at line 1323. The other direct `from_parts` fixture also uses `new_file_backed` at line 1371. The ordinary `for_test()` constructor still uses `new`, but its credentials are explicitly neither loaded nor saved at lines 527-559.
- The focused regression saves through the explicit constructor and requires the supplied path to exist at [oauth_credentials.rs](</home/david/.cache/dr-panel-tmp/dr-panel-gentle-mongoose-fa0612ee-150627/crates/services/src/services/oauth_credentials.rs:258>). A Keychain save would not create that supplied file, so the assertion distinguishes the unsafe backend side effect without mutating `OAUTH_CREDENTIALS_BACKEND`.
- Task 022 declares the new file, constructor, fixture requirement, verification command, and evidence requirement coherently at [022-fence-browser-login-commit-against-explicit-disconnect.md](</home/david/.cache/dr-panel-tmp/dr-panel-gentle-mongoose-fa0612ee-150627/docs/plans/local-node-browser-oauth/phase-1/022-fence-browser-login-commit-against-explicit-disconnect.md:9>), lines 107-112, 178-184, and 222-233.

No unsafe credential or environment side effect remains in the changed fixtures.

### 2. SQLite busy-snapshot calibration

Verified:

- The promised workstream exists, remains `active` pending merge, and documents its origin and completion criteria at [README.md](</home/david/.cache/dr-panel-tmp/dr-panel-gentle-mongoose-fa0612ee-150627/dev-docs/workstreams/sqlite-busy-snapshot-calibration-stability/README.md:1>).
- Both pools use WAL and at least two connections, allowing a transaction-held connection plus a distinct pooled writer: [lifecycle.rs](</home/david/.cache/dr-panel-tmp/dr-panel-gentle-mongoose-fa0612ee-150627/crates/db/src/models/execution_process/lifecycle.rs:1002>) and [queries.rs](</home/david/.cache/dr-panel-tmp/dr-panel-gentle-mongoose-fa0612ee-150627/crates/db/src/models/execution_process/queries.rs:1280>).
- The lifecycle control begins a transaction, performs its `SELECT`, awaits an autocommitted update through the pool, then attempts the original transaction’s write upgrade at [lifecycle.rs](</home/david/.cache/dr-panel-tmp/dr-panel-gentle-mongoose-fa0612ee-150627/crates/db/src/models/execution_process/lifecycle.rs:1110>).
- The orphan-recovery control forces the same schedule at [queries.rs](</home/david/.cache/dr-panel-tmp/dr-panel-gentle-mongoose-fa0612ee-150627/crates/db/src/models/execution_process/queries.rs:1368>).
- Both require extended code `517`, rather than accepting generic contention, through `is_busy_snapshot` at `lifecycle.rs:995-999` and `queries.rs:1273-1277`. SQLx 0.8.6 obtains `sqlite3_extended_errcode` and exposes it through `DatabaseError::code`, so the comparison is genuinely against the extended code.
- The live write-first production-shape tests remain enabled and unchanged at `lifecycle.rs:1033-1094` and `queries.rs:1311-1362`.
- The remediation diff introduces no `#[ignore]`, Cargo test suppression, doctest disabling, or quality-gate configuration change.

The implementation matches the workstream’s required read-snapshot/intervening-commit/write-upgrade sequence and exact-code assertion at [README.md](</home/david/.cache/dr-panel-tmp/dr-panel-gentle-mongoose-fa0612ee-150627/dev-docs/workstreams/sqlite-busy-snapshot-calibration-stability/README.md:28>).

### 3. Scope and lifecycle claims

Verified:

- Task 022 is `passed`, but covers no success criterion itself, at [022-fence-browser-login-commit-against-explicit-disconnect.md](</home/david/.cache/dr-panel-tmp/dr-panel-gentle-mongoose-fa0612ee-150627/docs/plans/local-node-browser-oauth/phase-1/022-fence-browser-login-commit-against-explicit-disconnect.md:1>). Its status therefore does not claim SC8 is complete.
- The accepted O8 crash window remains explicit at [decisions-ledger.md](</home/david/.cache/dr-panel-tmp/dr-panel-gentle-mongoose-fa0612ee-150627/docs/plans/local-node-browser-oauth/decisions-ledger.md:190>).
- The ledger expressly says route wiring 009–012 remains required before SC8 at [decisions-ledger.md](</home/david/.cache/dr-panel-tmp/dr-panel-gentle-mongoose-fa0612ee-150627/docs/plans/local-node-browser-oauth/decisions-ledger.md:242>).
- Tasks 009, 010, 011, and 012 each remain `status: ready` in their respective phase-2/phase-3 task files. No server route appears in the remediation file list.

No accepted residual or later route work was silently declared complete.

## Command evidence and limitation

Fresh read-only checks established:

- `HEAD` equals `fa0612ee4ad32bc7f5dbf49d232c29dd24f645a2`.
- `git status --short` produced no output.
- `git diff --check ae5ee15f..fa0612ee` produced no output.
- The range contains four commits and exactly seven changed files: four Rust files, task 022, the decisions ledger, and the new calibration workstream.

Focused Cargo tests were attempted with `DISABLE_WORKTREE_ORPHAN_CLEANUP=1` and `TMPDIR=/var/tmp`, but Cargo could not start because the read-only sandbox prohibited creating `target/debug/.cargo-lock` (`os error 30`). Accordingly, this report does not claim fresh test or clippy success. The branch records earlier focused tests, ten repeated calibration runs, DB-suite execution, and clippy results at [README.md](</home/david/.cache/dr-panel-tmp/dr-panel-gentle-mongoose-fa0612ee-150627/dev-docs/workstreams/sqlite-busy-snapshot-calibration-stability/README.md:36>) and [decisions-ledger.md](</home/david/.cache/dr-panel-tmp/dr-panel-gentle-mongoose-fa0612ee-150627/docs/plans/local-node-browser-oauth/decisions-ledger.md:247>); static inspection found those claims coherent with the committed code and gate mechanism.

Standards findings: 0. Spec findings: 0. Worst issue on either axis: none.

VERDICT: APPROVE