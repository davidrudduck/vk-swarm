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

## Post-review known issues (2026-08-28, pre-graduation review)
Adjudicated non-actionable findings from the close-gate code review (records:
`reviews/code-review-round-1.md`, `reviews/code-review-round-2.md`). Logged here so they do not
resurface as fresh findings in later rounds.

| # | Item | Disposition |
|---|------|-------------|
| K1 | `is_not_found()` matches ANY 404 from the configured hive base URL — an intermediary/gateway 404 (not "task row gone") also triggers irreversible local deletion | Accepted residual risk under ADR-0015 (status-only discrimination, deliberately decoupled from hive response bodies) |
| K2 | The 404 fall-through's bare `Task::delete` skips the attempt worktree cleanup the guarded core path performs — orphaned on-disk attempt worktrees possible | Edge case; DB stays consistent (FK `ON DELETE SET NULL`); ADR-0015 accepts row/attempts destruction, disk state recorded here |
| K3 | `seed_shared_task` duplicates the `CreateTask` block inside `seed_project` | Harness-local cosmetic duplication; not worth churn |
| K4 | tests/common Resp-mapping block is an 8th near-verbatim copy | Pre-existing pattern across the harness; extraction is standalone cleanup |
| K5 | Theoretical: a future caller passing `shared_task_id = None` to `delete_remote_task` would get a silent 202 no-op | Single guarded caller (`core.rs` `is_some()` guard); precondition documented in the fn doc comment |
| K6 | INFRA (new finding from close-time live verification): an external `sqlite3` CLI read against the LIVE node DB unlinks the node's WAL/SHM on CLI clean close; subsequent node writes commit into the unlinked inode and are lost on node exit (observed: task resurrection after graceful stop). Reproducible only when a CLI read happens mid-flow; API-only flows are durable. Pre-existing hazard for ANY admin/monitor sqlite3 read on a live DB — unrelated to this diff's code. | Filed to backlog; evidence protocol for future live verification: API reads mid-flow, CLI reads only after node shutdown |

## 2026-08-28 — pre-graduation code review (close gate)
Two-round HIGH-effort review per `/wai:close`. Round 1 (2 parallel finders, verified against
HEAD e51147c9) surfaced 3 actionable findings — dead `HiveHarness::delete()`, the unreachable
no-shared-id `else` branch in `delete_remote_task`, and the Ok-arm comment/log that claimed the
WS handler "cleans the local cache" while it actually soft-unlinks and retains the row — all
fixed (remote.rs + tests/common/mod.rs) with gates re-run green (fmt OK; clippy
`-p server --tests --all-features -D warnings` exit 0; `tasks_delete_routes` 3/3). Round 2
verified all three fixes (control-flow equivalent; no string assertions broken) and raised one
theoretical non-actionable (K5). Convergence: `Actionable: []` (round-2 record, Verdict:
Approve).

## Reachability gate (2026-08-28, close-time re-verification)
- `cargo fmt --all -- --check` — clean on the reviewed tree.
- `cargo clippy -p server --tests --all-features -- -D warnings` — exit 0 on the reviewed tree.
- `cargo test -p server --test tasks_delete_routes` — 3/3 passed, including
  `delete_task_with_dangling_shared_id_deletes_locally` (SC1+SC3 with logs_contain) and the
  409-abort retention pin (SC2).
- Live end-to-end re-run (release binary from this branch, stub-hive on loopback, real served
  router, real browser-session auth): SC1 DELETE with dangling `shared_task_id` (hive 404) →
  **202**, task row absent after shutdown (`SELECT count(*) FROM tasks;` → `0`); SC2 hive 409 →
  **409** surfaced to the client, row durably retained (`1|sc2-retain-me`); SC3 warn emitted
  naming both ids. Full transcripts under `## Deploy verification` below.
- Spec criteria SC1, SC2, SC3: all directly demonstrated on the running server.

VERDICT: PASS

## Deploy verification (2026-08-28)
Method: release binary (`/data/.cache/cargo-target/release/vks-node-server`) built from this
branch (including the review fixes), node served on `10.69.96.233:9012` with a scratch DB,
`VK_SHARED_API_BASE` pointed at a stub hive on loopback (`~/.cache/vlnbo-close/stub-hive.mjs`;
unmatched paths → 404; `STUB_DELETE_MODE=409` makes hive DELETE return 409 for the SC2 leg).
Auth via the real browser OAuth curl flow (handoff init → authorize → complete) against the
stub. DB counts read via sqlite3 CLI only AFTER graceful node shutdown (K6 protocol).

SC1 + SC3 — dangling shared id (hive 404) deletes locally, warns:

```
$ curl -s -o /dev/null -w '%{http_code}\n' -b jar -c jar \
    -H 'Content-Type: application/json' \
    -d '{"project_id":"<uuid>","title":"cli-free-delete","shared_task_id":"aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee"}' \
    http://10.69.96.233:9012/api/tasks
201
$ curl -s -o /dev/null -w '%{http_code}\n' -b jar -X DELETE http://10.69.96.233:9012/api/tasks/<task_id>
202
$ curl -s -b jar http://10.69.96.233:9012/api/tasks
{"data":[],"success":true}
(node graceful stop; then, offline:)
$ sqlite3 db.sqlite 'SELECT count(*) FROM tasks;'
0
(node log, SC3:)
2026-08-28T03:03:22.524627Z  WARN server::routes::tasks::handlers::remote: Hive returned not-found for dangling shared_task_id; deleting task locally task_id=fdb20126-bea8-4ad3-9f51-62f83d0ed55c shared_task_id=aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee
(stub-hive log:)
DELETE /v1/tasks/aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee body={"version":null}
```

SC2 — non-not-found hive error (409) aborts and retains:

```
(STUB_DELETE_MODE=409, fresh node + fresh DB, task 'sc2-retain-me' with dangling shared id)
$ curl -s -o /dev/null -w '%{http_code}\n' -b jar -X DELETE http://10.69.96.233:9012/api/tasks/<task_id>
409
$ curl -s -b jar http://10.69.96.233:9012/api/tasks
{"data":[{"title":"sc2-retain-me",...}],"success":true}
(node graceful stop; then, offline:)
$ sqlite3 db.sqlite 'SELECT count(*), title FROM tasks;'
1|sc2-retain-me
```
