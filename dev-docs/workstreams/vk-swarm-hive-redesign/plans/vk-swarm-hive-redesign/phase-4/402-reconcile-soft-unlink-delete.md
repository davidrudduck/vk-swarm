---
id: "402"
phase: 4
title: One delete semantic — both inbound legs soft-unlink via a single shared helper
status: ready
depends_on: []
parallel: false
conflicts_with: ["401", "403", "404"]
files:
  - crates/db/src/models/task/sync.rs
  - crates/services/src/services/node_runner.rs
  - crates/services/src/services/share/processor.rs
irreversible: false
scope_test: "crates/db/src/models/task/sync.rs"
allowed_change: edit
covers_criteria: [SC7]
covers_tests: []
---
## Failing test (write first)
Two bugs (ADR-0007 + a LATENT prod bug found at authoring — see ledger):
1. The connect-time reconcile leg **hard-deletes** the row
   (`Task::delete_by_shared_task_id` + `delete_stale_shared_tasks`, `node_runner.rs:1034,1039`).
2. The WS leg's existing "soft-unlink" (`processor.rs:437` `set_shared_task_id(.., None)`) is a **no-op
   for a LINKED row**: its `WHERE … AND (shared_task_id IS NULL OR shared_task_id = $2) …` with `$2 =
   NULL` reduces (SQLite three-valued logic, `= NULL` is never true) to `shared_task_id IS NULL`, so a
   row that IS linked (`shared_task_id = S`) matches 0 rows and is never cleared. Verified empirically:
   `UPDATE t SET s=NULL WHERE id=1 AND (s IS NULL OR s=NULL) …` leaves `s='S'`.

So today neither leg reliably soft-unlinks. The fix routes **BOTH legs through ONE working,
executor-generic `unlink_by_shared_task_id`** — "applied identically on both legs" by construction
(ADR-0007 §2). Test the helper directly in `sync.rs` (hermetic; reuses the module's existing
`setup_test_pool()` + `CreateProject`/`CreateTask`).

Add to the `#[cfg(test)] mod tests` block at the bottom of `crates/db/src/models/task/sync.rs` (it
already imports `CreateProject`, `Project`, `CreateTask`, `setup_test_pool`):

