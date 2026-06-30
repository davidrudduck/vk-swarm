---
id: "702"
phase: 7
title: Cutover guard — MUST-MIGRATE tables round-trip with id bridge intact + status remapped
status: ready
depends_on: ["701"]
parallel: false
conflicts_with: []
files:
  - crates/remote/tests/hive_cutover_must_migrate.rs
irreversible: false
scope_test: "crates/remote/tests/hive_cutover_must_migrate.rs"
allowed_change: create
covers_criteria: [SC6]
covers_tests: [TS6]
---
## Failing test (write first)
**PRECONDITION (Trap 2b — NON-NEGOTIABLE):** REQUIRES a live, migrated Postgres (the cutover migration
from 701 must be applied). The hive crate's tests SKIP without a DB; the `test -n "$DATABASE_URL" &&`
prefix in `## Done when` makes the gate FAIL-CLOSED. Stand up Postgres, run
`sqlx::migrate!("./migrations")`, export `DATABASE_URL=postgres://…` before the gate.

**This is the SC6/TS6 heart:** after cutover, every MUST-MIGRATE table still round-trips, the node↔hive
id bridge (`shared_tasks.source_task_id`/`source_node_id` ↔ node `tasks.shared_task_id`) survives, and
status values are canonical. **Verified fact that shapes the status assertion** (NOT a remap of stored
rows): the hive `task_status` enum is ALREADY kebab-case (`in-progress`/`in-review`) — see
`crates/remote/migrations/20251001000000_shared_tasks_activity.sql:57` and the `#[sqlx(type_name =
"task_status", rename_all = "kebab-case")]` enum at `crates/remote/src/db/tasks.rs:24`. The
`inprogress`/`inreview` (node-SQLite lowercase, `crates/db/src/models/task/mod.rs:24`) → `in-progress`/
`in-review` (hive) **remap happens at the node→hive INGEST boundary** (the op-log apply), so hive
data-at-rest is already canonical. This test asserts (a) the canonical hive value is the kebab form, and
(b) a row inserted via the `upsert_from_node` shape with the id bridge set round-trips with status
preserved canonically — i.e. nothing in the bridge or status is silently lost across cutover.

