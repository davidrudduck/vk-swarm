//! TS2 (SC3): a partition cannot cause double execution — stale-token commit is rejected.
//!
//! This is the SC3 acceptance test. TS2 is proven as THREE coordinated legs (NOT a mocked
//! end-to-end), recorded in `docs/plans/vk-swarm-hive-redesign/decisions-ledger.md` under
//! Task 210:
//!
//! 1. **Hive stale-token rejection (the at-most-once commit EFFECT)** — proven by 205's
//!    in-module `#[cfg(test)]` test `op_against_assigned_task_with_stale_token_is_rejected_not_applied`
//!    in `crates/remote/src/nodes/ws/session.rs`, which calls the private `handle_op_batch`
//!    and asserts a stale-token op is rejected (no apply, no advance, `LeaseRevoked` surfaced).
//!    This `tests/` file does NOT re-assert the reject: `handle_op_batch` is private (R2/F8 —
//!    making it `pub` would change a contract for a test, a STOP trigger), and the fencing-free
//!    repository layer (`upsert_from_node`) cannot exercise it (would be HOLLOW).
//! 2. **Reclaim → reassign → strictly-higher token (the partition-safety BASIS)** — the test
//!    below drives the real `try_claim` (203) → `reclaim_expired_leases` (209) → a second
//!    `try_claim` and asserts B's `fencing_token` is strictly higher than A's. This is the
//!    public-repo chain that makes 205's stale-token compare meaningful.
//! 3. **Node self-fence (bounded overlap)** — covered by 208's hermetic
//!    `self_fence_tests::assignments_with_expired_or_missing_lease_are_selected_for_fencing`
//!    and 206's `lease_state_tests::lease_grant_sets_token_and_expiry_on_active_assignment_then_revoke_clears`
//!    on the node `services` crate (asserted by reference — this crate cannot import that
//!    test module).

use sqlx::{PgPool, Row};
use uuid::Uuid;

use remote::db::task_assignments::TaskAssignmentRepository;

fn database_url() -> Option<String> {
    std::env::var("DATABASE_URL").ok()
}

macro_rules! skip_without_db {
    () => {
        if database_url().is_none() {
            eprintln!("Skipping test: DATABASE_URL not set");
            return;
        }
    };
}

async fn create_pool() -> PgPool {
    let url = database_url().expect("DATABASE_URL must be set");
    sqlx::PgPool::connect(&url)
        .await
        .expect("Failed to connect to database")
}

// Fixture helpers copied verbatim from 209's `sweep_tests` mod in
// `crates/remote/src/db/task_assignments.rs` — these are proven against the 201-migrated
// schema (the stale `backfill_e2e.rs` fixtures use the dropped `hostname/os_type/os_version`
// node columns and the pre-`slug` organization table; do NOT copy from there).

