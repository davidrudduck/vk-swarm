---
id: "301"
phase: 3
title: "REST API: breakdown lifecycle endpoints + dependencies query"
status: ready
depends_on: ["103","203"]
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
2. test_review_gate_no_outbox_before_accept — after trigger + items present, assert node_outbox has NO rows for the proposal/items (entity_type='task' count unchanged).
3. test_accept_returns_children_and_edges — seed a draft with 2 items (B dep A) via db fns; POST accept → 200 listing 2 tasks; GET dependencies for B's task returns the A edge.
4. test_edit_items_only_in_draft — PUT items on an accepted proposal → 4xx.
5. test_discard_and_retry — discard → status discarded; retry on a failed proposal creates a fresh draft run.
Note: the trigger's attempt-spawn is exercised only as far as the test harness allows without a real executor (assert the proposal row + linked attempt row exist; do NOT spawn a real CLI in tests — stub via the harness's existing pattern if present, else assert up to the DB effects and record the boundary in the ledger).


## Change
**File:** crates/server/src/routes/breakdown.rs (new) — read sibling labels.rs first for the single-file route-module shape (router fn + handlers + ApiResponse envelope + ApiError mapping). Handlers (all Result<ResponseJson<ApiResponse<T>>, ApiError>):
- `POST /tasks/{task_id}/breakdown` trigger_breakdown: load task (404 if absent); TaskBreakdownProposal::create (map unique violation → ApiError::Conflict / the crate's 409 variant); create a TaskAttempt on the task and call deployment.container().start_attempt(&attempt, executor_profile_id_from_project_default, false) with an ExecutorAction whose initial prompt is BreakdownService::breakdown_prompt(...) and run_reason Breakdown — read how create_task_and_start builds attempt+start (tasks/handlers/core.rs:305-452) and mirror EXACTLY the parts needed, diverging only in run_reason and prompt (justify divergences in ledger); link_execution_process on the proposal.
- `GET /tasks/{task_id}/breakdown` get_breakdown → proposal + items (204/null data when none).
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
ApiError lacks a 409-shaped variant (check error.rs; if truly absent, STOP and escalate rather than inventing one here — error.rs is unlisted); the attempt-creation path researched in core.rs:305-452 requires fields a breakdown context cannot supply; ExecutorAction construction for an initial prompt is not expressible without executor-crate changes.


## Manual verification (record in decisions-ledger)



## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-task-breakdown 301` exits 0
