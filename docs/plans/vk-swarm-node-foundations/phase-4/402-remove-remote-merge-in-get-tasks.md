---
id: "402"
phase: 4
title: Remove request-time remote task merge from get_tasks (local-only)
status: ready
depends_on: ["401"]
parallel: false
conflicts_with: ["401"]
files:
  - crates/server/src/routes/tasks/handlers/core.rs
irreversible: false
scope_test: "N/A"
allowed_change: edit
covers_criteria: [SC5]
---
## Failing test (write first)

`N/A — covered by: crates/db/tests/task_visibility_discriminator.rs` (task 401's tests pin the
local-only visibility behavior at the DB layer; this task removes the server-side merge that would
otherwise re-introduce remote rows). Behavior preservation of the remaining local path is asserted by
the existing handler/route tests and `cargo check -p server`. No new server-layer unit test is added
because `get_tasks` requires a full `DeploymentImpl` + live Hive client to exercise — out of scope for
a cheap unit test; the negative ("no remote rows") is structurally guaranteed once the merge code and
the hive-only branch are deleted.

## Change

> **Anchor discrepancy (recorded):** the brief/ADR cite L97–173, but the real request-time remote
> merge spans the whole swarm-linked branch **L52–223** (Step 2b fetch + Step 2c merge/dedup +
> sort/return) AND a second hive-only branch **L225–316** (Step 3, "project not found locally or is
> remote"). Both leak remote rows into the node UI and must go. The cited range was partial.

- **File:** `crates/server/src/routes/tasks/handlers/core.rs`
- **Anchor:** function `get_tasks` — entire body from the opening brace at L38 through its final
  `}` at L317. (Signature L35–38 is unchanged.) Replace the whole body.
- **Before:** the current body, which is (abbreviated to its load-bearing structure — match the live
  text exactly when editing):
```javascript
    use std::collections::HashMap;

    let pool = &deployment.db().pool;
    let project_id = query.project_id;

    // Step 1: Try to find the project locally (by ID or by remote_project_id)
    let project = match Project::find_by_id(pool, project_id).await? {
        Some(p) => Some(p),
        None => Project::find_by_remote_project_id(pool, project_id).await?,
    };

    // Step 2: If project exists locally and is NOT remote, fetch tasks from local DB
    // For swarm-linked projects, we also need to fetch from Hive and merge
    if let Some(ref project) = project
        && !project.is_remote
    {
        let local_tasks =
            Task::find_by_project_id_with_attempt_status(pool, project.id, query.include_archived)
                .await?;

        // Step 2a: If not swarm-linked, return local tasks only
        let Some(remote_project_id) = project.remote_project_id else {
            return Ok(ResponseJson(ApiResponse::success(local_tasks)));
        };

        // ... Step 2b (remote_client + list_swarm_project_tasks), Step 2c (HashMap merge/dedup),
        // ... sort, and `return Ok(ResponseJson(ApiResponse::success(merged)));`  ← L52–223
    }

    // Step 3: Project not found locally or is remote - fetch from Hive only
    // ... node_auth_client/remote_client + list_swarm_project_tasks + map SharedTask → ...
    // ... `Ok(ResponseJson(ApiResponse::success(tasks)))`  ← L225–316
```

- **After:** replace the entire body with the local-only implementation:
```javascript
    let pool = &deployment.db().pool;
    let project_id = query.project_id;

    // Node-local only (ADR-0002 / SC5): resolve the project locally and return ITS tasks.
    // The visibility discriminator lives in `Task::find_by_project_id_with_attempt_status`
    // (task 401). No request-time Hive merge and no hive-only proxy — a node renders only its
    // own local work. Inbound sync still writes remote rows; we simply do not surface them here.
    let project = match Project::find_by_id(pool, project_id).await? {
        Some(p) => Some(p),
        None => Project::find_by_remote_project_id(pool, project_id).await?,
    };

    let Some(project) = project else {
        return Err(ApiError::NotFound(format!("Project {project_id} not found")));
    };

    let tasks =
        Task::find_by_project_id_with_attempt_status(pool, project.id, query.include_archived)
            .await?;

    Ok(ResponseJson(ApiResponse::success(tasks)))
```

## Allowed moves

- Replace ONLY the body of `get_tasks`. Do not change its signature.
- The fn-local `use std::collections::HashMap;` disappears with the deleted merge — correct, it is
  used nowhere else in the file (verified). Do NOT remove module-level `use` lines: `remote::db::tasks`,
  `node_auth_client`/`remote_client`, and `Uuid` are still used by `get_task` and the create/update
  handlers in this same file.
- Do NOT touch any other function (`get_task`, `create_task`, `update_task`, `delete_task`,
  `create_task_and_start`).
- Do NOT touch the sync layer (SC5d): no `upsert_remote_task`, publisher, WS runner, or
  `remote_*`/`shared_task_id` writer.

## STOP triggers

- The `get_tasks` body does not match the structure above (someone reshaped the merge) — re-read and
  re-anchor before editing.
- Removing the body leaves an unused module-level import that trips `-D warnings` (means another
  handler stopped using `remote::db::tasks`/`Uuid` independently) — STOP; that is a separate cleanup,
  not this task.
- `ApiError::NotFound` is not in scope / has a different signature — re-confirm the error variant.
- The edit would require touching any file other than `core.rs`.

## Manual verification (record in decisions-ledger)

(`scope_test: N/A` because `get_tasks` needs a full `DeploymentImpl` + Hive client to exercise — too
heavy for a cheap unit test; the negative "no remote rows" is structurally guaranteed once the merge
and hive-only branches are deleted. These observable checks stand in for it.)

1. `cargo check -p server` → exits 0 (the fn-local `HashMap` import vanished with the body; no unused
   module-level import).
2. `git grep -nF 'list_swarm_project_tasks' crates/server/src/routes/tasks/handlers/core.rs` →
   returns ONLY hits inside `get_task` (the remaining single-task Hive fallback, untouched here); NO
   hit inside `get_tasks`. Record the line numbers.
3. `git grep -nF '.list_swarm_project_tasks(remote_project_id)' crates/server/src/routes/tasks/handlers/core.rs`
   → no output (the request-time list merge is gone).
   Record PASS/FAIL for "get_tasks is local-only" in the decisions-ledger.

## Done when

`WAI_TYPECHECK_CMD="cargo check -p server" WAI_TEST_CMD="cargo test -p server tasks::" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-node-foundations 402` exits 0

> Trap 1: server crate, explicit cargo overrides. This task adds no new SQL, so no `.sqlx` regen is
> needed; `get_tasks` now calls only the (already-compiled) 401 query.