```rust
    #[tokio::test]
    async fn unlink_by_shared_task_id_keeps_local_row() {
        let (pool, _temp_dir) = setup_test_pool().await;
        let project_id = Uuid::new_v4();
        let project_data = CreateProject {
            name: "Test Project".to_string(),
            git_repo_path: format!("/tmp/test-repo-{}", project_id),
            use_existing_repo: true,
            clone_url: None,
            setup_script: None,
            dev_script: None,
            cleanup_script: None,
            copy_files: None,
        };
        Project::create(&pool, &project_data, project_id).await.unwrap();

        let local_id = Uuid::new_v4();
        let shared_id = Uuid::new_v4();
        Task::create(&pool, &CreateTask::from_title_description(project_id, "t".into(), None), local_id)
            .await.unwrap();
        Task::set_shared_task_id(&pool, local_id, Some(shared_id)).await.unwrap();

        // Reconcile leg: hive deleted this shared task.
        let n = Task::unlink_by_shared_task_id(&pool, shared_id).await.unwrap();
        assert_eq!(n, 1, "one row unlinked");

        let still = Task::find_by_id(&pool, local_id).await.unwrap();
        assert!(still.is_some(), "local row is RETAINED (soft-unlink, not hard-delete)");
        assert!(still.unwrap().shared_task_id.is_none(), "shared_task_id cleared (tombstone)");
    }

    #[tokio::test]
    async fn unlink_stale_shared_tasks_keeps_local_rows() {
        let (pool, _temp_dir) = setup_test_pool().await;
        let project_id = Uuid::new_v4();
        let project_data = CreateProject {
            name: "Test Project".to_string(),
            git_repo_path: format!("/tmp/test-repo-{}", project_id),
            use_existing_repo: true,
            clone_url: None,
            setup_script: None,
            dev_script: None,
            cleanup_script: None,
            copy_files: None,
        };
        Project::create(&pool, &project_data, project_id).await.unwrap();

        let kept = Uuid::new_v4();
        let stale = Uuid::new_v4();
        let kept_shared = Uuid::new_v4();
        let stale_shared = Uuid::new_v4();
        for (lid, sid) in [(kept, kept_shared), (stale, stale_shared)] {
            Task::create(&pool, &CreateTask::from_title_description(project_id, "t".into(), None), lid)
                .await.unwrap();
            Task::set_shared_task_id(&pool, lid, Some(sid)).await.unwrap();
        }

        // Active set contains only `kept_shared`; `stale_shared` is no longer present on hive.
        let n = Task::unlink_stale_shared_tasks(&pool, project_id, &[kept_shared]).await.unwrap();
        assert_eq!(n, 1, "only the stale row is unlinked");

        assert!(Task::find_by_id(&pool, stale).await.unwrap().unwrap().shared_task_id.is_none());
        assert_eq!(
            Task::find_by_id(&pool, kept).await.unwrap().unwrap().shared_task_id,
            Some(kept_shared),
            "active task stays linked",
        );
        assert!(Task::find_by_id(&pool, stale).await.unwrap().is_some(), "stale local row retained");
    }

    #[tokio::test]
    async fn unlink_stale_shared_tasks_clears_all_when_active_set_is_empty() {
        // An empty active set means the hive dropped every shared task in this project — ALL linked
        // tasks must be unlinked (not skipped, which would leave stale links).
        let (pool, _temp_dir) = setup_test_pool().await;
        let project_id = Uuid::new_v4();
        let project_data = CreateProject {
            name: "Test Project".to_string(),
            git_repo_path: format!("/tmp/test-repo-{}", project_id),
            use_existing_repo: true,
            clone_url: None,
            setup_script: None,
            dev_script: None,
            cleanup_script: None,
            copy_files: None,
        };
        Project::create(&pool, &project_data, project_id).await.unwrap();

        let stale = Uuid::new_v4();
        let stale_shared = Uuid::new_v4();
        Task::create(&pool, &CreateTask::from_title_description(project_id, "t".into(), None), stale)
            .await.unwrap();
        Task::set_shared_task_id(&pool, stale, Some(stale_shared)).await.unwrap();

        // Empty active set → every linked task in the project is unlinked.
        let n = Task::unlink_stale_shared_tasks(&pool, project_id, &[]).await.unwrap();
        assert_eq!(n, 1, "the linked row is unlinked even when the active set is empty");
        assert!(Task::find_by_id(&pool, stale).await.unwrap().unwrap().shared_task_id.is_none(),
            "stale link cleared on empty active set");
    }
```
> `Task::find_by_id` lives in `task/queries.rs:166` (same `impl Task`, signature `(pool, id)`). The test
> block already has all needed imports — add NO new `use` lines.

## Change

### 1. `crates/db/src/models/task/sync.rs` — add two soft-unlink helpers
- **File:** `crates/db/src/models/task/sync.rs`
- **Anchor:** the existing hard-delete helpers `delete_by_shared_task_id` (~L367) and
  `delete_stale_shared_tasks` (~L385). **Do NOT remove them** — this task only ADDS the soft-unlink
  variants; the reconcile leg switches to them in change #2. Place the two new fns immediately AFTER
  `delete_stale_shared_tasks`'s closing `}` (~L416), before the `clear_orphaned_shared_task_ids` doc
  comment. The new SQL mirrors the existing `clear_orphaned_shared_task_ids` pattern in the SAME file
  (`SET shared_task_id = NULL, updated_at = CURRENT_TIMESTAMP`).
