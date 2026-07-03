---
id: "403"
phase: 4
title: Dirty-guard in upsert_remote_task — inbound never clobbers an unacked local edit
status: ready
depends_on: ["104"]
parallel: false
conflicts_with: ["401", "402"]
files:
  - crates/db/src/models/node_outbox.rs
  - crates/db/src/models/task/sync.rs
irreversible: false
scope_test: "crates/db/src/models/task/sync.rs"
allowed_change: edit
covers_criteria: [SC7]
covers_tests: []
---
## Failing test (write first)
The bug (ADR-0007 §One conflict policy): the inbound apply gate is `remote_version`-only
(`task/sync.rs:300` `WHERE excluded.remote_version > tasks.remote_version OR …`) and the LOCAL update
path bumps NO version (`task/queries.rs:305-307`), so an arriving hive update silently overwrites a
concurrent local edit. The fix: a **dirty-guard** — before applying, if the local task (resolved by
`shared_task_id`) has an UNACKED outbound op in `node_outbox`, the inbound apply is SKIPPED (the local
edit travels up the ordered outbox first; ADR-0008). Entity-level (skip the whole apply) is strictly
more conservative than field-level and satisfies TS5's "concurrent local edit not clobbered".

Add to the `#[cfg(test)] mod tests` block at the bottom of `crates/db/src/models/task/sync.rs`:

```rust
    #[tokio::test]
    async fn upsert_remote_task_skips_when_local_op_unacked() {
        use crate::models::node_outbox::{NewOutboxOp, OutboxRepository};
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

        // A hive-linked local task at version 1, title "remote-title".
        let local_id = Uuid::new_v4();
        let shared_id = Uuid::new_v4();
        Task::upsert_remote_task(
            &pool, local_id, project_id, shared_id, "remote-title".into(), None,
            TaskStatus::InReview, None, None, None, 1, None, None,
        ).await.unwrap();
        // Resolve the row actually written (upsert may have generated its own local id).
        let row = Task::find_by_shared_task_id(&pool, shared_id).await.unwrap().unwrap();

        // The user edits locally → an UNACKED outbox op for this task exists.
        OutboxRepository::enqueue_op(&pool, NewOutboxOp {
            op_type: "task.upsert".into(), entity_type: "task".into(), entity_id: row.id,
            payload: serde_json::json!({}), idempotency_key: format!("task:{}:1", row.id),
            fencing_token: None,
        }).await.unwrap();

        // Inbound update arrives (version 2, new title). Dirty-guard must SKIP it.
        let returned = Task::upsert_remote_task(
            &pool, Uuid::new_v4(), project_id, shared_id, "HIVE-CLOBBER".into(), None,
            TaskStatus::Done, None, None, None, 2, None, None,
        ).await.unwrap();

        let after = Task::find_by_shared_task_id(&pool, shared_id).await.unwrap().unwrap();
        assert_eq!(after.title, "remote-title", "local edit not clobbered (apply skipped)");
        assert_eq!(after.remote_version, 1, "version not advanced while dirty");
        assert_eq!(returned.title, "remote-title", "returns the retained local row");
    }

    #[tokio::test]
    async fn upsert_remote_task_applies_when_clean() {
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

        let shared_id = Uuid::new_v4();
        Task::upsert_remote_task(
            &pool, Uuid::new_v4(), project_id, shared_id, "v1".into(), None,
            TaskStatus::InReview, None, None, None, 1, None, None,
        ).await.unwrap();
        // No unacked op → inbound update applies normally (existing remote_version gate).
        Task::upsert_remote_task(
            &pool, Uuid::new_v4(), project_id, shared_id, "v2".into(), None,
            TaskStatus::InReview, None, None, None, 2, None, None,
        ).await.unwrap();

        let after = Task::find_by_shared_task_id(&pool, shared_id).await.unwrap().unwrap();
        assert_eq!(after.title, "v2", "clean path still applies");
        assert_eq!(after.remote_version, 2);
    }
```
> Depends on task **104** (`OutboxRepository`, `NewOutboxOp`) and task **101** (the `node_outbox`
> migration applied to the dev DB). If 104's `NewOutboxOp` field names differ, mirror its 108 test usage.

## Change

