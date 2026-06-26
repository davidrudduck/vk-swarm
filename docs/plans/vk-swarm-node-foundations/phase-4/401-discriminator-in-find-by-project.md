---
id: "401"
phase: 4
title: Add node-local visibility discriminator to find_by_project_id_with_attempt_status
status: ready
depends_on: []
parallel: false
conflicts_with: []
files:
  - crates/db/src/models/task/queries.rs
  - crates/db/tests/task_visibility_discriminator.rs
irreversible: false
scope_test: "crates/db/tests/task_visibility_discriminator.rs"
allowed_change: edit
covers_criteria: [SC5]
---
## Failing test (write first)

Create `crates/db/tests/task_visibility_discriminator.rs` (new integration test). Use the shared
template pool per CLAUDE.md.

```rust
//! Verifies the node-local visibility discriminator on
//! `Task::find_by_project_id_with_attempt_status`: a task is node-visible iff it was
//! created/owned on this node (no remote-mirror stamp) OR this node has a local task_attempt
//! for it. Mirrored-remote rows with no local attempt must be hidden.

use chrono::Utc;
use db::models::{
    project::{CreateProject, Project},
    task::{CreateTask, Task, TaskStatus},
};
use db::test_utils::create_test_pool;
use uuid::Uuid;

async fn make_project(pool: &sqlx::SqlitePool) -> Project {
    let id = Uuid::new_v4();
    let data = CreateProject {
        name: "Visibility Test".to_string(),
        git_repo_path: format!("/tmp/vis-{id}"),
        use_existing_repo: true,
        clone_url: None,
        setup_script: None,
        dev_script: None,
        cleanup_script: None,
        copy_files: None,
    };
    Project::create(pool, &data, id).await.expect("project")
}

async fn insert_local_attempt(pool: &sqlx::SqlitePool, task_id: Uuid) {
    sqlx::query(
        r#"INSERT INTO task_attempts (id, task_id, executor, branch, target_branch)
           VALUES ($1, $2, 'CLAUDE_CODE', 'b', 'main')"#,
    )
    .bind(Uuid::new_v4())
    .bind(task_id)
    .execute(pool)
    .await
    .expect("attempt");
}

/// Insert a row exactly as the inbound remote-mirror writer leaves it: shared_task_id set
/// AND remote_last_synced_at stamped. (We do NOT call the sync writer — off-limits per SC5d —
/// we reproduce the row shape it produces.)
async fn insert_mirrored_remote_task(pool: &sqlx::SqlitePool, project_id: Uuid) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        r#"INSERT INTO tasks (id, project_id, title, status, shared_task_id,
                              remote_version, remote_last_synced_at)
           VALUES ($1, $2, 'mirrored', 'todo', $3, 5, $4)"#,
    )
    .bind(id)
    .bind(project_id)
    .bind(Uuid::new_v4())
    .bind(Utc::now())
    .execute(pool)
    .await
    .expect("mirrored insert");
    id
}

#[tokio::test]
async fn locally_created_task_is_visible() {
    let (pool, _tmp) = create_test_pool().await;
    let project = make_project(&pool).await;
    let local = CreateTask::from_title_description(project.id, "local".into(), None);
    let local_id = Uuid::new_v4();
    Task::create(&pool, &local, local_id).await.expect("local task");

    let rows = Task::find_by_project_id_with_attempt_status(&pool, project.id, false)
        .await
        .expect("query");
    assert!(rows.iter().any(|r| r.task.id == local_id), "local task must be visible");
}

#[tokio::test]
async fn hive_assigned_task_with_local_attempt_is_visible() {
    let (pool, _tmp) = create_test_pool().await;
    let project = make_project(&pool).await;
    // Assigned task: shared_task_id set, and this node is running it (local attempt exists).
    let assigned = CreateTask::from_shared_task(
        project.id, "assigned".into(), None, TaskStatus::InProgress, Uuid::new_v4(),
    );
    let assigned_id = Uuid::new_v4();
    Task::create(&pool, &assigned, assigned_id).await.expect("assigned task");
    insert_local_attempt(&pool, assigned_id).await;

    let rows = Task::find_by_project_id_with_attempt_status(&pool, project.id, false)
        .await
        .expect("query");
    assert!(
        rows.iter().any(|r| r.task.id == assigned_id),
        "hive-assigned task WITH a local attempt must be visible"
    );
}

#[tokio::test]
async fn remote_mirrored_task_without_local_attempt_is_hidden() {
    let (pool, _tmp) = create_test_pool().await;
    let project = make_project(&pool).await;
    let mirrored_id = insert_mirrored_remote_task(&pool, project.id).await;

    let rows = Task::find_by_project_id_with_attempt_status(&pool, project.id, false)
        .await
        .expect("query");
    assert!(
        !rows.iter().any(|r| r.task.id == mirrored_id),
        "remote-mirrored task with NO local attempt must NOT be visible"
    );
}

#[tokio::test]
async fn locally_created_then_shared_task_is_visible() {
    // The trap: a task created here, then pushed to the hive, has shared_task_id SET but
    // remote_last_synced_at still NULL (the publisher only stamps shared_task_id). With no
    // attempt yet it must STILL be visible (created-here). A naive `shared_task_id IS NULL`
    // predicate would wrongly hide it.
    let (pool, _tmp) = create_test_pool().await;
    let project = make_project(&pool).await;
    let task_id = Uuid::new_v4();
    let data = CreateTask::from_title_description(project.id, "shared-local".into(), None);
    Task::create(&pool, &data, task_id).await.expect("task");
    // Simulate publisher stamping shared_task_id only (no remote_last_synced_at).
    Task::set_shared_task_id(&pool, task_id, Some(Uuid::new_v4()))
        .await
        .expect("stamp shared_task_id");

    let rows = Task::find_by_project_id_with_attempt_status(&pool, project.id, false)
        .await
        .expect("query");
    assert!(
        rows.iter().any(|r| r.task.id == task_id),
        "locally-created-then-shared task (no attempt) must be visible"
    );
}
```

