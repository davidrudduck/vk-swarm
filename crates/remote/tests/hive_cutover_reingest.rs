//! SC6 / TS6 — REGENERABLE re-ingest: after cutover (data cleared, schema kept), a simulated node
//! re-ingest repopulates node_task_attempts, with the id bridge to shared_tasks intact.
//! NOTE: drives the EXISTING `INSERT INTO node_task_attempts (...) ON CONFLICT …` upsert shape
//! (what `NodeTaskAttemptRepository::upsert` / `handle_attempt_sync` uses today) via raw SQL so the
//! whole test runs inside one rollback-able transaction — NOT the new ADR-0008 op-log. This proves
//! the schema is refillable post-cutover, not the op-log.
//!
//! These tests require a PostgreSQL database available at DATABASE_URL with the migrations applied.
//! Tests are skipped if DATABASE_URL is not set (the gate's `test -n "$DATABASE_URL" &&` prefix
//! fails-closed so a hollow skip never becomes a green).

use sqlx::PgPool;

/// Helper to check if database is available.
fn database_url() -> Option<String> {
    std::env::var("DATABASE_URL").ok()
}

/// Skip test if database is not available.
macro_rules! skip_without_db {
    () => {
        if database_url().is_none() {
            eprintln!("Skipping test: DATABASE_URL not set");
            return;
        }
    };
}

/// Create a test database connection pool.
async fn create_pool() -> PgPool {
    let url = database_url().expect("DATABASE_URL must be set");
    PgPool::connect(&url)
        .await
        .expect("Failed to connect to database")
}

#[tokio::test]
async fn regenerable_node_attempt_repopulates_from_reingest() {
    skip_without_db!(); // Trap 2b: a real migrated PG MUST be set or this is a hollow pass
    let pool = create_pool().await;
    // Wrap the entire setup + assertions in a transaction and roll it back at the end (matching 701's
    // pattern) so the shared test DB is left untouched — no persistent rows survive the test run.
    let mut tx = pool.begin().await.unwrap();

    // Seed MUST-MIGRATE parents (preserved across cutover) so the re-ingested attempt has a real
    // shared_task_id bridge. Column lists: confirmed against the cited migrations (STOP trigger).
    // Correction vs. task-file draft: `organizations.slug` is NOT NULL UNIQUE
    // (20251001000000_shared_tasks_activity.sql:16) — added to the seed INSERT (701 made the same
    // correction; recorded in this task's decisions-ledger line).
    let org_id = uuid::Uuid::new_v4();
    sqlx::query("INSERT INTO organizations (id, name, slug) VALUES ($1, 'reingest-test', $2)")
        .bind(org_id)
        .bind(uuid::Uuid::new_v4().to_string())
        .execute(&mut *tx)
        .await
        .unwrap();
    let node_id = uuid::Uuid::new_v4();
    sqlx::query("INSERT INTO nodes (id, organization_id, name, machine_id) \
                  VALUES ($1, $2, 'n1', $3)")
        .bind(node_id)
        .bind(org_id)
        .bind(uuid::Uuid::new_v4().to_string())
        .execute(&mut *tx)
        .await
        .unwrap();
    let swarm_project_id = uuid::Uuid::new_v4();
    sqlx::query("INSERT INTO swarm_projects (id, organization_id, name) VALUES ($1, $2, 'p1')")
        .bind(swarm_project_id)
        .bind(org_id)
        .execute(&mut *tx)
        .await
        .unwrap();
    let shared_task_id = uuid::Uuid::new_v4();
    sqlx::query(
        "INSERT INTO shared_tasks (id, organization_id, swarm_project_id, title, status) \
          VALUES ($1, $2, $3, 'reingested', 'in-review'::task_status)",
    )
    .bind(shared_task_id)
    .bind(org_id)
    .bind(swarm_project_id)
    .execute(&mut *tx)
    .await
    .unwrap();

    // The REGENERABLE table is empty for this task post-cutover (701 cleared it).
    let before: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM node_task_attempts WHERE shared_task_id = $1",
    )
    .bind(shared_task_id)
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    assert_eq!(
        before, 0,
        "REGENERABLE node_task_attempts must start empty post-cutover for this task"
    );

    // Simulate a node re-ingest via the EXISTING upsert path (handle_attempt_sync's mechanism).
    // Raw SQL mirrors `NodeTaskAttemptRepository::upsert` (crates/remote/src/db/node_task_attempts.rs:46)
    // — the `INSERT INTO node_task_attempts (...) ON CONFLICT (id) DO UPDATE` shape — so the whole
    // upsert runs inside the rollback-able tx. The column list is the minimal NOT-NULL set read from
    // node_task_attempts.rs:52 (sync_state defaults to 'partial'; assignment_id, executor_variant,
    // container_ref, setup_completed_at are nullable).
    let now = chrono::Utc::now();
    let attempt_id = uuid::Uuid::new_v4();
    sqlx::query(
        "INSERT INTO node_task_attempts \
          (id, shared_task_id, node_id, executor, branch, target_branch, worktree_deleted, \
           created_at, updated_at) \
          VALUES ($1, $2, $3, 'qa_mock', 'vk/reingest', 'main', false, $4, $5) \
          ON CONFLICT (id) DO UPDATE SET updated_at = EXCLUDED.updated_at",
    )
    .bind(attempt_id)
    .bind(shared_task_id)
    .bind(node_id)
    .bind(now)
    .bind(now)
    .execute(&mut *tx)
    .await
    .expect("re-ingest upsert");

    // The REGENERABLE row reappeared, linked to the preserved shared task.
    let after: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM node_task_attempts WHERE shared_task_id = $1 AND id = $2",
    )
    .bind(shared_task_id)
    .bind(attempt_id)
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    assert_eq!(
        after, 1,
        "REGENERABLE node_task_attempts must repopulate from re-ingest"
    );

    tx.rollback().await.unwrap(); // leave the shared test DB untouched — no persistent rows
}