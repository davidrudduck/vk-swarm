---
id: "701"
phase: 7
title: Cutover migration — data-clear REGENERABLE + DISCARDABLE hive-only state (in-place rebuild)
status: ready
depends_on: []
parallel: false
conflicts_with: []
files:
  - crates/remote/migrations/20260201000000_hive_cutover_clear_regenerable_discardable.sql
  - crates/remote/tests/hive_cutover_migration.rs
irreversible: true
scope_test: "crates/remote/tests/hive_cutover_migration.rs"
allowed_change: create
covers_criteria: [SC6]
covers_tests: [TS6]
---
## Cutover interpretation (RATIFIED 2026-06-30 — user chose fresh-schema; code path is necessarily in-place)
**Ratification outcome.** The user chose a "fresh-schema rebuild." Verified engineering reality: the
DISCARDABLE tables are **live infrastructure with code references** (`auth_sessions` 8 refs, `activity`
in `db/activity.rs`+`db/tasks.rs`, etc.), so NO schema — fresh or in-place — can omit them without
breaking `cargo check -p remote`; a `sqlx::migrate!` of a fresh DB recreates them **empty**, identical
end state to the in-place clear below. Therefore: the **CODE cutover is this in-place data operation**
(the only buildable form), and the **fresh-schema rebuild is captured as the operational DEPLOYMENT
RUNBOOK** in the decisions-ledger (dump MUST-MIGRATE → recreate DB → `sqlx::migrate!` → restore
MUST-MIGRATE → REGENERABLE refilled by node re-ingest). TS6's "discardable tables absent" is realized as
"discardable DATA not retained (tables empty post-cutover)" — true under both forms.

**Original supporting analysis (still valid):**
**PRECONDITION (Trap 2b — NON-NEGOTIABLE):** the test REQUIRES a live, migrated Postgres. The hive
crate's tests SKIP without a DB (`skip_without_db!`), so a skip-guarded run is a HOLLOW pass. The
executor MUST stand up Postgres, run `sqlx::migrate!("./migrations")` (which applies THIS migration plus
all of P1–P3's), and export `DATABASE_URL=postgres://…` before the gate runs. If no CI Postgres is
available, RAISE it before executing. The `test -n "$DATABASE_URL" &&` prefix in `## Done when` makes the
gate FAIL-CLOSED (tournament R1/F2).

**Rebuilt-schema interpretation (RATIFICATION REQUESTED).** ADR-0011 says "migrate … into the **rebuilt
hive**" and lists REGENERABLE as "drop and rebuild from node re-ingest." Two verified facts pin it:
1. **No task in this workstream rebuilds the schema.** P1/P2 add only ADDITIVE migrations (`node_op_log`
   table, `node_task_assignments` lease columns); nothing rebuilds `shared_tasks` or any MUST-MIGRATE
   table, and the hive `task_status` enum is ALREADY canonical kebab-case
   (`crates/remote/migrations/20251001000000_shared_tasks_activity.sql:57`). So the rebuild is **IN-PLACE**
   — MUST-MIGRATE tables stay where they are (id bridge `source_task_id`/`source_node_id` already intact
   as columns — `20260105120000_add_source_task_id.sql`).
2. **The cutover must be a DATA operation (TRUNCATE/DELETE), NOT a schema teardown.** VERIFIED by grep:
   EVERY REGENERABLE/DISCARDABLE table still has surviving `query!`/`query_as!` references in
   `crates/remote/src` (e.g. `node_task_attempts` 13, `node_local_projects` 22, `activity` in
   `db/activity.rs`+`db/tasks.rs`, `auth_sessions` 8). NONE of that code is removed by THIS workstream
   (the code-removal lives in P4/P5). An in-place teardown would therefore (a) break `cargo check -p
   remote` online query validation against the post-cutover schema, and (b) leave no table for the node
   re-ingest path (`handle_attempt_sync`, `node_task_attempts.rs:52`, an `INSERT INTO` — NOT a
   `CREATE TABLE`) to repopulate (703). So REGENERABLE = **TRUNCATE the data, keep the schema**;
   DISCARDABLE-with-surviving-code = **TRUNCATE the data, keep the schema**; completed
   `node_task_assignments` rows = **DELETE** (table is MUST-MIGRATE for active rows).

**The destructive alternative — teardown+recreate `shared_tasks`, copy MUST-MIGRATE data across a fresh
empty Postgres schema — would be INVENTED and dangerous; NOT authored. If the orchestrator/user wants a
fresh-schema copy instead, STOP and re-author 701/702/703 against that model.** Do NOT proceed on a
destructive teardown of MUST-MIGRATE data without ratification.

**FROZEN-SPEC COLLISION — NAME IT, DON'T REWORD IT (TS6 literal vs in-place reading; RATIFICATION
REQUESTED).** Spec TS6 reads: "…**discardable tables are absent** — nothing in the inventory is silently
lost." Under the in-place reading this is NOT literally delivered: DISCARDABLE tables (`activity`,
`auth_sessions`, `oauth_handoffs`, `revoked_refresh_tokens`) are kept-but-EMPTIED, not ABSENT, because
their `query!` refs in `crates/remote/src` are removed only in P4/P5 — and **P7 depends on P1–P3, not
P4/P5**, so the code that would make a teardown safe has not landed when P7 runs. This is a genuine
spec-vs-reality contradiction (tournament fidelity axis 9 / Trap 6), surfaced explicitly rather than
absorbed by quietly changing the assertion. **Orchestrator/user must ratify ONE of:** (a) accept
keep-but-empty as satisfying TS6's intent ("nothing silently lost") with a spec note, OR (b) sequence a
DROP-tables migration AFTER the P4/P5 code-removal phase (P7 then owns only the data-clear; the teardown
becomes a separately-sequenced task). 701's test asserts keep-but-empty (option a); if (b) is chosen, a
DROP-tables task is added and 701's `cutover_keeps_..._present` guard is relaxed for those tables.

