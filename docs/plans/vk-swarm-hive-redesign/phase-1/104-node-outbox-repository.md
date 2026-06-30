---
id: "104"
phase: 1
title: Add node OutboxRepository (enqueue_op / peek_unacked / mark_acked_through)
status: ready
depends_on: ["101"]
parallel: false
conflicts_with: []
files:
  - crates/db/src/models/node_outbox.rs
  - crates/db/src/models/mod.rs
irreversible: false
scope_test: "crates/db/src/models/node_outbox.rs"
allowed_change: mixed
covers_criteria: [SC2]
covers_tests: [TS1]
---
## Failing test (write first)
In the NEW `crates/db/src/models/node_outbox.rs`, add a `#[cfg(test)] mod tests` round-trip that proves:
seq is monotonic per enqueue, `peek_unacked` returns ops in seq order, and `mark_acked_through` clears
exactly the ops at/under a seq (cursor advance). Hermetic — `db::test_utils::create_test_pool()` (the
template DB has 101's migration applied).

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn outbox_enqueue_peek_ack_roundtrip() {
        let (pool, _tmp) = db::test_utils::create_test_pool().await;

        let mk = |k: &str| NewOutboxOp {
            op_type: "task.upsert".into(),
            entity_type: "task".into(),
            entity_id: uuid::Uuid::new_v4(),
            payload: serde_json::json!({"title": "x"}),
            idempotency_key: k.into(),
            fencing_token: None,
        };

        let a = OutboxRepository::enqueue_op(&pool, mk("task:a:1")).await.unwrap();
        let b = OutboxRepository::enqueue_op(&pool, mk("task:b:1")).await.unwrap();
        assert!(b.seq > a.seq, "seq must be per-node monotonic");

        let unacked = OutboxRepository::peek_unacked(&pool, 10).await.unwrap();
        assert_eq!(unacked.len(), 2);
        assert_eq!(unacked[0].seq, a.seq, "peek_unacked is seq-ordered");
        assert_eq!(unacked[1].seq, b.seq);

        // Ack through the first op's seq → only b remains unacked.
        OutboxRepository::mark_acked_through(&pool, a.seq).await.unwrap();
        let remaining = OutboxRepository::peek_unacked(&pool, 10).await.unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].seq, b.seq);
    }

    #[tokio::test]
    async fn enqueue_op_rejects_duplicate_idempotency_key() {
        let (pool, _tmp) = db::test_utils::create_test_pool().await;
        let op = NewOutboxOp {
            op_type: "task.upsert".into(), entity_type: "task".into(),
            entity_id: uuid::Uuid::new_v4(), payload: serde_json::json!({}),
            idempotency_key: "task:dup:1".into(), fencing_token: None,
        };
        OutboxRepository::enqueue_op(&pool, op.clone()).await.unwrap();
        assert!(OutboxRepository::enqueue_op(&pool, op).await.is_err(),
            "UNIQUE(idempotency_key) must reject a re-enqueue");
    }
}
```

## Change
- **File:** `crates/db/src/models/node_outbox.rs` (NEW)
- **Anchor:** new model module. **Sibling read (rubric #9):** `crates/db/src/models/draft.rs` — same
  conventions: `use chrono::{DateTime, Utc}`, `serde`, `sqlx::{FromRow, SqlitePool}`, `uuid::Uuid`;
  untyped `sqlx::query_as!`/`query!` macros; row model + repository fns taking `&SqlitePool` (stateless).
  Also see task 102's sibling `vk-swarm-node-foundations` 102 for the SQLx `usize` trap pattern (we use
  `i64` for `seq` here, so no `usize` issue).
- **Before:** (file does not exist)
- **After:** module with:
  - **`OutboxOp` row model** (the persisted row; distinct from the WS `OutboxOp` in hive_client.rs —
    this one carries `id`/`created_at`/`acked_at`):
    ```rust
    #[derive(Debug, Clone)]
    pub struct OutboxOp {
        pub id: Uuid,
        pub seq: i64,
        pub op_type: String,
        pub entity_type: String,
        pub entity_id: Uuid,
        pub payload: serde_json::Value,   // stored as TEXT JSON
        pub idempotency_key: String,
        pub fencing_token: Option<i64>,
        pub created_at: DateTime<Utc>,
        pub acked_at: Option<DateTime<Utc>>,
    }
    ```
  - **`NewOutboxOp`** (insert input — no id/seq/created_at/acked_at; the repo assigns id + seq):
    ```rust
    #[derive(Debug, Clone)]
    pub struct NewOutboxOp {
        pub op_type: String,
        pub entity_type: String,
        pub entity_id: Uuid,
        pub payload: serde_json::Value,
        pub idempotency_key: String,
        pub fencing_token: Option<i64>,
    }
    ```
  - **`OutboxRepository`** (stateless; methods take `&SqlitePool`):
    - `enqueue_op(pool, op: NewOutboxOp) -> Result<OutboxOp, sqlx::Error>`:
      `id = Uuid::new_v4()`; serialize `payload` with `serde_json::to_string`. **Allocate `seq` INLINE in
      ONE statement** — `INSERT INTO node_outbox (id, seq, op_type, entity_type, entity_id, payload,
      idempotency_key, fencing_token) VALUES (?, (SELECT COALESCE(MAX(seq),0)+1 FROM node_outbox), ?, ?,
      ?, ?, ?, ?) RETURNING …`. Computing `seq` as a **scalar subquery WITHIN the INSERT** is atomic under
      SQLite's single-writer lock, so two concurrent `Task::create`/`update` enqueues cannot read the same
      `MAX(seq)` and duplicate it (tournament R1/F4 — do NOT split into a separate `SELECT MAX(seq)` then
      `INSERT`). The `UNIQUE(seq)` constraint (101) is the backstop. Map the returned row back to
      `OutboxOp` (parse `payload` TEXT with `serde_json::from_str`). The `UNIQUE(idempotency_key)`
      constraint surfaces as `sqlx::Error` on a duplicate (test 2 asserts this) — do NOT swallow it.
    - `peek_unacked(pool, limit: i64) -> Result<Vec<OutboxOp>, sqlx::Error>`:
      `SELECT … FROM node_outbox WHERE acked_at IS NULL ORDER BY seq ASC LIMIT ?` (uses the
      `idx_node_outbox_unacked_seq` partial index).
    - `mark_acked_through(pool, seq: i64) -> Result<(), sqlx::Error>`:
      `UPDATE node_outbox SET acked_at = datetime('now','subsec') WHERE acked_at IS NULL AND seq <= ?`
      (cursor advance — only forward; already-acked rows untouched).
  - **SQLx note:** `payload` is TEXT in the DB but `serde_json::Value` in the model — the macros cannot
    map JSON↔Value automatically; SELECT the column as `String` then `serde_json::from_str`, and bind
    `serde_json::to_string(&op.payload)?` on INSERT. `entity_id`/`id` are BLOB UUIDs (repo's BLOB-UUID
    support handles `Uuid`). `seq`/`fencing_token` are `INTEGER` ↔ `i64`/`Option<i64>`.

- **File:** `crates/db/src/models/mod.rs`
- **Anchor:** the `pub mod …;` block (@17-34).
- **Before (@30-31):**
```rust
pub mod task_attempt;
pub mod task_variable;
```
- **After:**
```rust
pub mod node_outbox;
pub mod task_attempt;
pub mod task_variable;
```

## Allowed moves
ONLY create `node_outbox.rs` (row model + `NewOutboxOp` + `OutboxRepository` with the three methods +
the test module) and register `pub mod node_outbox;` in `mod.rs`. Do NOT wire any caller (105 owns the
task-create enqueue), do NOT touch the WS protocol (103), do NOT add a migration (101 owns it).

## STOP triggers
- The `query!`/`query_as!` macros fail to compile because the schema isn't materialized → export
  `DATABASE_URL=sqlite://<repo>/dev_assets/db.sqlite` against a dev DB with 101's migration applied so
  the macros validate against the live schema (Trap 2). Do NOT `cargo sqlx prepare` in this task (it
  churns the tracked `.sqlx` cache the gate rejects) — the regen is a `/wai:close` step.
- `node_outbox` table absent from the dev DB → 101 must be `passed` and its migration applied to the dev
  DB before this task's macros validate. (depends_on: 101.)
- A `usize` column sneaks in → keep `seq`/`fencing_token` as `i64`/`Option<i64>`; SQLite's sqlx driver
  has no `Decode` for `usize` (the node-foundations 102 trap).
- `serde_json::Value` is bound directly to a TEXT column or SELECTed into directly → STOP; round-trip via
  `String` + `serde_json::{to_string,from_str}` (the macros will not auto-map it).

## Done when
`WAI_TYPECHECK_CMD="cargo check -p db" WAI_TEST_CMD="cargo test -p db node_outbox" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-hive-redesign 104` exits 0
(export `DATABASE_URL=sqlite://<repo>/dev_assets/db.sqlite` migrated through 101 before running — Trap 2.)
