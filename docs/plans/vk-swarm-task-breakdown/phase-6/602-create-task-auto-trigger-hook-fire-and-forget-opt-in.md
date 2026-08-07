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
In the tasks-handler test module (mirror existing create_task tests): with auto_breakdown_enabled=false, POST create → NO task_breakdown_proposals row (byte-for-byte unchanged path); with it true and a description present and no parent_task_id → a draft proposal row exists after the handler returns (poll briefly if the spawn is async); with it true but empty description or a parent_task_id set → no proposal.


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
        tokio::spawn(async move {
            if let Err(e) = crate::routes::breakdown::start_breakdown_for_task(&deployment, task_id).await {
                tracing::warn!(task_id = %task_id, error = ?e, "auto-breakdown trigger failed");
            }
        });
    }
```
This requires 301's route module to expose its shared private trigger fn as `pub(crate) async fn start_breakdown_for_task(deployment, task_id)` — 301 already factors the trigger path into a shared fn; making it pub(crate) is within THIS task's allowed moves ONLY if it is a visibility keyword change in breakdown.rs; if more than the keyword is needed, STOP (301 must be amended).


## Allowed moves
The single insertion at the anchor; at most a `pub(crate)` visibility keyword on 301's existing shared fn (note: that file is otherwise off-limits here). No behaviour change for the disabled path.


## STOP triggers
project binding at the anchor doesn't carry auto_breakdown_enabled (601 threading gap — STOP); the handler's deployment/task bindings differ from researched; exposing the trigger requires structural changes in breakdown.rs.


## Manual verification (record in decisions-ledger)



## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-task-breakdown 602` exits 0