async fn create_test_organization(pool: &PgPool) -> Uuid {
    let org_id = Uuid::new_v4();
    let now = chrono::Utc::now();

    sqlx::query(
        r#"
        INSERT INTO organizations (id, name, slug, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(org_id)
    .bind(format!("Test Org {}", org_id))
    .bind(format!("test-org-{}", org_id))
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .expect("Failed to create test organization");

    org_id
}

async fn create_test_node(pool: &PgPool, org_id: Uuid) -> Uuid {
    let node_id = Uuid::new_v4();
    let now = chrono::Utc::now();

    sqlx::query(
        r#"
        INSERT INTO nodes (id, organization_id, name, machine_id, status, capabilities, created_at, updated_at)
        VALUES ($1, $2, $3, $4, 'online', '{}'::jsonb, $5, $6)
        "#,
    )
    .bind(node_id)
    .bind(org_id)
    .bind(format!("node-{}", node_id))
    .bind(format!("machine-{}", node_id))
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .expect("Failed to create test node");

    node_id
}

async fn create_test_swarm_project(pool: &PgPool, org_id: Uuid) -> Uuid {
    let sp_id = Uuid::new_v4();
    let now = chrono::Utc::now();

    sqlx::query(
        r#"
        INSERT INTO swarm_projects (id, organization_id, name, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(sp_id)
    .bind(org_id)
    .bind(format!("Swarm Project {}", sp_id))
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .expect("Failed to create test swarm project");

    sp_id
}

async fn create_test_swarm_project_node(
    pool: &PgPool,
    swarm_project_id: Uuid,
    node_id: Uuid,
) -> Uuid {
    let local_project_id = Uuid::new_v4();

    let row = sqlx::query(
        r#"
        INSERT INTO swarm_project_nodes (swarm_project_id, node_id, local_project_id, git_repo_path)
        VALUES ($1, $2, $3, $4)
        RETURNING id
        "#,
    )
    .bind(swarm_project_id)
    .bind(node_id)
    .bind(local_project_id)
    .bind("test-repo")
    .fetch_one(pool)
    .await
    .expect("Failed to create test swarm project node");

    row.get("id")
}

async fn create_test_shared_task(pool: &PgPool, org_id: Uuid) -> Uuid {
    let task_id = Uuid::new_v4();
    let now = chrono::Utc::now();

    sqlx::query(
        r#"
        INSERT INTO shared_tasks (id, organization_id, title, status, created_at, updated_at)
        VALUES ($1, $2, $3, 'todo'::task_status, $4, $5)
        "#,
    )
    .bind(task_id)
    .bind(org_id)
    .bind(format!("Test Task {}", task_id))
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .expect("Failed to create test shared task");

    task_id
}

async fn cleanup_org(pool: &PgPool, org_id: Uuid) {
    let _ = sqlx::query("DELETE FROM organizations WHERE id = $1")
        .bind(org_id)
        .execute(pool)
        .await;
}

/// TS2 leg 2 (SC3): reclaim → reassign → strictly-higher fencing token.
///
/// Node A leases a task, then is partitioned-but-alive. The lease-expiry sweep (209) reclaims
/// A's expired lease (bumping the fencing token), and node B re-claims the task with a
/// strictly-higher token. That strictly-higher token is exactly what 205's fencing compare
/// uses to REJECT node A's late op (stamped with the OLD token T1 < T2). The reject itself
/// is proven by 205's in-module `#[cfg(test)]` test against the private `handle_op_batch`
/// (R2/F8 — do NOT re-assert it here at the repo layer); the bounded-overlap self-fence is
/// proven by 208's `self_fence_tests` + 206's `lease_state_tests`.
#[tokio::test]
async fn partitioned_node_late_commit_is_rejected_after_reassignment() {
    skip_without_db!();
    let pool = create_pool().await;
    let repo = TaskAssignmentRepository::new(&pool);

    // Seed: org + node_a + node_b + swarm_project + swarm_project_node (np_id) + shared_task.
    let org_id = create_test_organization(&pool).await;
    let node_a = create_test_node(&pool, org_id).await;
    let node_b = create_test_node(&pool, org_id).await;
    let swarm_project = create_test_swarm_project(&pool, org_id).await;
    let np_id = create_test_swarm_project_node(&pool, swarm_project, node_a).await;
    let task_id = create_test_shared_task(&pool, org_id).await;

    // 1. Lease task T to node A (token T1). Use an already-past TTL to simulate the
    //    partition window — A's lease is born expired.
    let a = repo
        .try_claim(task_id, node_a, np_id, chrono::Duration::seconds(-1))
        .await
        .unwrap()
        .expect("A wins");

    // 2. A is partitioned-but-alive. The lease-expiry SWEEP (209) reclaims the expired lease,
    //    bumping the fencing token (exercises `reclaim_expired_leases` — why 210 depends_on 209).
    let reclaimed = repo.reclaim_expired_leases().await.unwrap();
    assert!(
        reclaimed.iter().any(|r| r.task_id == task_id),
        "the sweep reclaimed A's expired lease"
    );

    // 3. Reassign to node B with a live TTL.
    let b = repo
        .try_claim(task_id, node_b, np_id, chrono::Duration::seconds(300))
        .await
        .unwrap()
        .expect("B claims the reclaimed task");

    // 4. B's token is strictly higher than A's — the partition-safety BASIS. This is the
    //    public-repo chain that makes 205's stale-token compare meaningful: any late op A
    //    stamps with T1 is rejected against B's T2 (the reject itself is 205's in-module
    //    test `op_against_assigned_task_with_stale_token_is_rejected_not_applied`).
    assert!(
        b.fencing_token > a.fencing_token,
        "B's token is strictly higher (partition-safety BASIS)"
    );

    // SC3/TS2 = this token-bump chain (leg 2) + 205's reject test (leg 1) + 208's self-fence
    // test (leg 3, bounded overlap). Recorded in the decisions-ledger under Task 210.
    let _ = (a, b);

    cleanup_org(&pool, org_id).await;
}