**Sibling read (rubric #9):** inline backfill_e2e's `database_url()`/`skip_without_db!`/`create_pool()`
verbatim (no shared `common` module). Read `crates/remote/src/db/tasks.rs` `upsert_from_node`
(`tasks.rs:558`) for the exact insert columns + the `task_status` bind shape before writing the insert.

```rust
//! SC6 / TS6 — MUST-MIGRATE round-trip after cutover: id bridge intact, status canonical.
use sqlx::PgPool;

fn database_url() -> Option<String> { std::env::var("DATABASE_URL").ok() }
macro_rules! skip_without_db { () => {
    if database_url().is_none() { eprintln!("Skipping: DATABASE_URL not set"); return; }
}; }
async fn create_pool() -> PgPool {
    PgPool::connect(&database_url().unwrap()).await.expect("connect")
}

#[tokio::test]
async fn must_migrate_id_bridge_and_status_round_trip() {
    skip_without_db!(); // Trap 2b: a real migrated PG MUST be set or this is a hollow pass
    let pool = create_pool().await;

    // The canonical hive wire value for the node 'inprogress' state is kebab 'in-progress'
    // (CONTRACT.md §D; ADR-0010). Prove the enum accepts the canonical form and rejects the
    // node-lowercase form — i.e. the value space is canonical at rest, never the node form.
    let kebab_ok: bool = sqlx::query_scalar(
        "SELECT 'in-progress'::task_status = 'in-progress'::task_status")
        .fetch_one(&pool).await.unwrap();
    assert!(kebab_ok, "hive task_status must accept canonical kebab 'in-progress'");
    let lower_rejected = sqlx::query_scalar::<_, bool>("SELECT 'inprogress'::task_status IS NOT NULL")
        .fetch_one(&pool).await;
    assert!(lower_rejected.is_err(), "node-lowercase 'inprogress' must NOT be a valid hive task_status \
            value (remap happens at ingest, not at rest)");

    // Round-trip a MUST-MIGRATE shared_tasks row with the id bridge set, mirroring upsert_from_node.
    // Seed the minimal MUST-MIGRATE parents (org, swarm_project, node) so the FK/bridge is real.
    let org_id = uuid::Uuid::new_v4();
    sqlx::query("INSERT INTO organizations (id, name) VALUES ($1, 'cutover-test')")
        .bind(org_id).execute(&pool).await.unwrap();
    let node_id = uuid::Uuid::new_v4();
    // node insert columns: confirm against 20251202000000_nodes_swarm.sql before running (STOP trigger).
    sqlx::query("INSERT INTO nodes (id, organization_id, name, machine_id) \
                 VALUES ($1, $2, 'n1', $3)")
        .bind(node_id).bind(org_id).bind(uuid::Uuid::new_v4().to_string())
        .execute(&pool).await.unwrap();
    let swarm_project_id = uuid::Uuid::new_v4();
    sqlx::query("INSERT INTO swarm_projects (id, organization_id, name) VALUES ($1, $2, 'p1')")
        .bind(swarm_project_id).bind(org_id).execute(&pool).await.unwrap();

    // The id BRIDGE: source_task_id = the node-local tasks.id; source_node_id = the node.
    let source_task_id = uuid::Uuid::new_v4();
    let shared_id = uuid::Uuid::new_v4();
    sqlx::query(
        "INSERT INTO shared_tasks \
           (id, organization_id, swarm_project_id, source_node_id, source_task_id, title, status) \
         VALUES ($1, $2, $3, $4, $5, 'bridged', 'in-progress'::task_status)")
        .bind(shared_id).bind(org_id).bind(swarm_project_id)
        .bind(node_id).bind(source_task_id)
        .execute(&pool).await.unwrap();

    // Round-trip: the bridge resolves the shared task back from (source_node_id, source_task_id),
    // and the status survives as the canonical kebab form.
    let (got_id, got_status): (uuid::Uuid, String) = sqlx::query_as(
        "SELECT id, status::text FROM shared_tasks \
         WHERE source_node_id = $1 AND source_task_id = $2")
        .bind(node_id).bind(source_task_id).fetch_one(&pool).await.unwrap();
    assert_eq!(got_id, shared_id, "id bridge must resolve the shared task by (source_node_id, source_task_id)");
    assert_eq!(got_status, "in-progress", "status must round-trip as canonical kebab form");
}
```

> **Note on parent insert columns:** the exact `INSERT INTO nodes` / `swarm_projects` / `shared_tasks`
> column lists above are the minimal sets read from the migrations. The executor MUST confirm them
> against `20251202000000_nodes_swarm.sql`, `20260101000000_create_swarm_projects.sql`,
> `20260121000000_add_swarm_project_id.sql` (the `swarm_project_id` column on `shared_tasks`) and adjust
> binds to satisfy NOT-NULL constraints (STOP trigger below) — the assertion logic (bridge resolve +
> status text) is what is load-bearing, not the exact seed column set.

## Change
- **File:** `crates/remote/tests/hive_cutover_must_migrate.rs` (NEW) — exact test above.
- **Anchor:** new integration test beside `crates/remote/tests/hive_cutover_migration.rs` (701) and
  `backfill_e2e.rs`. No production code changes.
- **Before:** (file does not exist)
- **After:** the test module above (executor finalizes the seed-insert column lists per the Note).

## Allowed moves
ONLY create the one `tests/hive_cutover_must_migrate.rs` test. Do NOT add a migration, do NOT edit
`crates/remote/src/`, do NOT touch the WS protocol. This task is a VERIFICATION guard — it asserts the
in-place MUST-MIGRATE state (id bridge + canonical status) survives cutover; it changes no schema.

## STOP triggers
- A seeded parent INSERT fails on a NOT-NULL / FK column not in the list above → STOP, read the cited
  migration, add the missing required column to the seed (do NOT weaken the assertions). Record the
  corrected column set.
- `'in-progress'::task_status` errors (enum value absent) → the hive enum is not what the inventory
  claims; STOP and reconcile against `20251001000000_shared_tasks_activity.sql:57` before proceeding.
- The id-bridge columns (`source_task_id`/`source_node_id`) are absent on `shared_tasks` → STOP; the
  bridge that ADR-0011 says MUST be preserved is missing. Do NOT drop the assertion.
- **Rebuilt-schema interpretation contested** (see 701) → if a fresh-schema copy is chosen, this guard
  must run against the copied schema; re-author the seed/round-trip accordingly.
- `cargo sqlx prepare` is tempting → DO NOT (Trap 2). This task adds NO `query!`; only raw
  `sqlx::query(...)`/`query_as(...)`, which need only a live `DATABASE_URL`.

## Done when
`WAI_TYPECHECK_CMD="cargo check -p remote" WAI_TEST_CMD='test -n "$DATABASE_URL" && cargo test -p remote --test hive_cutover_must_migrate' bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-hive-redesign 702` exits 0
(run with `DATABASE_URL=postgres://…` against a Postgres with `./migrations` applied — Trap 2b. The
`test -n "$DATABASE_URL" &&` prefix makes the gate FAIL-CLOSED — no `DATABASE_URL` → short-circuit fail,
never a hollow skipped green.)
