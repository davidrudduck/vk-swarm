---
id: "105"
phase: 1
title: Enqueue a task.upsert outbox op on Task::create / Task::update
status: ready
depends_on: ["104"]
parallel: false
conflicts_with: []
files:
  - crates/db/src/models/task/queries.rs
irreversible: false
scope_test: "crates/db/src/models/task/queries.rs"
allowed_change: edit
covers_criteria: [SC2]
covers_tests: [TS1]
---
## Failing test (write first)
In `crates/db/src/models/task/queries.rs`, add a `#[cfg(test)] mod tests` (or extend an existing one)
proving a `task.upsert` op is enqueued in causal order alongside the create/update. Hermetic
(`db::test_utils::create_test_pool()` — 101+104 already in the template DB).

```rust
#[cfg(test)]
mod outbox_enqueue_tests {
    use super::*;
    use crate::models::node_outbox::OutboxRepository;

    async fn seed_project(pool: &SqlitePool) -> Uuid {
        let pid = Uuid::new_v4();
        sqlx::query("INSERT INTO projects (id, name, git_repo_path) VALUES (?, 'p', '/tmp/p')")
            .bind(pid).execute(pool).await.unwrap();
        pid
    }

    #[tokio::test]
    async fn create_then_update_enqueues_two_ordered_task_upsert_ops() {
        let (pool, _tmp) = db::test_utils::create_test_pool().await;
        let project_id = seed_project(&pool).await;

        let task_id = Uuid::new_v4();
        let created = Task::create(
            &pool,
            &CreateTask { project_id, title: "t1".into(), description: None,
                         status: None, parent_task_id: None, image_ids: None,
                         shared_task_id: None },
            task_id,
        ).await.unwrap();

        Task::update(&pool, created.id, project_id, "t2".into(), None,
                     TaskStatus::InProgress, None).await.unwrap();

        let ops = OutboxRepository::peek_unacked(&pool, 10).await.unwrap();
        assert_eq!(ops.len(), 2, "create + update each enqueue one op");
        assert!(ops.iter().all(|o| o.op_type == "task.upsert"));
        assert!(ops.iter().all(|o| o.entity_type == "task"));
        assert!(ops.iter().all(|o| o.entity_id == task_id));
        assert!(ops[1].seq > ops[0].seq, "causal order preserved");
        // Distinct idempotency keys (NOT task:{id}:{version} — see Change note): no UNIQUE violation.
        assert_ne!(ops[0].idempotency_key, ops[1].idempotency_key);
    }
}
```
> If `CreateTask`'s field set differs from the literal above, adjust the struct literal to match the
> real `CreateTask` (see `super::CreateTask`). STOP-check below.

## Change
- **File:** `crates/db/src/models/task/queries.rs`
- **Anchor:** `impl Task` — `create` (@262-292) and `update` (@294-327). The enqueue is appended AFTER
  each query returns its `Task`, on the SAME `pool`, BEFORE returning.
- **Sibling read (rubric #9):** the enqueue API is `crate::models::node_outbox::{OutboxRepository,
  NewOutboxOp}` (task 104). The op payload is the synced task shape; for the tracer, serialize the
  returned `Task` row itself (`serde_json::to_value(&task)`) — the hive's `upsert_from_node` (106) reads
  the fields it needs.
  Each edit is TWO literal changes: bind the `query_as!` result with `let task =`, and replace the
  trailing `.await` with `.await?; … Ok(task)`.
- **Before (`create`, the `query_as!` opening @268):**
```rust
        sqlx::query_as!(
            Task,
```
- **After:**
```rust
        let task = sqlx::query_as!(
            Task,
```
- **Before (`create` tail, @290-292):**
```rust
        )
        .fetch_one(pool)
        .await
    }
```
- **After:**
```rust
        )
        .fetch_one(pool)
        .await?;
        Self::enqueue_task_upsert_op(pool, &task).await;
        Ok(task)
    }
```
- **Before (`update`, the `query_as!` opening @303):**
```rust
        sqlx::query_as!(
            Task,
```
- **After:**
```rust
        let task = sqlx::query_as!(
            Task,
```
  > CRITICAL: `sqlx::query_as!(\n            Task,` appears SIX times in this file (@167,188,213,241,268,303).
  > Rebind `let task =` ONLY at the `create` (@268) and `update` (@303) occurrences — the other four are
  > SELECT methods using `.fetch_optional`/`.fetch_all` and MUST NOT be touched. Anchor by line, not by
  > the (non-unique) text. The `.fetch_one(pool)` tail IS unique to create/update (only @290,@325), so
  > the tail Before/After is unambiguous — but the `let task =` opener is not; pair them by proximity.
- **Before (`update` tail, @325-327):**
```rust
        )
        .fetch_one(pool)
        .await
    }
```
- **After:**
```rust
        )
        .fetch_one(pool)
        .await?;
        Self::enqueue_task_upsert_op(pool, &task).await;
        Ok(task)
    }
