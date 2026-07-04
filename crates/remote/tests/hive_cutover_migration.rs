//! SC6 / TS6 — seed → run the cutover SQL → assert: REGENERABLE + DISCARDABLE rows are cleared (schema
//! kept), MUST-MIGRATE rows (incl. the active assignment) survive, completed assignments are purged.
//! (702 round-trips the id-bridge + status; 703 the regenerable re-ingest into the surviving tables.)
//!
//! Both tests use `file_serial` because they run `TRUNCATE TABLE` (acquiring `ACCESS EXCLUSIVE`
//! locks) on tables that other test binaries (backfill_e2e) access concurrently. File-based
//! serialization prevents cross-binary lock conflicts on the shared Postgres test DB.
use serial_test::file_serial;
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

async fn count_where<'e, E>(pool: E, sql: &str) -> i64
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
{
    sqlx::query_scalar::<_, i64>(sql).fetch_one(pool).await.unwrap()
}

#[tokio::test]
#[file_serial]
async fn cutover_seed_run_clears_regenerable_discardable_keeps_must_migrate() {
    skip_without_db!(); // Trap 2b: a real migrated PG MUST be set or this is a hollow pass
    let pool = create_pool().await;
    let mut tx = pool.begin().await.unwrap(); // run in a tx; roll back so the shared DB stays clean

    // --- SEED MUST-MIGRATE parents + an active and a completed assignment, and one node_task_attempt
    //     (REGENERABLE) bridged to a shared_task. Column lists: confirm against the cited migrations
    //     (STOP trigger). Minimal NOT-NULL sets shown; executor finalizes exact columns. ---
    let org = uuid::Uuid::new_v4();
    sqlx::query("INSERT INTO organizations (id, name, slug) VALUES ($1, 'cut', $2)")
        .bind(org).bind(uuid::Uuid::new_v4().to_string()).execute(&mut *tx).await.unwrap();
    let node = uuid::Uuid::new_v4();
    sqlx::query("INSERT INTO nodes (id, organization_id, name, machine_id) VALUES ($1,$2,'n',$3)")
        .bind(node).bind(org).bind(uuid::Uuid::new_v4().to_string()).execute(&mut *tx).await.unwrap();
    let sp = uuid::Uuid::new_v4();
    sqlx::query("INSERT INTO swarm_projects (id, organization_id, name) VALUES ($1,$2,'p')")
        .bind(sp).bind(org).execute(&mut *tx).await.unwrap();
    // swarm_project_nodes parent (node_task_assignments.node_project_id is NOT NULL FK→swarm_project_nodes)
    let spn = uuid::Uuid::new_v4();
    sqlx::query("INSERT INTO swarm_project_nodes (id, swarm_project_id, node_id, local_project_id, \
                 git_repo_path) VALUES ($1,$2,$3,$4,'r')")
        .bind(spn).bind(sp).bind(node).bind(uuid::Uuid::new_v4()).execute(&mut *tx).await.unwrap();
    let shared = uuid::Uuid::new_v4();
    sqlx::query("INSERT INTO shared_tasks (id, organization_id, swarm_project_id, title, status) \
                 VALUES ($1,$2,$3,'t','in-review'::task_status)")  // MUST-MIGRATE row — must survive
        .bind(shared).bind(org).bind(sp).execute(&mut *tx).await.unwrap();

    // node_task_assignments: one ACTIVE (completed_at NULL → MUST-MIGRATE) + one COMPLETED (DISCARDABLE).
    // Confirm assignment columns against 20251202000000_nodes_swarm.sql (STOP trigger).
    let active_asg = uuid::Uuid::new_v4();
    sqlx::query("INSERT INTO node_task_assignments (id, task_id, node_id, node_project_id, completed_at) \
                 VALUES ($1,$2,$3,$4, NULL)").bind(active_asg).bind(shared).bind(node).bind(spn)
        .execute(&mut *tx).await.unwrap();
    let done_asg = uuid::Uuid::new_v4();
    sqlx::query("INSERT INTO node_task_assignments (id, task_id, node_id, node_project_id, completed_at) \
                 VALUES ($1,$2,$3,$4, now())").bind(done_asg).bind(shared).bind(node).bind(spn)
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
    // Fail-fast: use `.unwrap()` so a seed failure errors the transaction immediately (do NOT swallow
    // via `.unwrap_or_default()` — that would mask a missing-column / FK failure and produce a hollow pass).
    let user_id = uuid::Uuid::new_v4();
    sqlx::query("INSERT INTO users (id, email) VALUES ($1, $2)")
        .bind(user_id).bind(format!("u{}@cut.test", uuid::Uuid::new_v4())).execute(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO auth_sessions (id, user_id) VALUES ($1, $2)")
        .bind(uuid::Uuid::new_v4()).bind(user_id).execute(&mut *tx).await.unwrap();

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
#[file_serial]
async fn cutover_keeps_regenerable_and_discardable_tables_present() {
    // Regression guard: the cutover must NOT DROP these tables (data-clear, not drop) — a DROP would
    // break surviving query refs + the re-ingest path. Asserts a STABLE IDENTITY (table OID), not just
    // the table name: a DROP+CREATE would produce a new table with the same NAME but a different OID
    // (and a different relfilenode), so this test FAILS if the migration performs a DROP instead of a
    // data-clear. Uses the same tx-based seed/run/rollback pattern as the test above so the OID is
    // captured BEFORE the cutover SQL runs and re-checked AFTER.
    skip_without_db!();
    let pool = create_pool().await;
    let mut tx = pool.begin().await.unwrap();

    let tables = ["node_local_projects", "node_execution_processes", "node_task_output_logs",
                  "node_task_progress_events", "node_task_attempts", "project_activity_counters",
                  "activity", "auth_sessions", "oauth_handoffs", "revoked_refresh_tokens"];

    // Capture the OID of each table BEFORE the cutover SQL runs.
    let mut before: std::collections::HashMap<&str, i64> = std::collections::HashMap::new();
    for t in tables {
        let oid: i64 = sqlx::query_scalar(
            "SELECT c.oid::bigint FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace \
             WHERE c.relname = $1 AND n.nspname = 'public'")
            .bind(t).fetch_one(&mut *tx).await.unwrap();
        before.insert(t, oid);
    }

    // Run the cutover SQL (copy-identical to the migration body + the test above).
    for stmt in CUTOVER_SQL.split(';').map(str::trim).filter(|s| !s.is_empty()) {
        sqlx::query(stmt).execute(&mut *tx).await.unwrap();
    }

    // Re-read the OID AFTER the cutover: a DROP+CREATE would change it; a TRUNCATE keeps it.
    for t in tables {
        let oid_after: i64 = sqlx::query_scalar(
            "SELECT c.oid::bigint FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace \
             WHERE c.relname = $1 AND n.nspname = 'public'")
            .bind(t).fetch_one(&mut *tx).await.unwrap();
        assert_eq!(oid_after, before[t],
            "table {t} OID changed — cutover performed a DROP+CREATE, not a data-clear (TRUNCATE). \
             A DROP breaks surviving query refs + the re-ingest path.");
        // Also assert the table still EXISTS by name (belt-and-suspenders; the OID read above already
        // proves existence, but the explicit name check makes the failure message self-explanatory).
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM information_schema.tables \
             WHERE table_schema='public' AND table_name=$1)")
            .bind(t).fetch_one(&mut *tx).await.unwrap();
        assert!(exists, "table {t} must SURVIVE cutover (data-clear, not drop)");
    }

    tx.rollback().await.unwrap(); // leave the shared test DB untouched
}