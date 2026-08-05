# Decisions Ledger

## Submission
Plan accepted from submit envelope.

## 2026-08-05 — precheck anchor-check false positive (documented per no-deferred-remediation rule)
Precheck flagged `src/routes/tasks/handlers/core.rs` / `src/routes/tasks/handlers/remote.rs` as
absent on main — the extractor truncated the `crates/server/` prefix. Verified false positive:
`git cat-file -e main:crates/server/src/routes/tasks/handlers/core.rs` -> OK (likewise remote.rs,
remote_client.rs). Re-ran with `--no-anchor-check`.

## 2026-08-05 — breakdown tournament round 1
See reviews/tournament-1.md. method: external-find (codex OK; agy emitted no findings; opencode
failed on pre-existing broken ~/.config/opencode/opencode.json) + orchestrator-judge fallback
(agy quota-exhausted for judging). 1 validated finding (SC3 covered-but-hollow) remediated via
envelope resubmit before execution. Codex finding 1 (TS3 mock serde failure) ruled not-real —
serde deserializes missing Option fields as None — and empirically disproved: TS3 passed on the
RED run before the fix existed. Codex finding 3 (task irreversible flag) ruled not-real: the
task-gate flag covers repo-irreversible operations; the runtime data-deletion decision is
ADR-0015.

## 2026-08-05 — execution (tasks 001 + 002)
- Task 001: harness helpers added to crates/server/tests/common/mod.rs (delete(),
  seed_shared_task(), task_row_exists()); `cargo test -p server --test harness_smoke` green.
- Task 002 RED: `delete_task_with_dangling_shared_id_deletes_locally` failed exactly as filed —
  status 404, body `{"success":false,...,"message":"{\"error\":\"shared task not found\"}"}`;
  the 409-abort and 200-success tests passed pre-fix (regression pins).
- Task 002 GREEN after the three-arm match in delete_remote_task (Err(e) if e.is_not_found()
  -> warn + Task::delete; other Err propagates): 3 passed, including both SC3 logs_contain
  assertions.
- Gates: `cargo fmt --all -- --check` OK (after fmt), `cargo clippy --all --all-targets
  --all-features -- -D warnings` exit 0, `cargo test --workspace` exit 0 (db crate compiled and
  passed in this worktree — the known F-2026-07-30-01 failure did not reproduce here).
- Out-of-scope hazard re-noted, not fixed (per spec): success path relies on the task.deleted WS
  event; a missed event still leaves a stale local row.