```
  > Both `create` and `update` also end with the IDENTICAL `)\n        .fetch_one(pool)\n        .await\n    }`
  > tail — apply the rebind to BOTH (they are the only two such tails in this impl after the SELECTs,
  > which use `.fetch_optional`/`.fetch_all`; verify before editing).
- **Add a private helper** on `impl Task` (top of the impl block or just below `update`):
```rust
    /// Enqueue a `task.upsert` op into node_outbox alongside the local write (SC2 tracer).
    /// Runs ALONGSIDE the legacy hive_sync path (additive; hive apply is idempotent). Best-effort:
    /// a failed enqueue is logged, NOT propagated — the legacy path remains the backstop, and the
    /// enqueue is a separate statement from the task write (not one txn), so a crash between them is
    /// covered by the legacy sync. (Threading a shared txn through all Task::create callers is OUT of
    /// scope for the tracer — see decisions-ledger.)
    async fn enqueue_task_upsert_op(pool: &SqlitePool, task: &Task) {
        use crate::models::node_outbox::{NewOutboxOp, OutboxRepository};
        let payload = match serde_json::to_value(task) {
            Ok(v) => v,
            Err(e) => { tracing::warn!(error = %e, task_id = %task.id, "skip outbox enqueue: serialize failed"); return; }
        };
        // Per-write-unique idempotency key. DELIBERATELY NOT `task:{id}:{version}`: Task::update does
        // NOT bump any version column (queries.rs UPDATE sets only title/description/status/parent_task_id),
        // so a version-only key collides on every update and the UNIQUE(idempotency_key) constraint
        // would silently drop the update op. A fresh Uuid suffix is assigned ONCE here and persisted
        // with the row, so a re-transmit of the SAME outbox row reuses the SAME key and the hive dedups
        // (node_op_log PK). The hive also applies idempotently on (source_node_id, source_task_id), so
        // distinct keys across writes of the same task are safe. "Deterministic" is not an SC
        // requirement — only per-write uniqueness + stable-per-row.
        let op = NewOutboxOp {
            op_type: "task.upsert".to_string(),
            entity_type: "task".to_string(),
            entity_id: task.id,
            payload,
            idempotency_key: format!("task:{}:{}", task.id, Uuid::new_v4()),
            fencing_token: None,
        };
        if let Err(e) = OutboxRepository::enqueue_op(pool, op).await {
            tracing::warn!(error = %e, task_id = %task.id, "failed to enqueue task.upsert op (legacy sync is the backstop)");
        }
    }
```

## Allowed moves
ONLY: rebind the `create`/`update` query results to `let task = …await?;`, append the
`enqueue_task_upsert_op` call + `Ok(task)`, and add the one private `enqueue_task_upsert_op` helper. Do
NOT change `Task::create`/`Task::update` public signatures or return types, do NOT remove/alter the
legacy `hive_sync` path, do NOT touch any other `impl Task` method, do NOT edit `node_outbox.rs` or
`NewOutboxOp` (104 owns those).

## STOP triggers
- The real `CreateTask` struct fields differ from the test literal → adjust the test struct literal to
  the actual `super::CreateTask` shape (do NOT change `CreateTask`).
- `serde_json` is a CONFIRMED direct dep of `crates/db` (`crates/db/Cargo.toml:11`), so
  `serde_json::to_value`/`json!` resolve without a new dep. (No STOP needed; stated for grounding.)
- `query!`/macros fail to validate offline → export `DATABASE_URL=sqlite://<repo>/dev_assets/db.sqlite`
  migrated through 101 (Trap 2); do NOT `cargo sqlx prepare`.
- `node_outbox` table / `OutboxRepository` absent → 101 and 104 must be `passed` first (depends_on: 104,
  which depends_on 101).
- Making the enqueue part of the same transaction would require changing `Task::create`'s signature
  (callers pass `&SqlitePool`, not a txn) → DO NOT; the tracer is best-effort with legacy as backstop
  (decisions-ledger). Record if a reviewer pushes for atomicity (that is a later increment).

## Done when
`WAI_TYPECHECK_CMD="cargo check -p db" WAI_TEST_CMD="cargo test -p db outbox_enqueue" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-hive-redesign 105` exits 0
(export `DATABASE_URL=sqlite://<repo>/dev_assets/db.sqlite` migrated through 101 before running — Trap 2.)
