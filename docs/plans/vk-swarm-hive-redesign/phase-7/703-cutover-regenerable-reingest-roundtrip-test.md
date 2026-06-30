---
id: "703"
phase: 7
title: Cutover guard — REGENERABLE tables repopulate from a simulated node re-ingest
status: ready
depends_on: ["701"]
parallel: false
conflicts_with: []
files:
  - crates/remote/tests/hive_cutover_reingest.rs
irreversible: false
scope_test: "crates/remote/tests/hive_cutover_reingest.rs"
allowed_change: create
covers_criteria: [SC6]
covers_tests: [TS6]
---
## Failing test (write first)
**PRECONDITION (Trap 2b — NON-NEGOTIABLE):** REQUIRES a live, migrated Postgres with 701's cutover
migration applied (so the REGENERABLE tables exist but are EMPTY). The hive crate's tests SKIP without a
DB; the `test -n "$DATABASE_URL" &&` prefix in `## Done when` makes the gate FAIL-CLOSED. Stand up
Postgres, run `sqlx::migrate!("./migrations")`, export `DATABASE_URL=postgres://…` before the gate.

**This closes TS6's "regenerable tables repopulate from a simulated node re-ingest" clause.** After
cutover (701) the REGENERABLE node-mirror table `node_task_attempts` is EMPTY but present; this test
drives a node re-ingest and asserts the row reappears — proving the data-clear (not drop) reading lets
re-ingest refill the surviving schema, with the id bridge to a MUST-MIGRATE `shared_tasks` row intact.

**FIDELITY FLAG (ratified tracer limitation — record in decisions-ledger).** The NEW ADR-0008 op-log
re-ingest for attempt/exec/log op types **does NOT exist yet** — P1 shipped only `task.upsert` on the
new outbox (decisions-ledger "tracer honesty"). So this test simulates re-ingest by driving the
**EXISTING** node-mirror upsert path — `NodeTaskAttemptRepository::upsert`
(`crates/remote/src/db/node_task_attempts.rs:46`), the path `handle_attempt_sync`
(`session.rs:1188`) already uses today — NOT the new op-log. It proves the cutover leaves a refillable
schema; it does NOT prove the ADR-0008 op-log mechanism (that is P1/P5 fidelity, separately tracked).
State this in the test doc-comment so a reader does not mistake it for op-log proof.

