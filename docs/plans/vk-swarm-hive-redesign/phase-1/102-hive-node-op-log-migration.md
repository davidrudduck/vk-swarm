---
id: "102"
phase: 1
title: Add node_op_log table migration (Postgres / hive)
status: done
depends_on: []
parallel: false
conflicts_with: []
files:
  - crates/remote/migrations/20260128000000_add_node_op_log.sql
  - crates/remote/tests/node_op_log_migration.rs
irreversible: false
scope_test: "crates/remote/tests/node_op_log_migration.rs"
allowed_change: create
covers_criteria: [SC2]
---
## Failing test (write first)
**PRECONDITION (Trap 2b — NON-NEGOTIABLE):** this test REQUIRES a live, migrated Postgres. The hive
crate's tests SKIP without a DB (`skip_without_db!`), so a skip-guarded run is a HOLLOW pass. The
executor MUST stand up Postgres, run `sqlx::migrate!("./migrations")` (which applies THIS migration),
and export `DATABASE_URL=postgres://…` before the gate runs. If no CI Postgres is available, RAISE it
before executing — do not let the gate report a skipped test as green.

Create `crates/remote/tests/node_op_log_migration.rs`. **Sibling read (rubric #9):** there is NO shared
`common` module (`crates/remote/tests/` holds only `backfill_e2e.rs` + `pool_config.rs`) — inline
backfill_e2e's exact helpers verbatim: `fn database_url() -> Option<String>` (`std::env::var("DATABASE_URL").ok()`),
the `skip_without_db!` macro, and `async fn create_pool() -> PgPool` (`PgPool::connect(&url)`).

```rust
//! Asserts the node_op_log table + its dedup contract exist after migration.
use sqlx::PgPool;

// Inlined verbatim from crates/remote/tests/backfill_e2e.rs (no shared `common` module exists).
fn database_url() -> Option<String> { std::env::var("DATABASE_URL").ok() }
macro_rules! skip_without_db { () => {
    if database_url().is_none() { eprintln!("Skipping: DATABASE_URL not set"); return; }
}; }
async fn create_pool() -> PgPool {
    PgPool::connect(&database_url().unwrap()).await.expect("connect")
}

#[tokio::test]
async fn node_op_log_table_and_pk_exist() {
    skip_without_db!(); // Trap 2b: a real migrated PG MUST be set or this is a hollow pass
    let pool = create_pool().await;

    // Table exists with the dedup PRIMARY KEY (node_id, idempotency_key): inserting the same key twice
    // for the same node conflicts; ON CONFLICT DO NOTHING is what 106 relies on.
    let node_id = uuid::Uuid::new_v4();
    let entity_id = uuid::Uuid::new_v4();
    sqlx::query(
        "INSERT INTO node_op_log (node_id, idempotency_key, seq, op_type, entity_id) \
         VALUES ($1,$2,$3,$4,$5)")
        .bind(node_id).bind("task:1:1").bind(1_i64).bind("task.upsert").bind(entity_id)
        .execute(&pool).await.unwrap();

    let affected = sqlx::query(
        "INSERT INTO node_op_log (node_id, idempotency_key, seq, op_type, entity_id) \
         VALUES ($1,$2,$3,$4,$5) ON CONFLICT (node_id, idempotency_key) DO NOTHING")
        .bind(node_id).bind("task:1:1").bind(1_i64).bind("task.upsert").bind(entity_id)
        .execute(&pool).await.unwrap().rows_affected();
    assert_eq!(affected, 0, "duplicate (node_id, idempotency_key) must be deduped");

    // High-water = max(seq) per node.
    let hw: Option<i64> = sqlx::query_scalar(
        "SELECT MAX(seq) FROM node_op_log WHERE node_id = $1")
        .bind(node_id).fetch_one(&pool).await.unwrap();
    assert_eq!(hw, Some(1));
}
```

## Change
- **File:** `crates/remote/migrations/20260128000000_add_node_op_log.sql` (NEW)
- **Anchor:** new Postgres migration; timestamp must sort AFTER the latest existing PG migration
  (`20260127000000_add_backfill_request_id.sql`). Use `20260128000000` (or later — STOP-check below).
- **Sibling read (rubric #9):** `crates/remote/migrations/20260127000000_add_backfill_request_id.sql`
  — Postgres conventions: `UUID` type, `TIMESTAMPTZ`, partial index, no `IF NOT EXISTS` needed (these
  run once forward). Use `now()` default like other PG tables.
- **Before:** (file does not exist)
- **After:** exact contents:
```sql
-- Hive-side dedup + per-node high-water cursor for the node→hive op-log (SC2c). The hive applies each
-- op idempotently: INSERT … ON CONFLICT (node_id, idempotency_key) DO NOTHING (see 106). The durable
-- ack the node receives (HiveMessage::OpAck) is keyed off MAX(seq) per node = the applied-through cursor.
CREATE TABLE node_op_log (
    node_id         UUID NOT NULL,
    idempotency_key TEXT NOT NULL,
    seq             BIGINT NOT NULL,
    op_type         TEXT NOT NULL,
    entity_id       UUID NOT NULL,
    applied_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (node_id, idempotency_key)
);

-- Per-node high-water lookup: SELECT MAX(seq) WHERE node_id = $1.
CREATE INDEX idx_node_op_log_node_seq ON node_op_log (node_id, seq);
```

## Allowed moves
ONLY create the one migration file (exact SQL above) and the one `tests/node_op_log_migration.rs` test.
Do NOT add a Rust db-model module, edit `crates/remote/src/db/`, or touch the WS protocol — those are
103/106.

## STOP triggers
- A PG migration with a timestamp ≥ `20260128000000` already exists → bump to sort strictly last and
  update `files:` (record the new name).
- A divergent skip guard is tempting → DO NOT; inline backfill_e2e's exact `database_url()`/
  `skip_without_db!`/`create_pool()` (verified: no shared `common` module exists in `remote/tests/`).
- The migration fails to apply (e.g. type name clash) → STOP; do not silently rename. Record.
- `cargo sqlx prepare` is tempting because a later query task fails offline validation → DO NOT run it
  here (Trap 2). This task adds NO `query!`; only raw `sqlx::query(...)` in the test, which needs only
  a live `DATABASE_URL`, not the offline cache.

## Done when
`WAI_TYPECHECK_CMD="cargo check -p remote" WAI_TEST_CMD='test -n "$DATABASE_URL" && cargo test -p remote --test node_op_log_migration' bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-hive-redesign 102` exits 0
(run with `DATABASE_URL=postgres://…` pointed at a Postgres that has had `./migrations` applied — Trap 2b.
**The `test -n "$DATABASE_URL" &&` prefix makes the gate FAIL-CLOSED** (tournament R1/F2): `task-gate.sh`
runs `WAI_TEST_CMD` via `bash -c`, so with no `DATABASE_URL` the `test -n` fails, the `&&` short-circuits,
and the gate fails — instead of `skip_without_db!` reporting a skipped test as a hollow green.)
