//! SC6 / TS6 — MUST-MIGRATE round-trip after cutover: id bridge intact, status canonical.
//!
//! Uses `file_serial` because it touches `activity` (a TRUNCATE target in hive_cutover_migration's
//! CUTOVER_SQL). File-based serialization prevents cross-binary lock conflicts on the shared
//! Postgres test DB.
use serial_test::file_serial;
use sqlx::PgPool;

fn database_url() -> Option<String> {
    std::env::var("DATABASE_URL").ok()
}
macro_rules! skip_without_db {
    () => {
        if database_url().is_none() {
            eprintln!("Skipping: DATABASE_URL not set");
            return;
        }
    };
}
async fn create_pool() -> PgPool {
    PgPool::connect(&database_url().unwrap()).await.expect("connect")
}

#[file_serial]
#[tokio::test]
async fn must_migrate_id_bridge_and_status_round_trip() {
    skip_without_db!(); // Trap 2b: a real migrated PG MUST be set or this is a hollow pass
    let pool = create_pool().await;
    // Wrap the entire test body in a transaction and roll it back at the end (matching 701's pattern)
    // so the shared test DB is left untouched — no persistent rows survive the test run.
    let mut tx = pool.begin().await.unwrap();

    // The canonical hive wire value for the node 'inprogress' state is kebab 'in-progress'
    // (CONTRACT.md §D; ADR-0010). Prove the enum accepts the canonical form and rejects the
    // node-lowercase form — i.e. the value space is canonical at rest, never the node form.
    let kebab_ok: bool = sqlx::query_scalar(
        "SELECT 'in-progress'::task_status = 'in-progress'::task_status")
        .fetch_one(&mut *tx).await.unwrap();
    assert!(kebab_ok, "hive task_status must accept canonical kebab 'in-progress'");
    // The rejected cast aborts the surrounding transaction, so isolate it in a savepoint that
    // we roll back, leaving the outer tx usable for the round-trip seed below.
    sqlx::query("SAVEPOINT sp_lower").execute(&mut *tx).await.unwrap();
    let lower_rejected = sqlx::query_scalar::<_, bool>("SELECT 'inprogress'::task_status IS NOT NULL")
        .fetch_one(&mut *tx).await;
    assert!(lower_rejected.is_err(), "node-lowercase 'inprogress' must NOT be a valid hive task_status \
            value (remap happens at ingest, not at rest)");
    sqlx::query("ROLLBACK TO SAVEPOINT sp_lower").execute(&mut *tx).await.unwrap();

    // Round-trip a MUST-MIGRATE shared_tasks row with the id bridge set, mirroring upsert_from_node.
    // Seed the minimal MUST-MIGRATE parents (org, swarm_project, node) so the FK/bridge is real.
    let org_id = uuid::Uuid::new_v4();
    // organizations.slug is NOT NULL UNIQUE (20251001000000_shared_tasks_activity.sql:16) — seed it.
    sqlx::query("INSERT INTO organizations (id, name, slug) VALUES ($1, 'cutover-test', $2)")
        .bind(org_id).bind(org_id.to_string())
        .execute(&mut *tx).await.unwrap();
    let node_id = uuid::Uuid::new_v4();
    // node insert columns: nodes.machine_id is NOT NULL (20251202000000_nodes_swarm.sql).
    sqlx::query("INSERT INTO nodes (id, organization_id, name, machine_id) \
                 VALUES ($1, $2, 'n1', $3)")
        .bind(node_id).bind(org_id).bind(uuid::Uuid::new_v4().to_string())
        .execute(&mut *tx).await.unwrap();
    let swarm_project_id = uuid::Uuid::new_v4();
    sqlx::query("INSERT INTO swarm_projects (id, organization_id, name) VALUES ($1, $2, 'p1')")
        .bind(swarm_project_id).bind(org_id).execute(&mut *tx).await.unwrap();

    // The id BRIDGE: source_task_id = the node-local tasks.id; source_node_id = the node.
    let source_task_id = uuid::Uuid::new_v4();
    let shared_id = uuid::Uuid::new_v4();
    sqlx::query(
        "INSERT INTO shared_tasks \
           (id, organization_id, swarm_project_id, source_node_id, source_task_id, title, status) \
         VALUES ($1, $2, $3, $4, $5, 'bridged', 'in-progress'::task_status)")
        .bind(shared_id).bind(org_id).bind(swarm_project_id)
        .bind(node_id).bind(source_task_id)
        .execute(&mut *tx).await.unwrap();

    // Round-trip: the bridge resolves the shared task back from (source_node_id, source_task_id),
    // and the status survives as the canonical kebab form.
    let (got_id, got_status): (uuid::Uuid, String) = sqlx::query_as(
        "SELECT id, status::text FROM shared_tasks \
         WHERE source_node_id = $1 AND source_task_id = $2")
        .bind(node_id).bind(source_task_id).fetch_one(&mut *tx).await.unwrap();
    assert_eq!(got_id, shared_id, "id bridge must resolve the shared task by (source_node_id, source_task_id)");
    assert_eq!(got_status, "in-progress", "status must round-trip as canonical kebab form");

    // Node-side bridge guard (tournament R1/B): the existing test verified the HIVE-side lookup
    // (resolve shared_tasks by source_node_id + source_task_id). This step verifies the REVERSE
    // direction — that the node-local `tasks.shared_task_id` maps BACK to the shared task. The
    // hive Postgres schema has no node-local `tasks` table (that lives in node SQLite), so we
    // assert the bridge is queryable in the reverse direction via `shared_tasks.id` →
    // `(source_node_id, source_task_id)`: given the shared id the node would store in
    // `tasks.shared_task_id`, the bridge key resolves back to the originating node + local id.
    let (rev_node, rev_local): (uuid::Uuid, uuid::Uuid) = sqlx::query_as(
        "SELECT source_node_id, source_task_id FROM shared_tasks WHERE id = $1")
        .bind(shared_id).fetch_one(&mut *tx).await.unwrap();
    assert_eq!(rev_node, node_id,
        "reverse bridge: shared_tasks.id must resolve back to source_node_id (the node that owns the local task)");
    assert_eq!(rev_local, source_task_id,
        "reverse bridge: shared_tasks.id must resolve back to source_task_id (the node-local tasks.id)");

    tx.rollback().await.unwrap(); // leave the shared test DB untouched — no persistent rows
}