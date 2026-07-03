---
id: "502"
phase: 5
title: Node digest builder — emit NodeMessage::Digest of swarm-linked tasks each sync cycle
status: ready
depends_on: ["501"]
parallel: false
conflicts_with: []
files:
  - crates/db/src/models/task/queries.rs
  - crates/services/src/services/hive_sync.rs
irreversible: false
scope_test: "crates/services/src/services/hive_sync.rs"
allowed_change: edit
covers_criteria: [SC5]
---
## Failing test (write first)
Two tests. **(1)** A `db`-crate query test for the new digest read, hermetic via `create_test_pool()`.
**(2)** A `services` test driving the new `sync_digest` helper over an enqueued fixture and asserting a
`NodeMessage::Digest` carrying one entry per swarm-linked task (entity_id = local task id, version =
`remote_version`) is pushed to the command channel — and NO op/ack side effect (digest is read-only).

### (1) In `crates/db/src/models/task/queries.rs` `#[cfg(test)] mod tests` (or the crate's task test mod):
```rust
#[tokio::test]
async fn find_digest_entries_returns_only_swarm_linked_tasks_with_version() {
    let (pool, _tmp) = db::test_utils::create_test_pool().await;
    // seed a project; one task WITH shared_task_id set (+ remote_version = 3), one task WITHOUT.
    // (use Task::create then set shared_task_id/remote_version via the existing sync update path, or a
    //  direct UPDATE in the test — whichever the sibling tests in this file already do.)
    let entries = Task::find_digest_entries(&pool).await.unwrap();
    assert_eq!(entries.len(), 1, "only the swarm-linked (shared_task_id IS NOT NULL) task is in the digest");
    assert_eq!(entries[0].version, 3, "version is the task's remote_version");
    // entries[0].entity_id == the linked task's LOCAL id (the id-bridge key = hive source_task_id).
}
```

### (2) In `crates/services/src/services/hive_sync.rs` `#[cfg(test)] mod tests`:
```rust
#[tokio::test]
async fn sync_digest_sends_one_entry_per_swarm_linked_task() {
    let (pool, _tmp) = db::test_utils::create_test_pool().await;
    // seed one swarm-linked task (shared_task_id set, remote_version = 1) as in test (1).
    let (command_tx, mut command_rx) = tokio::sync::mpsc::channel(8);
    let service = HiveSyncService::new(pool.clone(), command_tx, HiveSyncConfig::default());
    service.sync_digest().await.unwrap(); // the new helper (see Change)

    let msg = command_rx.try_recv().expect("a Digest was sent");
    match msg {
        NodeMessage::Digest { entries } => {
            assert_eq!(entries.len(), 1);
            assert_eq!(entries[0].entity_type, "task");
        }
        other => panic!("expected Digest, got {other:?}"),
    }
}
```
> The digest is READ-ONLY: it never marks anything synced/acked and never enqueues an op. It is the
> divergence-detection probe; the heal is 503 (hive reply) + 504 (node acts). "On reconnect" is folded
> into "the first `sync_once` after (re)connect" (the 30s loop drives `sync_once` each tick; a reconnect
> re-establishes the command channel and the next tick emits a fresh digest). There is no distinct
> reconnect callback to anchor — do NOT invent one (STOP trigger below).

## Change

### 1. `crates/db/src/models/task/queries.rs` — add `find_digest_entries`
- **File:** `crates/db/src/models/task/queries.rs`
- **Anchor:** the `impl Task` query block (the `find_*` methods; `remote_version` is selected at @34, the
  row maps it at @121 — the column is already in the SELECT vocabulary here). Add a NEW method beside the
  existing `find_*` reads.
