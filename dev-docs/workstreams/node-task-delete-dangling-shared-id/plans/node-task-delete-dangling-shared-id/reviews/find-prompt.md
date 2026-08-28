ADVERSARIAL TOURNAMENT — FIND + REMEDIATE. You are ONE competitor against 2 peers. Find every way
this breakdown will FAIL an implementer, and for EACH finding propose a concrete, applicable fix.
Scoring: +1 per REAL cited problem, +1 per correct fix — BUT every finding is judged by a PEER (not
you); a finding the peer rules not-real scores 0, a hand-wavy fix scores 0. Quality beats quantity:
a padded/pedantic nit LOSES points. An honest `FINDINGS: 0` beats a rejected nit.

SPEC / PLAN / TASK FILES (relative to --cwd = repo root):
- docs/superpowers/specs/2026-08-05-node-task-delete-dangling-shared-id.md  (FROZEN spec)
- docs/plans/node-task-delete-dangling-shared-id/plan.md
- docs/plans/node-task-delete-dangling-shared-id/phase-1-idempotent-delete.md
- docs/plans/node-task-delete-dangling-shared-id/phase-1/001-extend-hiveharness-with-delete-seed-shared-task-task-row-exists.md
- docs/plans/node-task-delete-dangling-shared-id/phase-1/002-tdd-dangling-shared-task-id-delete-falls-through-locally-non-not-found-still-aborts.md

KEY PRODUCTION FILES to verify anchors against:
- crates/server/src/routes/tasks/handlers/remote.rs (delete_remote_task)
- crates/server/src/routes/tasks/handlers/core.rs (delete_task)
- crates/server/tests/common/mod.rs (HiveHarness)
- crates/server/tests/nodes_routes.rs (sibling test pattern)
- crates/services/src/services/remote_client.rs (RemoteClientError, delete_shared_task)
- crates/remote/src/db/tasks.rs (SharedTask serde shape for the 200 mock body)
- crates/server/src/error.rs (ApiError status mapping)

Attack axes (cite task id + the contradicting repo file:line):
1. Not bite-sized / two concerns in one task. 2. Wrong/non-existent anchor/symbol/Before-text
   (VERIFY against the repo). 3. Ambiguous instruction. 4. allowed_change mismatch. 5. Dependency/
   conflict error or cycle. 6. Unmarked irreversible. 7. Untestable or HOLLOW test (passes without
   the implementation). 8. CONTROL-FLOW GROUNDING: open the real code; a plausible-but-inverted call
   path is a finding — symbol existence ≠ control-flow correctness. Pay special attention to:
   does DELETE /api/tasks/{task_id} actually route through delete_remote_task when
   shared_task_id is set? Does the wiremock harness + real JWT token flow actually reach the mock
   hive DELETE endpoint? Will the mocked 200 SharedTaskResponse body deserialize against the real
   SharedTask struct? 9. Fidelity: an SC/TS clause no task truly delivers (covered-but-hollow);
   walk EACH SC1/SC2/SC3 and TS1/TS2/TS3 to a task.
DOMAIN-SPECIFIC TRAP (this run already produced five over-broad-predicate defects elsewhere): the
fix MUST discriminate hive not-found ONLY (RemoteClientError::is_not_found, Http status 404) — if
any task text would let an implementer write a blanket is_err()/catch-all or string-match, that is
a HIGH finding.

TOURNAMENT RULES (non-negotiable):
- You INSPECT and REPORT; you never mutate the repo. NEVER revert or discard
  working-tree state: no git checkout/restore/stash/reset/clean in ANY form, with
  or without a path argument. Parts of the decomposition you are reviewing are
  UNCOMMITTED working-tree state; a single git checkout destroys it with no trace.
- Propose fixes as TEXT in your findings. Do not apply them yourself.

Output one Markdown row per finding (severity | task | file:line | issue | remediation), then
`FINDINGS: <n>` and a one-line self-assessment of why they survive peer review.
