---
id: "301"
phase: 3
title: "REST API: breakdown lifecycle endpoints + dependencies query"
status: ready
depends_on: ["103","203","204"]
parallel: false
conflicts_with: []
files:
  - "crates/server/src/routes/breakdown.rs"
  - "crates/server/src/routes/mod.rs"
siblings: ["crates/server/src/routes/labels.rs","crates/server/src/routes/tasks/handlers/core.rs"]
irreversible: false
scope_test: "crates/server"
allowed_change: mixed
covers_criteria: []
covers_tests: ["TS3"]
---
## Failing test (write first)
In crates/server/src/routes/breakdown.rs `#[cfg(test)] mod tests` using db::test_utils::create_test_pool() and direct handler-fn calls or the crate's existing router-test harness (read how labels.rs / tasks handler tests are structured FIRST and mirror; if the crate tests handlers via full axum Router, do the same):
1. test_trigger_creates_draft_and_409_on_second — POST breakdown for a task → 200 with draft proposal; second POST → 409 (map the unique-index violation).
2. test_review_gate_no_outbox_before_accept — after trigger + items present, assert BOTH: zero node_outbox rows whose entity_id equals the proposal id or ANY item id, AND the entity_type='task' row count is unchanged from before the trigger (the task-count check alone would pass with proposal rows present; CodeRabbit PR470).
3. test_accept_returns_children_and_edges — seed a draft with 2 items (B dep A) via db fns; POST accept → 200 listing 2 tasks; GET dependencies for B's task returns the A edge.
4. test_edit_items_only_in_draft — PUT items on an accepted proposal → 4xx.
5. test_discard_and_retry — discard → status discarded; retry on a failed proposal creates a fresh draft run.
6. test_spawn_failure_marks_failed (tournament R1 F9) — force stage-2 failure (e.g. a task/project state that makes attempt creation error, or an unresolvable executor profile); for determinism call the pub(crate) spawn_breakdown_run(...) DIRECTLY and AWAIT it in the test (do not race the detached handler path; CodeRabbit PR470 R2); then assert the proposal ends status='failed' with error set (NOT a stranded draft) and that a subsequent trigger succeeds with a new draft.
Note: the trigger's attempt-spawn is exercised only as far as the test harness allows without a real executor (assert the proposal row + linked attempt row exist; do NOT spawn a real CLI in tests — stub via the harness's existing pattern if present, else assert up to the DB effects and record the boundary in the ledger).
7. test_remote_project_rejected — seed a task whose project is remote/mirrored (mirror how create_task_and_start's guard is tested, or construct the project state core.rs:305-316 rejects); POST breakdown → 4xx and NO proposal row created.


## Change
**File:** crates/server/src/routes/breakdown.rs (new) — read sibling labels.rs first for the single-file route-module shape (router fn + handlers + ApiResponse envelope + ApiError mapping). Handlers (all Result<ResponseJson<ApiResponse<T>>, ApiError>):
- `POST /tasks/{task_id}/breakdown` trigger_breakdown: structured as TWO shared pub(crate) fns — `create_draft_proposal(deployment, task_id)` (stage 1) and `spawn_breakdown_run(deployment, proposal)` (stage 2, detaches internally) — composed by the handler (tournament R1 F9/F10): STAGE 1 (synchronous, awaited): load task (404 if absent); reject tasks of remote/mirrored projects with the SAME guard + error create_task_and_start uses (tasks/handlers/core.rs:305-316) BEFORE creating a draft — attempts execute only on the origin node (spec Constraints; CodeRabbit PR470); TaskBreakdownProposal::create (map the one-draft unique violation → the crate's 409/Conflict ApiError variant — read error.rs first; if no 409-shaped variant exists STOP). STAGE 2 (spawn): create a TaskAttempt on the task (mirror create_task_and_start's attempt creation, tasks/handlers/core.rs:305-452), then call deployment.container().start_breakdown_attempt(&attempt, project_default_profile, BreakdownService::breakdown_prompt(...)) (the 204 entry point) and link_execution_process on the proposal. ANY error in stage 2 (attempt creation, spawn, linking) must mark the proposal Failed with the error text — never leave a stranded draft (the unique index would 409-block retriggering forever). The route handler awaits stage 1 (so a draft row exists deterministically at response time) and stage 2's outcome may be deferred; return the proposal.
- `GET /tasks/{task_id}/breakdown` get_breakdown → proposal + items; when none exists return 200 with ApiResponse success and data: null (ONE contract — do NOT use HTTP 204; clients and tests assert the 200/null shape; CodeRabbit PR470 R2).
- `PUT /breakdown-proposals/{id}/items` put_items(Json<UpsertProposalItems>) → replace_items (draft-only errors → 409).
- `POST /breakdown-proposals/{id}/accept` accept → accept_proposal → Vec<Task>.
- `POST /breakdown-proposals/{id}/discard` discard → update_status(Discarded, None).
- `POST /breakdown-proposals/{id}/retry` retry: only when status Failed → new draft + new run (reuse trigger path via a shared private fn).
- `GET /tasks/{task_id}/dependencies` get_dependencies → Vec<TaskDependency>.
`pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl>` matching sibling.

**File:** crates/server/src/routes/mod.rs — Anchor: the `pub mod` list (lines ~8-40) and base_routes merge chain (~48-80). Add `pub mod breakdown;` and `.merge(breakdown::router(&deployment))` alongside the tasks entry.


## Allowed moves
Create breakdown.rs; the two mod.rs lines. NO edits to tasks/handlers/core.rs (mirroring means reading it, not changing it). If start_attempt requires plumbing not reachable from a route handler, STOP.


## STOP triggers
ApiError lacks a 409-shaped variant (check error.rs; if truly absent, STOP and escalate rather than inventing one here — error.rs is unlisted); the attempt-creation path researched in core.rs:305-452 requires fields a breakdown context cannot supply; 204's start_breakdown_attempt is absent or its signature differs.


## Manual verification (record in decisions-ledger)



## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-task-breakdown 301` exits 0