- **Before (the boundary right after `delete_stale_shared_tasks`'s closing brace):**
```rust
        let result = query_builder.execute(pool).await?;
        Ok(result.rows_affected())
    }

    /// Clear shared_task_id for orphaned tasks.
```
- **After:**
```rust
        let result = query_builder.execute(pool).await?;
        Ok(result.rows_affected())
    }

    /// Soft-unlink a task from a deleted hive shared task (ADR-0007 one-delete semantic).
    ///
    /// Clears `shared_task_id` (tombstone) but RETAINS the local row and its `task_attempt`/run
    /// artifacts — the hub owns the board, the node never loses local work it ran. This is the
    /// SINGLE soft-unlink used by BOTH inbound legs — the WS `task.deleted` path (`processor.rs`, via a
    /// `&mut Transaction`) AND the cold-start/gap-fill reconcile (`node_runner.rs`, via a `&SqlitePool`) —
    /// so a hive soft-delete yields ONE node outcome regardless of which leg applies it. Executor-generic
    /// (mirrors `set_shared_task_id`) so the tx-based caller can pass `tx.as_mut()`.
    /// Returns the number of rows unlinked.
    pub async fn unlink_by_shared_task_id<'e, E>(
        executor: E,
        shared_task_id: Uuid,
    ) -> Result<u64, sqlx::Error>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        let result = sqlx::query!(
            r#"UPDATE tasks
               SET shared_task_id = NULL, updated_at = CURRENT_TIMESTAMP
               WHERE shared_task_id = ?"#,
            shared_task_id
        )
        .execute(executor)
        .await?;
        Ok(result.rows_affected())
    }

    /// Soft-unlink shared tasks no longer present in the reconcile snapshot (ADR-0007).
    ///
    /// The stale-sweep analogue of `unlink_by_shared_task_id`: any task in this project whose
    /// `shared_task_id` is NOT in the active set is unlinked (cleared) rather than hard-deleted,
    /// preserving local attempts. Only touches rows that HAVE a `shared_task_id` (hive-synced),
    /// leaving locally-created tasks untouched. Returns the number of rows unlinked.
    pub async fn unlink_stale_shared_tasks(
        pool: &SqlitePool,
        project_id: Uuid,
        active_shared_task_ids: &[Uuid],
    ) -> Result<u64, sqlx::Error> {
        // NOTE: do NOT short-circuit when `active_shared_task_ids` is empty — an empty snapshot means
        // EVERY linked task in this project is stale (the hive dropped them all), so all
        // `shared_task_id` values for linked tasks must be cleared. The early-return that was here
        // (`if active.is_empty() { return Ok(0); }`) skipped the unlink entirely, leaving stale links.
        let query = if active_shared_task_ids.is_empty() {
            // No active tasks → unlink ALL linked tasks in this project.
            r#"UPDATE tasks
               SET shared_task_id = NULL, updated_at = CURRENT_TIMESTAMP
               WHERE project_id = $1
               AND shared_task_id IS NOT NULL"#.to_string()
        } else {
            let placeholders: Vec<String> = active_shared_task_ids
                .iter()
                .enumerate()
                .map(|(i, _)| format!("${}", i + 2))
                .collect();
            let placeholders_str = placeholders.join(", ");
            format!(
                r#"UPDATE tasks
                   SET shared_task_id = NULL, updated_at = CURRENT_TIMESTAMP
                   WHERE project_id = $1
                   AND shared_task_id IS NOT NULL
                   AND shared_task_id NOT IN ({})"#,
                placeholders_str
            )
        };

        let mut query_builder = sqlx::query(&query).bind(project_id);
        for id in active_shared_task_ids {
            query_builder = query_builder.bind(id);
        }

        let result = query_builder.execute(pool).await?;
        Ok(result.rows_affected())
    }

    /// Clear shared_task_id for orphaned tasks.
```

### 2. `crates/services/src/services/node_runner.rs` — reconcile leg calls soft-unlink
- **File:** `crates/services/src/services/node_runner.rs`
- **Anchor:** `sync_remote_project_tasks` (~L986), the `// Handle deleted tasks` + `// Clean up stale
  shared tasks` blocks (~L1031-1041) that today call the hard-delete helpers.
- **Before:**
```rust
    // Handle deleted tasks
    for deleted_id in snapshot.deleted_task_ids {
        Task::delete_by_shared_task_id(pool, deleted_id).await?;
    }

    // Clean up stale shared tasks for this project
    if !active_task_ids.is_empty() {
        Task::delete_stale_shared_tasks(pool, local_project_id, &active_task_ids).await?;
    }
```
- **After:**
```rust
    // Handle deleted tasks — SOFT-UNLINK (ADR-0007 one delete semantic): clear shared_task_id,
    // retain the local row + its task_attempt. Identical outcome to the WS `task.deleted` leg.
    for deleted_id in snapshot.deleted_task_ids {
        Task::unlink_by_shared_task_id(pool, deleted_id).await?;
    }

    // Soft-unlink stale shared tasks for this project (no longer present in the snapshot).
    // NOTE: run UNCONDITIONALLY — an empty `active_task_ids` means the hive dropped every shared task
    // in this project, so `unlink_stale_shared_tasks` clears ALL linked tasks (no `NOT IN` filter).
    // The previous `if !active_task_ids.is_empty()` guard skipped the unlink on an empty snapshot,
    // leaving stale links behind.
    Task::unlink_stale_shared_tasks(pool, local_project_id, &active_task_ids).await?;
```

### 3. `crates/services/src/services/share/processor.rs` — WS leg uses the SAME helper
- **File:** `crates/services/src/services/share/processor.rs`
- **Anchor:** `process_task_deleted_event` (~L405), the find-then-clear block (~L433-444). The current
  code finds the local task by `shared_task_id` then calls `set_shared_task_id(.., None)` BY LOCAL ID —
  which (bug above) is a no-op for a linked row. Replace the whole find-then-clear dance with one
  `unlink_by_shared_task_id` call keyed on the HIVE id, identical to the reconcile leg.
- **Before:**
```rust
        // Find local task by shared_task_id and unlink it
        if let Some(existing) = Task::find_by_shared_task_id(tx.as_mut(), hive_task.id).await? {
            // Clear the shared_task_id to unlink from Hive
            // We don't delete the local task - just unlink it
            Task::set_shared_task_id(tx.as_mut(), existing.id, None).await?;

            debug!(
                local_task_id = %existing.id,
                shared_task_id = %hive_task.id,
                "Unlinked local task from deleted Hive task"
            );
        }
```
- **After:**
```rust
        // Soft-unlink via the SAME helper the reconcile leg uses (ADR-0007 one delete semantic):
        // clear shared_task_id, keep the local row + its task_attempt. Keyed on the hive id so both
        // legs are identical by construction (set_shared_task_id-by-id with NULL was a no-op for a
        // linked row — SQLite `= NULL` three-valued logic; see ledger).
        let unlinked = Task::unlink_by_shared_task_id(tx.as_mut(), hive_task.id).await?;
        if unlinked > 0 {
            debug!(
                shared_task_id = %hive_task.id,
                "Unlinked local task from deleted Hive task"
            );
        }
```
  > `tx.as_mut()` satisfies `E: Executor` (same as the surrounding `find_by_shared_task_id`/
  > `set_shared_task_id` calls). `find_by_shared_task_id` may become unused HERE — if the compiler warns
  > of an unused import, that import line is in `files:` scope to remove; do not touch other uses.

## Allowed moves
ONLY: add the two `unlink_*` helpers (executor-generic) in `sync.rs`; repoint the two reconcile call
sites in `node_runner.rs` from `delete_*` to `unlink_*`; replace the WS `process_task_deleted_event`
find-then-`set_shared_task_id` block with one `unlink_by_shared_task_id` call; add the
`#[tokio::test]`s. Do NOT remove the `delete_by_shared_task_id`/`delete_stale_shared_tasks` fns (task 405
owns dead-code deletion if they become unused). Do NOT bump `remote_version` or any other field in the
unlink SQL. Do NOT change the cursor-upsert tail of `process_event`.

## STOP triggers
- `delete_by_shared_task_id` / `delete_stale_shared_tasks` are NOT at the cited node_runner.rs anchors
  (snapshot fields `deleted_task_ids`/`tasks` differ) → re-locate via
  `grep -n "delete_by_shared_task_id\|delete_stale_shared_tasks" crates/services/src/services/node_runner.rs`;
  if the reconcile structure has changed, STOP.
- A grep shows `delete_by_shared_task_id` or `delete_stale_shared_tasks` has callers OUTSIDE
  `node_runner.rs` reconcile → STOP and report (this task must not change THEIR behavior; it only
  repoints the reconcile leg).
- `clear_orphaned_shared_task_ids` (the SQL sibling this mirrors) is absent → still author the helpers
  from the `SET shared_task_id = NULL` shape, but note the missing sibling in the ledger.
- The new `query!` fails to compile offline → `DATABASE_URL` not exported against the migrated dev DB
  (Trap 2). Export it; never run `cargo sqlx prepare` in this gated task.
- `process_task_deleted_event` no longer parses `hive_task` / its body changed shape → re-locate the
  find-then-clear block; if the WS delete handler was restructured, STOP.
- `tx.as_mut()` does not satisfy `unlink_by_shared_task_id`'s `E: Executor` bound → the helper MUST be
  executor-generic (this task makes it so); if you wrote it `pool`-only, fix the signature first.
- The empirical `= NULL` no-op claim does NOT reproduce (a re-read shows `set_shared_task_id(None)` DOES
  clear a linked row) → the WS leg already worked; then keep the reconcile repoint + helper but the
  processor.rs change is optional hygiene — record the finding and proceed.

## Done when
`WAI_TYPECHECK_CMD="cargo check -p db && cargo check -p services" WAI_TEST_CMD="cargo test -p db unlink_" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-hive-redesign 402` exits 0
(export `DATABASE_URL=sqlite://<repo>/dev_assets/db.sqlite` against the migrated dev DB before running — Trap 2. `cargo check -p services` covers the processor.rs WS-leg edit.)