- **Sibling read (rubric #9):** the existing `find_*` methods in THIS file are the pattern — `query!`
  (NOT `query_as!` into `Task`, since we return a lean projection), `&SqlitePool` arg, BLOB-UUID select
  casts (`id as "id!: Uuid"`), `remote_version as "remote_version!: i64"`. Mirror their cast style.
- **Before:** (no `find_digest_entries` method exists)
- **After:** add a lean projection + method (the WS `DigestEntry` lives in `services`, so `db` returns a
  small local row struct; `services` maps it):
```rust
/// A node-side anti-entropy digest row: the id-bridge key + the version the node believes the hive holds.
#[derive(Debug, Clone)]
pub struct TaskDigestRow {
    pub id: Uuid,            // local task id == the hive's shared_tasks.source_task_id (id bridge)
    pub remote_version: i64,
}

impl Task {
    /// All swarm-linked tasks (`shared_task_id IS NOT NULL`) with their `remote_version`, for the SC5
    /// anti-entropy digest. Read-only; ordered by `id` for a stable digest. NO `limit` cap — the digest
    /// MUST cover EVERY swarm-linked task in one shot so the hive can detect divergence on any task
    /// (a `limit` would silently truncate the digest and leave divergences undetected past the batch
    /// boundary with no cursor/pagination to advance). The node's swarm-linked task count is bounded by
    /// its local `tasks` table, so the unbounded read is acceptable. The `archived_at IS NULL` filter
    /// was REMOVED to align with the "all swarm-linked tasks" requirement: an archived task that still
    /// carries a `shared_task_id` is still part of the swarm link, and the hive must see it in the
    /// digest to detect if the hive lost it (hive-has/node-lacks divergence includes archived tasks).
    pub async fn find_digest_entries(
        pool: &SqlitePool,
    ) -> Result<Vec<TaskDigestRow>, sqlx::Error> {
        sqlx::query!(
            r#"SELECT id as "id!: Uuid", remote_version as "remote_version!: i64"
               FROM tasks
               WHERE shared_task_id IS NOT NULL
               ORDER BY id ASC"#,
        )
        .fetch_all(pool)
        .await
        .map(|rows| {
            rows.into_iter()
                .map(|r| TaskDigestRow { id: r.id, remote_version: r.remote_version })
                .collect()
        })
    }
}
```
> `shared_task_id IS NOT NULL` is the node-side id-bridge predicate (the task has been synced to the
> hive, so the hive SHOULD have a matching `shared_tasks` row keyed by `source_node_id + source_task_id =
> this node + id`). A node-linked task the hive lost is exactly the node-has/hive-lacks divergence TS4
> seeds. Use `id` (local) as `entity_id` — NOT `shared_task_id` — because the bridge key the hive
> compares against is `source_task_id` = the node's LOCAL id (`shared_tasks` migration
> `20260105120000`; comment "Original local task ID from source node").

### 2. `crates/services/src/services/hive_sync.rs` — build + send the digest each cycle
- **File:** `crates/services/src/services/hive_sync.rs`
- **Anchor:** `sync_once` (tail — 107 added a `self.sync_outbox()` call there; place the digest call
  AFTER it), and the `use super::hive_client::{…}` import block. `DigestEntry` is the WS
  `crate::services::hive_client::DigestEntry` from 501.