**Sibling read (rubric #9):** inline backfill_e2e's `database_url()`/`skip_without_db!`/`create_pool()`
verbatim (no shared `common` module). Read `UpsertNodeTaskAttempt` + `upsert`
(`crates/remote/src/db/node_task_attempts.rs:18,46`) for the exact field set and the `(id) DO UPDATE`
shape before writing the call. Seed the MUST-MIGRATE parents (org, swarm_project, node, shared_task)
exactly as 702 does — confirm NOT-NULL columns against the cited migrations (STOP trigger).

```rust
//! SC6 / TS6 — REGENERABLE re-ingest: after cutover (data cleared, schema kept), a simulated node
//! re-ingest repopulates node_task_attempts, with the id bridge to shared_tasks intact.
//! NOTE: drives the EXISTING NodeTaskAttemptRepository::upsert path (what handle_attempt_sync uses
//! today), NOT the new ADR-0008 op-log — the op-log re-ingest for attempts does not exist yet
//! (P1 shipped only task.upsert). This proves the schema is refillable post-cutover, not the op-log.
use remote::db::node_task_attempts::{NodeTaskAttemptRepository, UpsertNodeTaskAttempt};
use sqlx::PgPool;

fn database_url() -> Option<String> { std::env::var("DATABASE_URL").ok() }
macro_rules! skip_without_db { () => {
    if database_url().is_none() { eprintln!("Skipping: DATABASE_URL not set"); return; }
}; }
async fn create_pool() -> PgPool {
    PgPool::connect(&database_url().unwrap()).await.expect("connect")
}

#[tokio::test]
async fn regenerable_node_attempt_repopulates_from_reingest() {
    skip_without_db!(); // Trap 2b: a real migrated PG MUST be set or this is a hollow pass
    let pool = create_pool().await;

    // Seed MUST-MIGRATE parents (preserved across cutover) so the re-ingested attempt has a real
    // shared_task_id bridge. Column lists: confirm against the cited migrations (STOP trigger).
    let org_id = uuid::Uuid::new_v4();
    sqlx::query("INSERT INTO organizations (id, name) VALUES ($1, 'reingest-test')")
        .bind(org_id).execute(&pool).await.unwrap();
    let node_id = uuid::Uuid::new_v4();
    sqlx::query("INSERT INTO nodes (id, organization_id, name, machine_id) \
                 VALUES ($1, $2, 'n1', $3)")
        .bind(node_id).bind(org_id).bind(uuid::Uuid::new_v4().to_string())
        .execute(&pool).await.unwrap();
    let swarm_project_id = uuid::Uuid::new_v4();
    sqlx::query("INSERT INTO swarm_projects (id, organization_id, name) VALUES ($1, $2, 'p1')")
        .bind(swarm_project_id).bind(org_id).execute(&pool).await.unwrap();
    let shared_task_id = uuid::Uuid::new_v4();
    sqlx::query(
        "INSERT INTO shared_tasks (id, organization_id, swarm_project_id, title, status) \
         VALUES ($1, $2, $3, 'reingested', 'in-review'::task_status)")
        .bind(shared_task_id).bind(org_id).bind(swarm_project_id)
        .execute(&pool).await.unwrap();

    // The REGENERABLE table is empty for this task post-cutover (701 cleared it).
    let before: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM node_task_attempts WHERE shared_task_id = $1")
        .bind(shared_task_id).fetch_one(&pool).await.unwrap();
    assert_eq!(before, 0, "REGENERABLE node_task_attempts must start empty post-cutover for this task");

    // Simulate a node re-ingest via the EXISTING upsert path (handle_attempt_sync's mechanism).
    let now = chrono::Utc::now();
    let attempt_id = uuid::Uuid::new_v4();
    let repo = NodeTaskAttemptRepository::new(&pool);
    let upserted = repo.upsert(&UpsertNodeTaskAttempt {
        id: attempt_id,
        assignment_id: None,
        shared_task_id,            // the id bridge to a MUST-MIGRATE shared_tasks row
        node_id,
        executor: "qa_mock".into(),
        executor_variant: None,
        branch: "vk/reingest".into(),
        target_branch: "main".into(),
        container_ref: None,
        worktree_deleted: false,
        setup_completed_at: None,
        created_at: now,
        updated_at: now,
    }).await.expect("re-ingest upsert");

    // The REGENERABLE row reappeared, linked to the preserved shared task.
    assert_eq!(upserted.id, attempt_id);
    assert_eq!(upserted.shared_task_id, shared_task_id, "re-ingested attempt keeps its bridge to shared_tasks");
    let after: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM node_task_attempts WHERE shared_task_id = $1")
        .bind(shared_task_id).fetch_one(&pool).await.unwrap();
    assert_eq!(after, 1, "REGENERABLE node_task_attempts must repopulate from re-ingest");
}
```

> **Note on the import path + seed columns:** `use remote::db::node_task_attempts::…` assumes those
> items are `pub` from the `remote` crate root — confirm the actual module path/visibility
> (`crates/remote/src/db/mod.rs` re-exports, `crates/remote/src/lib.rs`) before running; if not public,
> drive the same `INSERT INTO node_task_attempts (...)` raw SQL (exact column list at
> `node_task_attempts.rs:52`) instead. The seed `INSERT` column lists for nodes/swarm_projects/
> shared_tasks are the minimal sets — confirm NOT-NULL columns against `20251202000000_nodes_swarm.sql`,
> `20260101000000_create_swarm_projects.sql`, `20260121000000_add_swarm_project_id.sql` and adjust.

## Change
- **File:** `crates/remote/tests/hive_cutover_reingest.rs` (NEW) — exact test above.
- **Anchor:** new integration test beside `hive_cutover_migration.rs` (701) and `backfill_e2e.rs`. No
  production code changes.
- **Before:** (file does not exist)
- **After:** the test module above (executor finalizes import path + seed column lists per the Note).

## Allowed moves
ONLY create the one `tests/hive_cutover_reingest.rs` test. Do NOT add a migration, do NOT edit
`crates/remote/src/`, do NOT touch the WS protocol. VERIFICATION only — proves REGENERABLE state refills
from re-ingest into the surviving (cleared) schema.

## STOP triggers
- `NodeTaskAttemptRepository`/`UpsertNodeTaskAttempt` are not importable from the `remote` crate root →
  switch to the raw `INSERT INTO node_task_attempts (...)` (exact columns at `node_task_attempts.rs:52`,
  `sync_state` defaulted `'partial'`). Do NOT weaken the assertions.
- A seeded parent INSERT fails on a NOT-NULL / FK column not in the list above → STOP, read the cited
  migration, add the missing required column to the seed. Record the corrected set.
- `node_task_attempts` does NOT exist (701 DROPped instead of cleared it) → STOP; this contradicts the
  data-clear interpretation. Reconcile with 701 before proceeding — re-ingest needs the surviving table.
- **Rebuilt-schema interpretation contested** (see 701) → if a fresh-schema copy is chosen, this
  re-ingest must target the copied/rebuilt schema; re-author accordingly.
- `cargo sqlx prepare` is tempting → DO NOT (Trap 2). This task adds NO `query!`; it calls the existing
  repo (already in the committed `.sqlx` cache) and raw `sqlx::query(...)`, needing only a live
  `DATABASE_URL`.

## Done when
`WAI_TYPECHECK_CMD="cargo check -p remote" WAI_TEST_CMD='test -n "$DATABASE_URL" && cargo test -p remote --test hive_cutover_reingest' bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-hive-redesign 703` exits 0
(run with `DATABASE_URL=postgres://…` against a Postgres with `./migrations` applied — Trap 2b. The
`test -n "$DATABASE_URL" &&` prefix makes the gate FAIL-CLOSED — no `DATABASE_URL` → short-circuit fail,
never a hollow skipped green.)