## Failing test (write first)
Create `crates/remote/tests/hive_cutover_migration.rs`. **Sibling read (rubric #9):** there is NO shared
`common` module (`crates/remote/tests/` holds only `backfill_e2e.rs` + `pool_config.rs` + the P1
`node_op_log_migration.rs`) — inline backfill_e2e's exact helpers verbatim: `fn database_url() ->
Option<String>` (`std::env::var("DATABASE_URL").ok()`), the `skip_without_db!` macro, and `async fn
create_pool() -> PgPool` (`PgPool::connect(&url)`).

**CRITICAL — NOT A HOLLOW TEST (tournament axis 7).** The cutover SQL runs during `sqlx::migrate!` on
an EMPTY fresh Postgres, so a test that merely connects and asserts `row_count == 0` would PASS even with
an empty migration body (nothing was ever inserted) — hollow. This test therefore uses the data-migration
shape **seed → re-run the cutover SQL → assert retained-vs-cleared**: it SEEDs representative rows into a
regenerable table, a discardable table, AND must-migrate tables (incl. an ACTIVE and a COMPLETED
`node_task_assignments` row), then EXECUTEs the exact cutover statements (inlined as `CUTOVER_SQL`,
copy-identical to the migration body), then asserts the regenerable/discardable rows are GONE, the
must-migrate rows (incl. the active assignment) are RETAINED, and the completed assignment is GONE. This
is the only shape that catches (a) a `TRUNCATE … CASCADE` silently reaching a must-migrate table and (b)
an over-broad `DELETE FROM node_task_assignments` wiping active rows.

```rust
//! SC6 / TS6 — seed → run the cutover SQL → assert: REGENERABLE + DISCARDABLE rows are cleared (schema
//! kept), MUST-MIGRATE rows (incl. the active assignment) survive, completed assignments are purged.
//! (702 round-trips the id-bridge + status; 703 the regenerable re-ingest into the surviving tables.)
use sqlx::PgPool;

// Inlined verbatim from crates/remote/tests/backfill_e2e.rs (no shared `common` module exists).
fn database_url() -> Option<String> { std::env::var("DATABASE_URL").ok() }
macro_rules! skip_without_db { () => {
    if database_url().is_none() { eprintln!("Skipping: DATABASE_URL not set"); return; }
}; }
async fn create_pool() -> PgPool {
    PgPool::connect(&database_url().unwrap()).await.expect("connect")
}

// EXACT cutover statements — copy-identical to the migration body in `## Change` (keep in sync; the
// STOP trigger flags drift). Executed here against SEEDED rows so the assertions are non-hollow.
const CUTOVER_SQL: &str = r#"
TRUNCATE TABLE node_execution_processes, node_task_output_logs, node_task_progress_events,
               node_task_attempts, node_local_projects, project_activity_counters;
TRUNCATE TABLE activity, auth_sessions, oauth_handoffs, revoked_refresh_tokens;
DELETE FROM node_task_assignments WHERE completed_at IS NOT NULL;
"#;

async fn count_where(pool: &PgPool, sql: &str) -> i64 {
    sqlx::query_scalar::<_, i64>(sql).fetch_one(pool).await.unwrap()
}

#[tokio::test]
async fn cutover_seed_run_clears_regenerable_discardable_keeps_must_migrate() {
    skip_without_db!(); // Trap 2b: a real migrated PG MUST be set or this is a hollow pass
    let pool = create_pool().await;
    let mut tx = pool.begin().await.unwrap(); // run in a tx; roll back so the shared DB stays clean

    // --- SEED MUST-MIGRATE parents + an active and a completed assignment, and one node_task_attempt
    //     (REGENERABLE) bridged to a shared_task. Column lists: confirm against the cited migrations
    //     (STOP trigger). Minimal NOT-NULL sets shown; executor finalizes exact columns. ---
    let org = uuid::Uuid::new_v4();
    sqlx::query("INSERT INTO organizations (id, name) VALUES ($1, 'cut')").bind(org)
        .execute(&mut *tx).await.unwrap();
    let node = uuid::Uuid::new_v4();
    sqlx::query("INSERT INTO nodes (id, organization_id, name, machine_id) VALUES ($1,$2,'n',$3)")
        .bind(node).bind(org).bind(uuid::Uuid::new_v4().to_string()).execute(&mut *tx).await.unwrap();
    let sp = uuid::Uuid::new_v4();
    sqlx::query("INSERT INTO swarm_projects (id, organization_id, name) VALUES ($1,$2,'p')")
        .bind(sp).bind(org).execute(&mut *tx).await.unwrap();
    let shared = uuid::Uuid::new_v4();
    sqlx::query("INSERT INTO shared_tasks (id, organization_id, swarm_project_id, title, status) \
                 VALUES ($1,$2,$3,'t','in-review'::task_status)")  // MUST-MIGRATE row — must survive
        .bind(shared).bind(org).bind(sp).execute(&mut *tx).await.unwrap();

    // node_task_assignments: one ACTIVE (completed_at NULL → MUST-MIGRATE) + one COMPLETED (DISCARDABLE).
    // Confirm assignment columns against 20251202000000_nodes_swarm.sql (STOP trigger).
    let active_asg = uuid::Uuid::new_v4();
    sqlx::query("INSERT INTO node_task_assignments (id, node_id, shared_task_id, completed_at) \
                 VALUES ($1,$2,$3, NULL)").bind(active_asg).bind(node).bind(shared)
        .execute(&mut *tx).await.unwrap();
    let done_asg = uuid::Uuid::new_v4();
    sqlx::query("INSERT INTO node_task_assignments (id, node_id, shared_task_id, completed_at) \
                 VALUES ($1,$2,$3, now())").bind(done_asg).bind(node).bind(shared)
        .execute(&mut *tx).await.unwrap();

    // REGENERABLE node_task_attempt bridged to the MUST-MIGRATE shared_task (tests CASCADE does NOT
    // reach shared_tasks: the FK points attempt→shared_task, so truncating attempts must not delete it).
    let attempt = uuid::Uuid::new_v4();
    sqlx::query("INSERT INTO node_task_attempts \
                 (id, shared_task_id, node_id, executor, branch, target_branch, worktree_deleted, \
                  created_at, updated_at) \
                 VALUES ($1,$2,$3,'qa_mock','b','main',false, now(), now())")
        .bind(attempt).bind(shared).bind(node).execute(&mut *tx).await.unwrap();

    // DISCARDABLE: an auth_sessions row (confirm columns against 20251001000000_shared_tasks_activity.sql).
    sqlx::query("INSERT INTO auth_sessions (id, user_id) VALUES ($1, NULL)")
        .bind(uuid::Uuid::new_v4()).execute(&mut *tx).await.unwrap_or_default();

    // --- RUN the cutover SQL against the seeded rows ---
    for stmt in CUTOVER_SQL.split(';').map(str::trim).filter(|s| !s.is_empty()) {
        sqlx::query(stmt).execute(&mut *tx).await.unwrap();
    }

    // --- ASSERT: regenerable/discardable rows gone; must-migrate (incl. active assignment) retained ---
    assert_eq!(count_where(&mut *tx,
        &format!("SELECT COUNT(*) FROM node_task_attempts WHERE id = '{attempt}'")).await, 0,
        "REGENERABLE node_task_attempts row must be cleared");
    assert_eq!(count_where(&mut *tx,
        "SELECT COUNT(*) FROM auth_sessions").await, 0, "DISCARDABLE auth_sessions must be cleared");
    // The bridged MUST-MIGRATE shared_task must NOT have cascaded away when attempts were truncated.
    assert_eq!(count_where(&mut *tx,
        &format!("SELECT COUNT(*) FROM shared_tasks WHERE id = '{shared}'")).await, 1,
        "MUST-MIGRATE shared_tasks row must survive (TRUNCATE must NOT cascade into it)");
    // Active assignment kept; completed assignment purged (the DELETE WHERE is load-bearing).
    assert_eq!(count_where(&mut *tx,
        &format!("SELECT COUNT(*) FROM node_task_assignments WHERE id = '{active_asg}'")).await, 1,
        "ACTIVE node_task_assignment (completed_at NULL) must be RETAINED");
    assert_eq!(count_where(&mut *tx,
        &format!("SELECT COUNT(*) FROM node_task_assignments WHERE id = '{done_asg}'")).await, 0,
        "COMPLETED node_task_assignment must be PURGED");

    tx.rollback().await.unwrap(); // leave the shared test DB untouched
}

#[tokio::test]
async fn cutover_keeps_regenerable_and_discardable_tables_present() {
    // Regression guard: the cutover must NOT DROP these tables (data-clear, not drop) — a DROP would
    // break surviving query refs + the re-ingest path. This half is non-hollow on its own.
    skip_without_db!();
    let pool = create_pool().await;
    let exists = |n: &'static str| {
        let pool = pool.clone();
        async move {
            sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS (SELECT 1 FROM information_schema.tables \
                 WHERE table_schema='public' AND table_name=$1)")
                .bind(n).fetch_one(&pool).await.unwrap()
        }
    };
    for t in ["node_local_projects", "node_execution_processes", "node_task_output_logs",
              "node_task_progress_events", "node_task_attempts", "project_activity_counters",
              "activity", "auth_sessions", "oauth_handoffs", "revoked_refresh_tokens"] {
        assert!(exists(t).await, "table {t} must SURVIVE cutover (data-clear, not drop)");
    }
}
```

## Change
- **File:** `crates/remote/migrations/20260201000000_hive_cutover_clear_regenerable_discardable.sql` (NEW)
- **Anchor:** new Postgres migration; timestamp must sort AFTER every prior migration. Latest on main is
  `20260127000000_add_backfill_request_id.sql`; P1 adds `20260128000000_add_node_op_log.sql` (task 102)
  and P2 adds lease columns; `20260201000000` sorts strictly last (STOP-check below).
- **Sibling read (rubric #9):** `crates/remote/migrations/20260124100000_remove_legacy_projects.sql` and
  `20260124200000_remove_node_projects.sql` are the existing data-cleanup migration siblings — but they
  DROP tables whose code was removed in the same wave. Here the code is NOT removed (P4/P5 own that), so
  this migration follows the `DELETE FROM` / `TRUNCATE` data-clear shape instead, forward-only. VERIFIED:
  `project_activity_counters` was dropped then RECREATED in `20260124100000` (so it currently EXISTS and
  is cleared, not dropped).
- **Before:** (file does not exist)
- **After:** exact contents:
```sql
-- SC6 / ADR-0011 — one-time hive-only-state cutover (DATA-CLEAR leg). IRREVERSIBLE (data loss). Gate
-- behind a pre-cutover backup (ADR-0011 Consequences). Forward-only.
--
-- Interpretation (plan.md Phase 7 note; ratified judgment call): the rebuild is IN-PLACE and the cutover
-- is a DATA operation, not a schema teardown. Every REGENERABLE/DISCARDABLE table still has surviving
-- query references in crates/remote/src (NOT removed by this workstream — that is P4/P5), and the node
-- re-ingest path INSERTs into the existing table rather than recreating it. So we TRUNCATE the data and
-- KEEP the schema. MUST-MIGRATE tables (shared_tasks incl. the source_task_id/source_node_id id bridge,
-- node_api_keys, nodes, swarm_projects/_nodes, swarm_templates, labels/shared_task_labels,
-- identity/tenancy) are NOT touched here; their preservation is asserted by 702.