## Change

- **File:** `crates/db/src/models/task/queries.rs`
- **Anchor:** the `WHERE` clause of the `query!` in `find_by_project_id_with_attempt_status`
  (function starts at L15; the WHERE/ORDER BY block is L92–94).
- **Before:**
```text
FROM tasks t
LEFT JOIN projects p ON p.id = t.project_id
WHERE t.project_id = $1
  AND (t.archived_at IS NULL OR $2)
ORDER BY COALESCE(t.activity_at, t.created_at) DESC"#,
```

- **After:**
```bash
FROM tasks t
LEFT JOIN projects p ON p.id = t.project_id
WHERE t.project_id = $1
  AND (t.archived_at IS NULL OR $2)
  -- Node-local visibility (ADR-0002): show a task iff it was created/owned on THIS node
  -- (the inbound mirror always stamps remote_last_synced_at; local-origin rows never do,
  -- even after the publisher sets shared_task_id) OR this node is/was running it (a local
  -- task_attempt row exists; remote attempts are never persisted locally).
  AND (
    t.remote_last_synced_at IS NULL
    OR EXISTS (SELECT 1 FROM task_attempts ta WHERE ta.task_id = t.id)
  )
ORDER BY COALESCE(t.activity_at, t.created_at) DESC"#,
```

- **File:** `crates/db/tests/task_visibility_discriminator.rs` — create with the test body above.

## Allowed moves

- In `queries.rs`: edit ONLY the `WHERE … ORDER BY` lines of this one `query!` string (add the
  visibility predicate). Do NOT touch the SELECT columns, the binds (`$1`,`$2`), the row mapping
  (L101+), the function signature, or any other function.
- Create the new test file with exactly the cases above.
- Do NOT add/alter columns, do NOT touch `upsert_remote_task`, the publisher, the WS runner, or
  any `remote_*`/`shared_task_id` writer (SC5d — read-layer only).

## Sibling alignment

The naive sibling predicate is the merge/dedup logic in `get_tasks`
(`crates/server/src/routes/tasks/handlers/core.rs`), which keys local-vs-remote on
`shared_task_id`. Read its keying before writing this predicate and note the divergence: the
read-layer discriminator here deliberately does NOT use `shared_task_id IS NULL` (a local task
keeps its row but gains `shared_task_id` after publishing); it uses `remote_last_synced_at IS NULL`,
the only column the inbound mirror (`upsert_remote_task` in `task/sync.rs`, which binds
`remote_last_synced_at = now`) sets that the local-create/publish path never does. (`remote_version`
is unusable: the tasks schema defaults it to `1`, not `0`.) That sibling is removed in task 402;
this predicate replaces it.

## STOP triggers

- The `WHERE … ORDER BY` text at L92–94 does not match Before exactly (someone re-touched the query).
- `task_attempts` has no `task_id` column, or `tasks` has no `remote_last_synced_at` column
  (schema drift) — re-confirm the discriminator against the live schema before proceeding.
- Any test case fails for a reason other than the predicate (e.g. `create_test_pool` signature
  changed) — halt and re-verify the helper API.
- The change would require editing any file other than the two listed.

## Done when

`WAI_TYPECHECK_CMD="cargo check -p db" WAI_TEST_CMD="cargo test -p db --test task_visibility_discriminator" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-node-foundations 401` exits 0

> SQLx note (Trap 2): this task CHANGES the query's WHERE clause — even though it references only
> existing columns, the modified query text is a NEW entry from sqlx's offline-cache perspective (the
> cache keys on the query string's hash), so the stale `.sqlx` will NOT contain it and an offline build
> fails. **Precondition:** export `DATABASE_URL=sqlite://<repo>/dev_assets/db.sqlite` (migrated) so
> `query_as!` checks the live schema. Do NOT `cargo sqlx prepare` in this task (it churns the tracked
> `.sqlx` cache the gate rejects; regen is a `/wai:close` step).
