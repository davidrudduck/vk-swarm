---
id: "602"
phase: 6
title: "create_task auto-trigger hook (fire-and-forget, opt-in)"
status: ready
depends_on: ["601","301"]
parallel: false
conflicts_with: []
files:
  - "crates/server/src/routes/tasks/handlers/core.rs"
irreversible: false
scope_test: "crates/server"
allowed_change: edit
covers_criteria: []
covers_tests: []
---
## Failing test (write first)
In the tasks-handler test module (mirror existing create_task tests). Deterministic by construction (tournament R1 F10): the hook AWAITS stage 1 of start_breakdown_for_task (draft insert — see 301's two-stage structure) before detaching stage 2, so: with auto_breakdown_enabled=false, POST create → NO task_breakdown_proposals row (byte-for-byte unchanged path); with it true + description + no parent_task_id → a proposal row with status='draft' EXISTS at handler return (no polling, no status-race — stage 2 may later mark it failed but that is not asserted here; live end-to-end proof is 701/SC5); with it true but empty description or parent_task_id set → no proposal.


## Change
**File:** crates/server/src/routes/tasks/handlers/core.rs
**Anchor:** create_task handler (~192-299), AFTER the auto-share block (~267-296), BEFORE the final Ok(...).
Insert a guarded fire-and-forget:
```rust
    if project.auto_breakdown_enabled
        && task.parent_task_id.is_none()
        && task.description.as_deref().is_some_and(|d| !d.trim().is_empty())
    {
        let deployment = deployment.clone();
        let task_id = task.id;
        match crate::routes::breakdown::create_draft_proposal(&deployment, task_id).await {
        Ok(proposal) => { crate::routes::breakdown::spawn_breakdown_run(deployment.clone(), proposal); }
        Err(e) => tracing::warn!(task_id = %task_id, error = ?e, "auto-breakdown trigger failed"),
    }
    }
```
(Stage 1 `create_draft_proposal` is AWAITED so the draft row exists deterministically before the handler returns — tournament R1 F10; stage 2 `spawn_breakdown_run` detaches internally.) 301 already structures start_breakdown_for_task in these two stages as pub(crate) fns; if the names/signatures differ, adapt the call site only — if the two-stage split is absent, STOP (301 must be amended).


## Allowed moves
The single insertion at the anchor; at most a `pub(crate)` visibility keyword on 301's existing shared fn (note: that file is otherwise off-limits here). No behaviour change for the disabled path.


## STOP triggers
project binding at the anchor doesn't carry auto_breakdown_enabled (601 threading gap — STOP); the handler's deployment/task bindings differ from researched; exposing the trigger requires structural changes in breakdown.rs.


## Manual verification (record in decisions-ledger)



## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-task-breakdown 602` exits 0