-- REGENERABLE — node-mirror caches / logs / sync bookkeeping; data rebuilt by node re-ingest (ADR-0008
-- outbox + the existing handle_attempt_sync path). Clear data, keep schema. These are listed TOGETHER in
-- one TRUNCATE so their intra-set FKs (node_execution_processes.attempt_id → node_task_attempts;
-- *_logs/_events.assignment_id → node_task_assignments stays UNtruncated) are satisfied WITHOUT CASCADE —
-- CASCADE is deliberately OMITTED so the operation can NEVER silently reach a MUST-MIGRATE table (the
-- seed test asserts shared_tasks survives). No RESTART IDENTITY: these are UUID-PK tables, no serials.
-- (sync_state / backfill_request_id / last_full_sync_at are COLUMNS on node_task_attempts — cleared with
-- its rows.) NOTE: node_task_output_logs/_events FK node_task_assignments(id) ON DELETE CASCADE, but
-- assignments are not TRUNCATEd here (only DELETEd by WHERE below), so no cross-set dependency.
TRUNCATE TABLE node_execution_processes, node_task_output_logs, node_task_progress_events,
               node_task_attempts, node_local_projects, project_activity_counters;

-- DISCARDABLE — not migrated (ADR-0011). The tables stay (kept auth/activity code references them — the
-- code removal is out of this workstream's scope); their history is cleared. activity is partitioned;
-- TRUNCATE empties all partitions. No CASCADE / no RESTART IDENTITY: these are leaf, UUID-PK tables.
TRUNCATE TABLE activity, auth_sessions, oauth_handoffs, revoked_refresh_tokens;

-- DISCARDABLE rows inside a MUST-MIGRATE table: completed assignments (ADR-0011). Keep active
-- (completed_at IS NULL) — the only record of which node owns which in-flight task.
DELETE FROM node_task_assignments WHERE completed_at IS NOT NULL;
```

## Allowed moves
ONLY create the one migration file (exact SQL above) and the one `tests/hive_cutover_migration.rs` test.
Do NOT TRUNCATE/DROP/alter any MUST-MIGRATE table, do NOT touch `shared_tasks`, do NOT DROP any table
(data-clear only — DROP breaks the surviving query refs + the re-ingest path), do NOT add a Rust
db-model module or edit `crates/remote/src/db/`, do NOT touch the WS protocol. The status remap +
id-bridge GUARD is 702; the regenerable re-ingest round-trip is 703.

## STOP triggers
- A PG migration with a timestamp ≥ `20260201000000` already exists (e.g. a P2/P3 migration sorted
  later than expected) → bump THIS file to sort strictly last and update `files:` (record the new name).
- **Rebuilt-schema interpretation is contested** — if the orchestrator/user decides the cutover should
  copy MUST-MIGRATE data into a NEW/empty Postgres schema rather than evolve in place → STOP and
  re-author 701/702/703 against that model. Do NOT improvise a destructive `shared_tasks` drop.
- A plain `TRUNCATE` ERRORS because some NON-truncated table has an inbound FK into one of these → this
  is the SAFE failure mode (CASCADE is deliberately omitted so the op can never silently delete
  must-migrate data). STOP; investigate the FK — do NOT just add `CASCADE` (that is exactly the silent
  must-migrate-deletion danger the seed test guards against). Resolve by either truncating that table too
  (if it is itself REGENERABLE/DISCARDABLE) or escalating.
- **`CUTOVER_SQL` in the test drifts from the migration body** → they MUST stay copy-identical (the test
  re-runs the migration's statements against seeded rows; that is what makes it non-hollow). If you edit
  one, edit the other in the same change.
- A grep shows a REGENERABLE/DISCARDABLE table's code WAS removed elsewhere on the branch (so a true
  DROP would now be safe) → STILL prefer data-clear here unless the orchestrator ratifies the DROP; a
  DROP is irreversible schema change beyond this task's scope.
- `cargo sqlx prepare` is tempting because a later query task fails offline validation → DO NOT run it
  here (Trap 2). This task adds NO `query!`; only raw `sqlx::query(...)` in the test.

## Done when
`WAI_TYPECHECK_CMD="cargo check -p remote" WAI_TEST_CMD='test -n "$DATABASE_URL" && cargo test -p remote --test hive_cutover_migration' bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-hive-redesign 701` exits 0
(run with `DATABASE_URL=postgres://…` pointed at a Postgres that has had `./migrations` applied — Trap 2b.
**The `test -n "$DATABASE_URL" &&` prefix makes the gate FAIL-CLOSED:** with no `DATABASE_URL` the
`test -n` fails, the `&&` short-circuits, and the gate fails — instead of `skip_without_db!` reporting a
skipped test as a hollow green. Because `irreversible: true`, the executor must record a human approval
token (`reviews/701.approved`) before the gate runs.)
