---
id: "201"
phase: 2
title: Add lease_expires_at + fencing_token columns and node_fencing_token_seq (Postgres/hive)
status: done
depends_on: []
parallel: false
conflicts_with: []
files:
  - crates/remote/migrations/20260128000001_add_lease_fencing.sql
  - crates/remote/tests/lease_fencing_migration.rs
irreversible: false
scope_test: "crates/remote/tests/lease_fencing_migration.rs"
allowed_change: create
covers_criteria: [SC3]
covers_tests: []
---
## Failing test (write first)
**PRECONDITION (Trap 2b — NON-NEGOTIABLE):** this test REQUIRES a live, migrated Postgres. The hive
crate's tests SKIP without a DB (`skip_without_db!`), so a skip-guarded run is a HOLLOW pass. The
executor MUST stand up Postgres, run `sqlx::migrate!("./migrations")` (which applies THIS migration over
the existing `node_task_assignments` table from `20251202000000_nodes_swarm.sql`), and export
`DATABASE_URL=postgres://…` before the gate runs. If no CI Postgres is available, RAISE it — do not let
the gate report a skipped test as green.

Create `crates/remote/tests/lease_fencing_migration.rs`. **Sibling read (rubric #9):** there is NO shared
`common` module (`crates/remote/tests/` holds only `backfill_e2e.rs`, `pool_config.rs`, and the Phase-1
`node_op_log_migration.rs`) — inline backfill_e2e's exact helpers verbatim: `fn database_url() ->
Option<String>` (`std::env::var("DATABASE_URL").ok()`), the `skip_without_db!` macro, and `async fn
create_pool() -> PgPool` (`PgPool::connect(&url)`). Model the assertion shape on `node_op_log_migration.rs`
(Phase-1 task 102) — same `skip_without_db!` + raw `sqlx::query` style.

```rust
//! Asserts the lease/fencing columns + the per-hive monotonic token sequence exist after migration.
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
async fn lease_columns_and_token_sequence_exist() {
    skip_without_db!(); // Trap 2b: a real migrated PG MUST be set or this is a hollow pass
    let pool = create_pool().await;

    // (1) The new columns are present and typed. A SELECT that names them must compile + run.
    //     fencing_token is NOT NULL DEFAULT 0; lease_expires_at is nullable.
    let row = sqlx::query(
        "SELECT lease_expires_at, fencing_token FROM node_task_assignments LIMIT 0")
        .fetch_optional(&pool).await.unwrap();
    assert!(row.is_none(), "empty-on-purpose select just proves the columns resolve");

    // (2) fencing_token defaults to 0 for a freshly inserted assignment (pre-existing rows keep 0).
    //     Seed an org/node/task minimally, OR rely on the DEFAULT: insert a row exercising the default.
    //     (Use the same minimal seed style backfill_e2e uses for create_test_organization/node; or, if a
    //     bare insert is blocked by FKs, assert the default via the column's information_schema metadata.)
    let default_expr: Option<String> = sqlx::query_scalar(
        "SELECT column_default FROM information_schema.columns \
         WHERE table_name = 'node_task_assignments' AND column_name = 'fencing_token'")
        .fetch_one(&pool).await.unwrap();
    assert!(default_expr.as_deref().unwrap_or("").contains('0'),
        "fencing_token must default to 0");

    // (3) The monotonic token source exists and is strictly increasing: two nextval calls differ by 1.
    let a: i64 = sqlx::query_scalar("SELECT nextval('node_fencing_token_seq')")
        .fetch_one(&pool).await.unwrap();
    let b: i64 = sqlx::query_scalar("SELECT nextval('node_fencing_token_seq')")
        .fetch_one(&pool).await.unwrap();
    assert!(b > a, "node_fencing_token_seq must be strictly monotonic (b={b} > a={a})");
}
```

## Change
- **File:** `crates/remote/migrations/20260128000001_add_lease_fencing.sql` (NEW)
- **Anchor:** new Postgres migration; timestamp must sort AFTER the latest existing PG migration. The
  latest on disk is `20260127000000_add_backfill_request_id.sql`; **Phase-1 task 102 introduces
  `20260128000000_add_node_op_log.sql`** which lands before this phase. Use `20260128000001` so it sorts
  strictly after BOTH (STOP-check below).
- **Sibling read (rubric #9):** `crates/remote/migrations/20260127000000_add_backfill_request_id.sql`
  (Postgres conventions: forward-only DDL, `TIMESTAMPTZ`, no `IF NOT EXISTS`) AND the original
  `node_task_assignments` DDL in `20251202000000_nodes_swarm.sql` (the table this ALTERs). Confirm the
  table name + that `fencing_token`/`lease_expires_at` are not already present before authoring.
- **Before:** (file does not exist)
- **After:** exact contents:
```sql
-- Lease + fencing-token columns on node_task_assignments, and the per-hive monotonic token source
-- (CONTRACT §B / ADR-0009 SC3). The lease is the atomic-checkout expiry; the fencing_token is the
-- partition-safety mechanism — every grant bumps it via nextval, and a stale op (token < the
-- assignment's current token) is rejected by the hive (task 205). Pre-existing rows default to token 0.
ALTER TABLE node_task_assignments
    ADD COLUMN lease_expires_at TIMESTAMPTZ,
    ADD COLUMN fencing_token    BIGINT NOT NULL DEFAULT 0;

-- Per-hive monotonic, strictly-increasing fencing token source. try_claim / renew (203) and the
-- expiry sweep (209) call nextval('node_fencing_token_seq') so a reassigned lease ALWAYS gets a
-- strictly higher token than any prior holder — the basis of stale-token rejection (205).
CREATE SEQUENCE node_fencing_token_seq AS BIGINT START WITH 1 INCREMENT BY 1;
```

## Allowed moves
ONLY create the one migration file (exact SQL above) and the one `tests/lease_fencing_migration.rs` test.
Do NOT add a Rust db-model column to `NodeTaskAssignment` (domain.rs) — task 203 reads the new columns via
narrow `RETURNING`/`query_scalar` and does NOT widen the `FromRow` struct (see 203). Do NOT edit
`crates/remote/src/db/task_assignments.rs`, the WS protocol, or any other migration.

## STOP triggers
- A PG migration with a timestamp ≥ `20260128000001` already exists (e.g. 102 chose a later stamp) →
  bump this file to sort strictly last and update `files:` + the test name (record the new name in the
  ledger).
- `node_task_assignments` already has a `fencing_token` or `lease_expires_at` column, or a
  `node_fencing_token_seq` sequence already exists → STOP; the schema drifted. Record; do not duplicate.
- `cargo sqlx prepare` is tempting because a later query task (203) fails offline validation → DO NOT run
  it here (Trap 2). This task adds NO `query!`; the test uses only raw `sqlx::query(...)` which needs a
  live `DATABASE_URL`, not the offline cache. The `.sqlx` regen is a one-shot `/wai:close` housekeeping step.

## Done when
`WAI_TYPECHECK_CMD="cargo check -p remote" WAI_TEST_CMD='test -n "$DATABASE_URL" && cargo test -p remote --test lease_fencing_migration' bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-hive-redesign 201` exits 0
(run with `DATABASE_URL=postgres://…` pointed at a Postgres that has had `./migrations` applied — Trap 2b.
**The `test -n "$DATABASE_URL" &&` prefix makes the gate FAIL-CLOSED** (tournament R1/F2): `task-gate.sh`
runs `WAI_TEST_CMD` via `bash -c`, so with no `DATABASE_URL` the `test -n` fails, the `&&` short-circuits,
and the gate fails — instead of `skip_without_db!` reporting a skipped test as a hollow green.)
