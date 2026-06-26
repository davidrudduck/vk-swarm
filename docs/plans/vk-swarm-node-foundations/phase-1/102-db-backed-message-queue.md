---
id: "102"
phase: 1
title: Back MessageQueueStore with queued_messages table + boot drain
status: ready
depends_on: ["101"]
parallel: false
conflicts_with: []
files:
  - crates/local-deployment/src/message_queue.rs
  - crates/local-deployment/src/container.rs
irreversible: false
scope_test: "crates/local-deployment/src/message_queue.rs"
allowed_change: edit
covers_criteria: [SC2]
---
## Failing test (write first)
In `crates/local-deployment/src/message_queue.rs` `#[cfg(test)] mod tests`, ADD a persistence test that
proves queued messages survive a "restart" (a fresh store over the SAME pool). The existing in-memory
tests must be migrated to construct the store with a pool (see Change); KEEP their assertions.

```rust
#[tokio::test]
async fn test_queue_persists_across_store_recreation() {
    let (pool, _tmp) = db::test_utils::create_test_pool().await;
    // a task_attempt row must exist for the FK; create the minimal fixture the other
    // db tests use (see crates/db tests for the helper, or insert a task_attempt directly).
    let attempt_id = seed_task_attempt(&pool).await; // helper: see STOP triggers
    let store = MessageQueueStore::new(pool.clone());
    store.add(attempt_id, "first".to_string(), None).await;
    store.add(attempt_id, "second".to_string(), None).await;

    // Simulate a restart: drop the store, build a NEW one over the same pool.
    drop(store);
    let store2 = MessageQueueStore::new(pool.clone());
    let remaining = store2.list(attempt_id).await;
    assert_eq!(remaining.len(), 2);
    assert_eq!(remaining[0].content, "first");
    assert_eq!(remaining[0].position, 0);
    assert_eq!(remaining[1].position, 1);

    let popped = store2.pop_next(attempt_id).await.unwrap();
    assert_eq!(popped.content, "first");
    assert_eq!(store2.list(attempt_id).await[0].position, 0); // reindexed
}
```

## Change
- **File:** `crates/local-deployment/src/message_queue.rs`
- **Anchor:** `struct MessageQueueStore` (L48-51), `impl Default` (L53-57), `impl MessageQueueStore`
  (L59-181), and the `#[cfg(test)] mod tests` block (L183-297).
- **Before:** `queues: Arc<RwLock<HashMap<Uuid, Vec<QueuedMessage>>>>` field; `new()` takes no args;
  every method operates on the in-memory `HashMap`.
- **After:** Replace the backing store with the SQLite pool. EXACT contract to preserve (each method
  keeps its current signature + return type — ONLY the body becomes a DB query):
  - struct field: `pool: sqlx::SqlitePool` (drop the `Arc<RwLock<HashMap…>>`). Keep `#[derive(Clone)]`
    (SqlitePool is Clone).
  - `new(pool: SqlitePool) -> Self` (NOW TAKES A POOL). Remove the `Default` impl (cannot default a
    pool) — or keep a `new()`-less type; update all constructors (see container.rs change).
  - `list(attempt) -> Vec<QueuedMessage>`: `SELECT … WHERE task_attempt_id = ? ORDER BY position ASC`.
  - `add(attempt, content, variant) -> QueuedMessage`: compute `position = COUNT(*) WHERE
    task_attempt_id = ?`; `INSERT` a new row with `Uuid::new_v4()` and `Utc::now()`; return the row.
  - `update(attempt, id, content, variant) -> Option<QueuedMessage>`: same variant semantics as today
    (None preserves; empty-string clears; non-empty sets) via `UPDATE` + re-`SELECT` the row.
  - `remove(attempt, id) -> bool`: `DELETE`; if a row was deleted, **re-pack positions** to remain
    0-based contiguous (`UPDATE … SET position = position - 1 WHERE task_attempt_id = ? AND position >
    <removed_pos>`), return whether a row was removed.
  - `reorder(attempt, ids) -> Option<Vec<QueuedMessage>>`: keep the validation (len + id-set equality);
    rewrite positions per the new order in a transaction; return the reordered rows.
  - `peek_next(attempt) -> Option<QueuedMessage>`: `SELECT … ORDER BY position ASC LIMIT 1`.
  - `pop_next(attempt) -> Option<QueuedMessage>`: select first, `DELETE` it, re-pack positions, return it.
  - `clear(attempt)`: `DELETE … WHERE task_attempt_id = ?`.
  - All methods become genuinely fallible at the DB layer: log-and-degrade to the current return shape
    (e.g. `.unwrap_or_default()` / `None` / `false`) so signatures are unchanged — match how the
    callers at `container.rs:1203` (`peek_next`) and `container.rs:1294` (`remove`) consume them.
  - Migrate the existing 7 unit tests to build `MessageQueueStore::new(pool)` over a `create_test_pool`
    and a seeded `task_attempt`; KEEP every assertion.

- **File:** `crates/local-deployment/src/container.rs`
- **Anchor:** L127 `let message_queue = crate::message_queue::MessageQueueStore::new();`
- **Before:** `MessageQueueStore::new()`
- **After:** `MessageQueueStore::new(<pool>)` — pass the same `SqlitePool` the deployment already holds
  (find the pool/`DBService` in scope at L127; it is constructed nearby — use it). The boot drain is
  AUTOMATIC: because the queue now reads from the table, any rows present at startup are returned by
  the existing `peek_next`/drain path (`try_consume_queued_message`, `container.rs:1179`, fired at
  `:738`) with NO new boot code. Confirm no other `MessageQueueStore::new()` call sites exist
  (`grep -rn "MessageQueueStore::new" crates/`); update each.

## Allowed moves
Convert the queue's backing store from in-memory to SQLite, preserving every method signature and the
position-contiguity invariant. Update the one (or more) `MessageQueueStore::new()` call site(s) to pass
the pool. Migrate the existing tests. Do NOT change `QueuedMessage`/request structs, the queue's public
method names, or the consumer logic at `container.rs:1179/1203/1294`.

## STOP triggers
- `MessageQueueStore::new()` is called somewhere outside `container.rs:127` that has no pool in scope →
  STOP, record in ledger (the store may need threading the pool further than expected).
- No existing test seed helper for a `task_attempt` row → create a minimal local `seed_task_attempt`
  helper inside the test module (insert the required parent rows: project → task → task_attempt) OR
  reuse a `crates/db` test fixture if one is exported. Record which.
- Removing `impl Default for MessageQueueStore` breaks a caller relying on `Default`/`derive(Default)`
  on an owner struct → STOP and thread the pool instead.
- The `query_as!`/`query!` macros fail to compile because the schema isn't materialized → apply the
  101 migration to the dev DB AND/OR run `cargo sqlx prepare --workspace` (Trap 2 in the ledger).

## Done when
`WAI_TYPECHECK_CMD="cargo check -p local-deployment" WAI_TEST_CMD="cargo test -p local-deployment message_queue" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-node-foundations 102` exits 0