### 1. `crates/db/src/models/node_outbox.rs` — add an unacked-by-entity probe
- **File:** `crates/db/src/models/node_outbox.rs` (created by task 104)
- **Sibling read (rubric #9):** `OutboxRepository::peek_unacked` (104) filters `WHERE acked_at IS NULL`
  (the `node_outbox.acked_at` column from migration 101 is NULL until a durable hive ack; the partial
  index `idx_node_outbox_unacked_seq` is `WHERE acked_at IS NULL`). The new probe reuses that EXACT
  unacked predicate, scoped to one `entity_id` (BLOB-UUID, same bind idiom as `enqueue_op`).
- **Anchor:** `impl OutboxRepository`, immediately after `peek_unacked`.
- **Before (the closing `}` of `peek_unacked`):**
```rust
    }
```
- **After:**
```rust
    }

    /// True if `entity_id` has at least one UNACKED outbound op (ADR-0007 dirty-guard).
    ///
    /// Used by the inbound apply path to skip applying a hive update while the node still has a
    /// pending, not-yet durably-acked outbound op for that entity. Reuses `peek_unacked`'s exact
    /// "unacked" predicate (`acked_at IS NULL`, the same one the partial index covers), scoped to one
    /// entity. `entity_id` is a BLOB UUID (same bind idiom as `enqueue_op`).
    pub async fn has_unacked_for_entity(
        pool: &sqlx::SqlitePool,
        entity_id: uuid::Uuid,
    ) -> Result<bool, sqlx::Error> {
        let exists = sqlx::query_scalar!(
            r#"SELECT EXISTS(
                   SELECT 1 FROM node_outbox WHERE entity_id = ? AND acked_at IS NULL
               ) as "exists!: i64""#,
            entity_id
        )
        .fetch_one(pool)
        .await?;
        Ok(exists != 0)
    }
```

### 2. `crates/db/src/models/task/sync.rs` — guard `upsert_remote_task`
- **File:** `crates/db/src/models/task/sync.rs`
- **Anchor:** `upsert_remote_task` (L253), the function body's FIRST line `let now = Utc::now();` (L266,
  immediately after the signature's `) -> Result<Self, sqlx::Error> {`). Insert the guard BEFORE the
  upsert SQL so BOTH inbound legs (WS `processor.rs` and reconcile `node_runner.rs`) funnel through this
  ONE guard — "applied identically on both legs" by construction.
- **Before:**
```rust
    ) -> Result<Self, sqlx::Error> {
        let now = Utc::now();
        let result = sqlx::query_as!(
```
- **After:**
```rust
    ) -> Result<Self, sqlx::Error> {
        // Dirty-guard (ADR-0007 one conflict policy): if the local task linked to this shared_task_id
        // has an UNACKED outbound op, an inbound update must NOT overwrite it — the local edit travels
        // up the ordered outbox first (ADR-0008). Skip the apply and return the retained local row.
        if let Some(existing) = Task::find_by_shared_task_id(pool, shared_task_id).await?
            && crate::models::node_outbox::OutboxRepository::has_unacked_for_entity(pool, existing.id)
                .await?
        {
            return Ok(existing);
        }
        let now = Utc::now();
        let result = sqlx::query_as!(
```

## Allowed moves
ONLY: add `has_unacked_for_entity` to `node_outbox.rs`; add the dirty-guard early-return at the top of
`upsert_remote_task`; add the two `#[tokio::test]`s. Do NOT change the existing `remote_version` gate SQL
(it stays as the version ordering for the CLEAN path — the guard sits IN FRONT of it). Do NOT add a
field-level dirty model (entity-level is the ratified granularity — see ledger). Do NOT touch the WS or
reconcile call sites (403's guard lives in the shared apply fn; that is the point).

## STOP triggers
- `node_outbox.rs` does not exist / `OutboxRepository` absent → task 104 not `passed`; depends_on: 104.
  STOP.
- 104/101's unacked column is NOT `acked_at` (e.g. a separate cursor table) → it is `acked_at IS NULL`
  per migration 101 (`acked_at TEXT … NULL until ack`) and 104's `peek_unacked`; if 101/104 changed the
  schema, MATCH their actual predicate — do NOT invent a column. Read `peek_unacked` first.
- `Task::find_by_shared_task_id` is not `(pool, shared_task_id) -> Result<Option<Self>>` → it is (used at
  sync.rs:330 and processor.rs:434); if its signature changed, STOP.
- Letrust `let … && …` chains are unavailable (edition < 2024) → the workspace is edition 2024
  (CLAUDE.md); if a clippy/borrow error appears, split into nested `if let { if … }`.
- The new `query_scalar!` fails offline → `DATABASE_URL` not exported against a dev DB migrated through
  task 101's `node_outbox` migration (Trap 2). Export it; never run `cargo sqlx prepare` here.

## Done when
`WAI_TYPECHECK_CMD="cargo check -p db" WAI_TEST_CMD="cargo test -p db upsert_remote_task_" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-hive-redesign 403` exits 0
(export `DATABASE_URL=sqlite://<repo>/dev_assets/db.sqlite` migrated through task 101 before running — Trap 2.)
