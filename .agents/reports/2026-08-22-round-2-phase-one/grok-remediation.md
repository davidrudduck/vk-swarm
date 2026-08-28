# Phase-1 remediation recheck — grok-4.6

**Target:** detached checkout `fa0612ee` (`ae5ee15f..fa0612ee`)
**Range commits:** `183da92f` plan, `594d531c` keychain isolation, `7b5d6eff` busy-snapshot, `fa0612ee` ledger close
**Governing intent:** `docs/superpowers/specs/2026-08-21-local-node-browser-oauth.md`, phase-1 tasks 001–005 + 022, `docs/plans/local-node-browser-oauth/decisions-ledger.md`
**Read-only:** no checkout/restore/stash/reset/clean/commit; review worktree status clean at `fa0612ee`

## Findings

No `[BLOCKING]` or `[SHOULD-FIX]` items.

### [INFO] Path-scope test is existence-only; Linux cannot distinguish `new()` from `new_file_backed()`

`crates/services/src/services/oauth_credentials.rs:258-274` saves via `new_file_backed` and asserts `path.exists()`. That fails if Keychain is selected (no file). On this Linux host `Backend::detect` always returns `File` (`:117-121`), so the same assertion would pass if the test called `new()`. Isolation is the constructor itself (`:55-59` builds `Backend::File` and never calls `detect`). Acceptable; not a remaining keychain hole.

### [INFO] Generated `dev-docs/MASTER.md` does not yet list the new workstream

Tracked README exists at `dev-docs/workstreams/sqlite-busy-snapshot-calibration-stability/README.md` and is named in the ledger (`decisions-ledger.md:253-259`). AGENTS.md's scope-split rule is satisfied. MASTER is generated; absence is hygiene, not a silent deferral.

### [INFO] Task 022 clippy Done-when still omits `-p services`

`022-...md:218` still runs `cargo clippy -p db -p deployment -p local-deployment`. The new file is in `services`. Ledger (`:251-252`) records a separate focused services clippy. Default gate `scope_test` remains `handoff.rs`; the new test is only in `WAI_TEST_CMD`. `task-gate.sh:702-714` always prints that scope path even when the override ran.

## Verification

### 1. Keychain isolation

- `OAuthCredentials::new_file_backed` is explicit and path-scoped: `oauth_credentials.rs:55-59`.
- Production `new()` still calls `Backend::detect` (`:48-52`). `detect` at `fa0612ee` matches `ae5ee15f` (macOS env/`debug_assertions`; non-macOS file).
- Task-022 fixtures that construct credentials use it: save path `local-deployment/src/lib.rs:1320-1330`; non-saving `from_parts` fixture `:1371-1373`. Only one task-022 test calls `save`.
- `for_test` still uses `new()` (`:559`) and does not save (`:530-532`).
- No `OAUTH_CREDENTIALS_BACKEND` process-wide mutation. No route edits. STOP trigger at `022-...md:219` held.
- Task 022 `status: passed`; `files:` includes `oauth_credentials.rs`; gate for `594d531c` records 2 declared paths (`decisions-ledger.md:263-271`).

### 2. Deterministic SQLite calibration

- Workstream present, `status: active`, merge box unchecked (`README.md:4,47`).
- Both controls force deferred `BEGIN` + `SELECT` + other-connection `UPDATE` + original `UPDATE`:
  - `queries.rs:1381-1413`
  - `lifecycle.rs:1120-1148`
- sqlx 0.8.6 `pool.begin()` emits ANSI `BEGIN` (deferred). Extended codes enabled; `is_busy_snapshot` requires `"517"` (`queries.rs:1273-1278`, `lifecycle.rs:995-1000`).
- Production write-first tests unchanged and live: `update_completion_does_not_read_then_upgrade`, `mark_orphaned_as_failed_does_not_read_then_upgrade`. No `#[ignore]`, no gate disable.
- Pre-fix 0/200 cite `queries.rs:1437` at `ae5ee15f` is accurate.

### 3. Phase-1 / O8 / 009–012 honesty

- Remediation diff is 7 files; no `browser_auth` models or `routes/oauth.rs`.
- 001–005 and 022 remain `passed`. 009–012 remain `status: ready`.
- Ledger still: routes 009–012 required before SC8 (`:246`); O8 crash window accepted (`:190-192`). Not marked complete.

### Command evidence

`TMPDIR=/home/david/.cache/dr-panel-tmp/grok-review-tmp`, `DISABLE_WORKTREE_ORPHAN_CLEANUP=1` for local-deployment.

```text
services explicit_file_backend_is_path_scoped: ok. 1 passed
local-deployment (epoch + startup + raw api-base): ok. 3 passed
db calibration controls, 5 consecutive runs: ok. 2 passed each
write-first update_completion: 0/200 SQLITE_BUSY_SNAPSHOT, 0 other errors
write-first mark_orphaned_as_failed: 0/200 SQLITE_BUSY_SNAPSHOT, 0 other errors
```

VERDICT: APPROVE