- **Sibling read (rubric #9):** 107's `sync_outbox` helper in THIS file is the exact template — read from
  `self.pool`, build a `NodeMessage`, push via `self.command_tx.send(...)`, best-effort, no ack. Mirror it.
- **Before (import block — add `DigestEntry`; 107 already added `OutboxOp` here):**
```rust
use super::hive_client::{
    AttemptSyncMessage, ExecutionSyncMessage, LocalProjectSyncInfo, LogsBatchMessage, NodeMessage,
    OutboxOp, ProjectsSyncMessage, SyncLogEntry, TaskOutputType, TaskSyncMessage,
};
```
- **After:** add `DigestEntry`:
```rust
use super::hive_client::{
    AttemptSyncMessage, DigestEntry, ExecutionSyncMessage, LocalProjectSyncInfo, LogsBatchMessage,
    NodeMessage, OutboxOp, ProjectsSyncMessage, SyncLogEntry, TaskOutputType, TaskSyncMessage,
};
```
- **Before (the `sync_once` tail that 107 left — the `sync_outbox` call before `Ok(())`):**
```rust
        if let Err(e) = self.sync_outbox().await {
            warn!(error = ?e, "Failed to drain node_outbox op-log");
        }

        Ok(())
    }
```
- **After:** add the digest emission after the outbox drain:
```rust
        if let Err(e) = self.sync_outbox().await {
            warn!(error = ?e, "Failed to drain node_outbox op-log");
        }

        // Emit the anti-entropy digest (SC5): a per-entity version snapshot the hive compares against
        // its own state to detect silent divergence the ack cursor misses, then replies DigestResult
        // (503). Read-only — does NOT mark anything synced/acked. Runs every cycle (so "on reconnect" =
        // the first cycle after the channel is re-established); the heal is applied by 504.
        if let Err(e) = self.sync_digest().await {
            warn!(error = ?e, "Failed to send anti-entropy digest");
        }

        Ok(())
    }

    /// Build and push the SC5 anti-entropy digest: one `DigestEntry` per swarm-linked task
    /// (`shared_task_id IS NOT NULL`) carrying its `remote_version`. Best-effort, read-only; an empty
    /// set sends nothing. Does NOT advance any cursor (it is divergence DETECTION, not sync).
    async fn sync_digest(&self) -> Result<(), HiveSyncError> {
        use db::models::task::Task;
        let rows = Task::find_digest_entries(&self.pool).await?;
        if rows.is_empty() {
            return Ok(());
        }
        let entries: Vec<DigestEntry> = rows
            .into_iter()
            .map(|r| DigestEntry {
                entity_type: "task".to_string(),
                entity_id: r.id,
                version: r.remote_version,
            })
            .collect();
        self.command_tx
            .send(NodeMessage::Digest { entries })
            .await
            .map_err(|e| HiveSyncError::Send(e.to_string()))?;
        Ok(())
    }
```
> `HiveSyncError::Send` already exists (107 used it). If the `Task` import path differs, align to this
> crate's existing `use db::models::task::Task;` convention (the legacy `sync_tasks` helper already
> imports `Task`). The digest reads ALL swarm-linked tasks unbounded (no `limit`/`max_tasks_per_batch`)
> — see the `find_digest_entries` rationale above.

## Allowed moves
ONLY: add `find_digest_entries` + the `TaskDigestRow` projection to `task/queries.rs`; add `DigestEntry`
to the `hive_sync.rs` import; call `self.sync_digest()` at the end of `sync_once` (AFTER 107's
`sync_outbox` call); and add the private `sync_digest` helper. Do NOT mark any task synced/acked, do NOT
enqueue an outbox op, do NOT touch the WS enum (501 owns it), do NOT change `sync_once`'s signature, do
NOT add a config field, do NOT build a per-attempt/exec/log digest (tracer scope: tasks only).

## STOP triggers
- `Task::remote_version`/`shared_task_id` columns are absent or renamed → STOP; the digest version source
  is `remote_version` (`task/mod.rs:51`) and the linked predicate is `shared_task_id IS NOT NULL`. Do not
  substitute another column.
- `query!` fails offline → export `DATABASE_URL=sqlite://<repo>/dev_assets/db.sqlite` against a migrated
  dev DB (Trap 2). Do NOT `cargo sqlx prepare` in this task (the regen is a `/wai:close` step).
- The WS `DigestEntry`/`NodeMessage::Digest` are absent → 501 must be `passed` (depends_on: 501); without
  it `cargo check -p services` will not compile.
- A reconnect callback is sought to send the digest "on reconnect" → there is none; the 30s `sync_once`
  loop is the only driver. Fold reconnect into "first cycle after reconnect" (this task already does);
  do NOT add a new reconnect hook (out of scope, would expand `files:`).
- The digest tries to enqueue an op or advance a cursor → BUG: the digest is read-only DETECTION. The
  heal happens in 503 (hive reply) + 504 (node re-stream/pull). Keep `sync_digest` side-effect-free.

## Done when
`WAI_TYPECHECK_CMD="cargo check -p services" WAI_TEST_CMD="cargo test -p db find_digest_entries && cargo test -p services sync_digest" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-hive-redesign 502` exits 0
(export `DATABASE_URL=sqlite://<repo>/dev_assets/db.sqlite` migrated through 101 before running — Trap 2;
the `db` test validates the new `query!` against the live schema, the `services` test is hermetic.)
