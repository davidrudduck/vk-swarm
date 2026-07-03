use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row};
use thiserror::Error;
use uuid::Uuid;

use crate::nodes::{NodeTaskAssignment, UpdateAssignmentData};

#[derive(Debug, Error)]
pub enum TaskAssignmentError {
    #[error("task assignment not found")]
    NotFound,
    #[error("task already has an active assignment")]
    AlreadyAssigned,
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
}

/// Narrow result of an atomic checkout / renew — only the lease fields the wire grant needs.
#[derive(Debug, Clone)]
pub struct LeaseClaim {
    pub assignment_id: Uuid,
    pub node_id: Uuid,
    pub task_id: Uuid,
    pub fencing_token: i64,
    pub lease_expires_at: DateTime<Utc>,
}

/// Narrow result of a reclaim sweep — the lease fields visible post-reclaim.
///
/// `lease_expires_at` is deliberately omitted: the reclaim UPDATE sets it NULL, so it cannot
/// populate the non-Option `LeaseClaim.lease_expires_at` without widening 203's struct (which
/// would force touching `session.rs`, out of scope for 209). The reclaim test only asserts
/// `assignment_id` + `fencing_token`, so this narrower struct is sufficient (ADR-0009 / SC3).
#[derive(Debug, Clone)]
pub struct ReclaimedLease {
    pub assignment_id: Uuid,
    pub node_id: Uuid,
    pub task_id: Uuid,
    pub fencing_token: i64,
}

pub struct TaskAssignmentRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> TaskAssignmentRepository<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    /// Create a new task assignment
    pub async fn create(
        &self,
        task_id: Uuid,
        node_id: Uuid,
        node_project_id: Uuid,
    ) -> Result<NodeTaskAssignment, TaskAssignmentError> {
        let assignment = sqlx::query_as::<_, NodeTaskAssignment>(
            r#"
            INSERT INTO node_task_assignments (task_id, node_id, node_project_id)
            VALUES ($1, $2, $3)
            RETURNING
                id,
                task_id,
                node_id,
                node_project_id,
                local_task_id,
                local_attempt_id,
                execution_status,
                assigned_at,
                started_at,
                completed_at,
                created_at
            "#,
        )
        .bind(task_id)
        .bind(node_id)
        .bind(node_project_id)
        .fetch_one(self.pool)
        .await
        .map_err(|e| {
            if let sqlx::Error::Database(ref db_err) = e {
                // Check for unique constraint violation on active assignments
                if db_err.constraint() == Some("idx_task_assignments_active") {
                    return TaskAssignmentError::AlreadyAssigned;
                }
            }
            TaskAssignmentError::Database(e)
        })?;

        Ok(assignment)
    }

    /// Atomically claim (or reclaim an expired lease on) a task for a node — the partition-safe
    /// checkout (ADR-0009 SC3). Succeeds (returns Some) only when the task has NO live assignment:
    /// either no active row, or the active row's lease has expired. Each grant bumps the monotonic
    /// fencing token via nextval, so a reassigned lease always outranks any prior holder. Two nodes
    /// can never both win (the UPDATE … RETURNING is atomic under the row lock). Returns None when a
    /// live lease blocks the claim.
    pub async fn try_claim(
        &self,
        task_id: Uuid,
        node_id: Uuid,
        node_project_id: Uuid,
        lease_ttl: chrono::Duration,
    ) -> Result<Option<LeaseClaim>, TaskAssignmentError> {
        let expires_at = Utc::now() + lease_ttl;

        // 1. UPDATE the active row IFF it is reclaimable (no live lease). Atomic under the row lock.
        let row = sqlx::query(
            r#"
            UPDATE node_task_assignments
            SET node_id = $2,
                node_project_id = $3,
                lease_expires_at = $4,
                fencing_token = nextval('node_fencing_token_seq'),
                local_task_id = NULL,
                local_attempt_id = NULL,
                execution_status = 'pending',
                started_at = NULL
            WHERE task_id = $1
              AND completed_at IS NULL
              AND (lease_expires_at IS NULL OR lease_expires_at < now())
            RETURNING id, node_id, task_id, fencing_token, lease_expires_at
            "#,
        )
        .bind(task_id)
        .bind(node_id)
        .bind(node_project_id)
        .bind(expires_at)
        .fetch_optional(self.pool)
        .await?;

        if let Some(row) = row {
            return Ok(Some(LeaseClaim {
                assignment_id: row.get("id"),
                node_id: row.get("node_id"),
                task_id: row.get("task_id"),
                fencing_token: row.get("fencing_token"),
                lease_expires_at: row.get("lease_expires_at"),
            }));
        }

        // 2. No row was updated: either no active row exists (→ INSERT a fresh one), or a live
        //    lease blocks the claim (→ the INSERT trips the partial unique index → None).
        let inserted = sqlx::query(
            r#"
            INSERT INTO node_task_assignments (task_id, node_id, node_project_id, lease_expires_at, fencing_token)
            VALUES ($1, $2, $3, $4, nextval('node_fencing_token_seq'))
            RETURNING id, node_id, task_id, fencing_token, lease_expires_at
            "#,
        )
        .bind(task_id)
        .bind(node_id)
        .bind(node_project_id)
        .bind(expires_at)
        .fetch_optional(self.pool)
        .await;

        match inserted {
            Ok(Some(row)) => Ok(Some(LeaseClaim {
                assignment_id: row.get("id"),
                node_id: row.get("node_id"),
                task_id: row.get("task_id"),
                fencing_token: row.get("fencing_token"),
                lease_expires_at: row.get("lease_expires_at"),
            })),
            Ok(None) => Ok(None),
            Err(e) => {
                if let sqlx::Error::Database(ref db_err) = e
                    && db_err.constraint() == Some("idx_task_assignments_active")
                {
                    return Ok(None);
                }
                Err(TaskAssignmentError::Database(e))
            }
        }
    }

    /// Renew (extend) a live lease for its current holder. Does NOT change the fencing token (renewal is
    /// not a reassignment). Returns None if the assignment is gone, completed, or held by a different
    /// node (a foreign node cannot renew someone else's lease).
    pub async fn renew_lease(
        &self,
        assignment_id: Uuid,
        node_id: Uuid,
        lease_ttl: chrono::Duration,
    ) -> Result<Option<LeaseClaim>, TaskAssignmentError> {
        let expires_at = Utc::now() + lease_ttl;

        let row = sqlx::query(
            r#"
            UPDATE node_task_assignments
            SET lease_expires_at = $3
            WHERE id = $1
              AND node_id = $2
              AND completed_at IS NULL
              AND lease_expires_at > NOW()
            RETURNING id, node_id, task_id, fencing_token, lease_expires_at
            "#,
        )
        .bind(assignment_id)
        .bind(node_id)
        .bind(expires_at)
        .fetch_optional(self.pool)
        .await?;

        match row {
            Some(row) => Ok(Some(LeaseClaim {
                assignment_id: row.get("id"),
                node_id: row.get("node_id"),
                task_id: row.get("task_id"),
                fencing_token: row.get("fencing_token"),
                lease_expires_at: row.get("lease_expires_at"),
            })),
            None => Ok(None),
        }
    }

    /// Find an assignment by ID
    pub async fn find_by_id(
        &self,
        assignment_id: Uuid,
    ) -> Result<Option<NodeTaskAssignment>, TaskAssignmentError> {
        let assignment = sqlx::query_as::<_, NodeTaskAssignment>(
            r#"
            SELECT
                id,
                task_id,
                node_id,
                node_project_id,
                local_task_id,
                local_attempt_id,
                execution_status,
                assigned_at,
                started_at,
                completed_at,
                created_at
            FROM node_task_assignments
            WHERE id = $1
            "#,
        )
        .bind(assignment_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(assignment)
    }

    /// Find an assignment by `id` but ONLY if it is an active lease held by `node_id`
    /// (tournament R1/A): `find_by_id` filters solely on `id`, so any node that knows
    /// the assignment_id can look it up. This method adds `node_id`, `completed_at IS NULL`,
    /// and `lease_expires_at IS NOT NULL` guards — the 304 status handler must only act on
    /// an assignment whose lease this node currently holds, not a reclaimed or foreign one.
    pub async fn find_active_lease_for_node(
        &self,
        assignment_id: Uuid,
        node_id: Uuid,
    ) -> Result<Option<NodeTaskAssignment>, TaskAssignmentError> {
        let assignment = sqlx::query_as::<_, NodeTaskAssignment>(
            r#"
            SELECT
                id,
                task_id,
                node_id,
                node_project_id,
                local_task_id,
                local_attempt_id,
                execution_status,
                assigned_at,
                started_at,
                completed_at,
                created_at
            FROM node_task_assignments
            WHERE id = $1
              AND node_id = $2
              AND completed_at IS NULL
              AND lease_expires_at > NOW()
            "#,
        )
        .bind(assignment_id)
        .bind(node_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(assignment)
    }

    /// Find the active assignment for a task (if any)
    pub async fn find_active_for_task(
        &self,
        task_id: Uuid,
    ) -> Result<Option<NodeTaskAssignment>, TaskAssignmentError> {
        let assignment = sqlx::query_as::<_, NodeTaskAssignment>(
            r#"
            SELECT
                id,
                task_id,
                node_id,
                node_project_id,
                local_task_id,
                local_attempt_id,
                execution_status,
                assigned_at,
                started_at,
                completed_at,
                created_at
            FROM node_task_assignments
            WHERE task_id = $1
              AND completed_at IS NULL
            "#,
        )
        .bind(task_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(assignment)
    }

    /// Find the most recent assignment for a task (active or completed).
    /// Used for stream connection info to find which node handles the task.
    pub async fn find_latest_by_task_id(
        &self,
        task_id: Uuid,
    ) -> Result<Option<NodeTaskAssignment>, TaskAssignmentError> {
        let assignment = sqlx::query_as::<_, NodeTaskAssignment>(
            r#"
            SELECT
                id,
                task_id,
                node_id,
                node_project_id,
                local_task_id,
                local_attempt_id,
                execution_status,
                assigned_at,
                started_at,
                completed_at,
                created_at
            FROM node_task_assignments
            WHERE task_id = $1
            ORDER BY assigned_at DESC
            LIMIT 1
            "#,
        )
        .bind(task_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(assignment)
    }

    /// Find an assignment by local_attempt_id.
    /// Used to look up assignment when querying logs for an attempt.
    pub async fn find_by_local_attempt_id(
        &self,
        local_attempt_id: Uuid,
    ) -> Result<Option<NodeTaskAssignment>, TaskAssignmentError> {
        let assignment = sqlx::query_as::<_, NodeTaskAssignment>(
            r#"
            SELECT
                id,
                task_id,
                node_id,
                node_project_id,
                local_task_id,
                local_attempt_id,
                execution_status,
                assigned_at,
                started_at,
                completed_at,
                created_at
            FROM node_task_assignments
            WHERE local_attempt_id = $1
            LIMIT 1
            "#,
        )
        .bind(local_attempt_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(assignment)
    }

    /// List all assignments for a node
    pub async fn list_by_node(
        &self,
        node_id: Uuid,
    ) -> Result<Vec<NodeTaskAssignment>, TaskAssignmentError> {
        let assignments = sqlx::query_as::<_, NodeTaskAssignment>(
            r#"
            SELECT
                id,
                task_id,
                node_id,
                node_project_id,
                local_task_id,
                local_attempt_id,
                execution_status,
                assigned_at,
                started_at,
                completed_at,
                created_at
            FROM node_task_assignments
            WHERE node_id = $1
            ORDER BY assigned_at DESC
            "#,
        )
        .bind(node_id)
        .fetch_all(self.pool)
        .await?;

        Ok(assignments)
    }

    /// List active assignments for a node
    pub async fn list_active_by_node(
        &self,
        node_id: Uuid,
    ) -> Result<Vec<NodeTaskAssignment>, TaskAssignmentError> {
        let assignments = sqlx::query_as::<_, NodeTaskAssignment>(
            r#"
            SELECT
                id,
                task_id,
                node_id,
                node_project_id,
                local_task_id,
                local_attempt_id,
                execution_status,
                assigned_at,
                started_at,
                completed_at,
                created_at
            FROM node_task_assignments
            WHERE node_id = $1
              AND completed_at IS NULL
            ORDER BY assigned_at DESC
            "#,
        )
        .bind(node_id)
        .fetch_all(self.pool)
        .await?;

        Ok(assignments)
    }

    /// Update an assignment with local IDs and status
    pub async fn update(
        &self,
        assignment_id: Uuid,
        data: UpdateAssignmentData,
    ) -> Result<NodeTaskAssignment, TaskAssignmentError> {
        let assignment = sqlx::query_as::<_, NodeTaskAssignment>(
            r#"
            UPDATE node_task_assignments
            SET local_task_id = COALESCE($2, local_task_id),
                local_attempt_id = COALESCE($3, local_attempt_id),
                execution_status = COALESCE($4, execution_status),
                started_at = CASE
                    WHEN $4 = 'running' AND started_at IS NULL THEN NOW()
                    ELSE started_at
                END
            WHERE id = $1
            RETURNING
                id,
                task_id,
                node_id,
                node_project_id,
                local_task_id,
                local_attempt_id,
                execution_status,
                assigned_at,
                started_at,
                completed_at,
                created_at
            "#,
        )
        .bind(assignment_id)
        .bind(data.local_task_id)
        .bind(data.local_attempt_id)
        .bind(&data.execution_status)
        .fetch_optional(self.pool)
        .await?
        .ok_or(TaskAssignmentError::NotFound)?;

        Ok(assignment)
    }

    /// Mark an assignment as completed
    pub async fn complete(
        &self,
        assignment_id: Uuid,
        status: &str,
    ) -> Result<(), TaskAssignmentError> {
        let result = sqlx::query(
            r#"
            UPDATE node_task_assignments
            SET execution_status = $2,
                completed_at = $3
            WHERE id = $1
            "#,
        )
        .bind(assignment_id)
        .bind(status)
        .bind(Utc::now())
        .execute(self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(TaskAssignmentError::NotFound);
        }

        Ok(())
    }

    /// Fail all active assignments for a node (used when node goes offline)
    pub async fn fail_node_assignments(
        &self,
        node_id: Uuid,
    ) -> Result<Vec<Uuid>, TaskAssignmentError> {
        let rows = sqlx::query(
            r#"
            UPDATE node_task_assignments
            SET execution_status = 'failed',
                completed_at = $2
            WHERE node_id = $1
              AND completed_at IS NULL
            RETURNING task_id
            "#,
        )
        .bind(node_id)
        .bind(Utc::now())
        .fetch_all(self.pool)
        .await?;

        Ok(rows.iter().map(|r| r.get("task_id")).collect())
    }

    /// Reclaim all leases whose expiry has lapsed: bump each to a strictly-higher fencing token so
    /// the prior holder's late ops are stale (ADR-0009 / SC3 §C). Returns the reclaimed
    /// assignments. Does NOT reassign to a new node here — it frees the lease (and advances the
    /// token) so the next `try_claim` (or a dispatcher) can take it; the token bump alone is what
    /// bounces the partitioned writer. Mirrors `fail_node_assignments`' runtime-query + RETURNING
    /// shape (no `query!` macro — no offline cache).
    pub async fn reclaim_expired_leases(&self) -> Result<Vec<ReclaimedLease>, TaskAssignmentError> {
        let rows = sqlx::query(
            r#"
            UPDATE node_task_assignments
            SET fencing_token = nextval('node_fencing_token_seq'),
                lease_expires_at = NULL
            WHERE completed_at IS NULL
              AND lease_expires_at IS NOT NULL
              AND lease_expires_at < now()
            RETURNING id, node_id, task_id, fencing_token
            "#,
        )
        .fetch_all(self.pool)
        .await?;

        Ok(rows
            .iter()
            .map(|r| ReclaimedLease {
                assignment_id: r.get("id"),
                node_id: r.get("node_id"),
                task_id: r.get("task_id"),
                fencing_token: r.get("fencing_token"),
            })
            .collect())
    }

    /// Create a synthetic assignment for locally-started tasks.
    ///
    /// These are tasks that were started on the node without Hive dispatch.
    /// We create a "synthetic" assignment so that logs and execution data
    /// can be properly linked via assignment_id foreign keys.
    ///
    /// If an active assignment already exists for this task+node, returns it instead.
    pub async fn create_or_find_synthetic(
        &self,
        task_id: Uuid,
        node_id: Uuid,
        node_project_id: Uuid,
    ) -> Result<NodeTaskAssignment, TaskAssignmentError> {
        // First try to find an existing active assignment for this task
        if let Some(existing) = self.find_active_for_task(task_id).await? {
            return Ok(existing);
        }

        // Create a new synthetic assignment
        // Use 'running' status since the task is already in progress locally
        let assignment = sqlx::query_as::<_, NodeTaskAssignment>(
            r#"
            INSERT INTO node_task_assignments (task_id, node_id, node_project_id, execution_status, started_at)
            VALUES ($1, $2, $3, 'running', NOW())
            ON CONFLICT (task_id) WHERE completed_at IS NULL DO UPDATE
            SET execution_status = node_task_assignments.execution_status
            RETURNING
                id,
                task_id,
                node_id,
                node_project_id,
                local_task_id,
                local_attempt_id,
                execution_status,
                assigned_at,
                started_at,
                completed_at,
                created_at
            "#,
        )
        .bind(task_id)
        .bind(node_id)
        .bind(node_project_id)
        .fetch_one(self.pool)
        .await?;

        Ok(assignment)
    }
}

#[cfg(test)]
mod lease_tests {
    use super::*;
    use sqlx::PgPool;

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

    async fn create_test_organization(pool: &PgPool) -> Uuid {
        let org_id = Uuid::new_v4();
        let now = Utc::now();

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
        let now = Utc::now();

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
        let now = Utc::now();

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
        let now = Utc::now();

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

    #[tokio::test]
    async fn try_claim_wins_when_available_and_assigns_monotonic_token() {
        skip_without_db!();
        let pool = create_pool().await;
        let repo = TaskAssignmentRepository::new(&pool);

        let org_id = create_test_organization(&pool).await;
        let node_a = create_test_node(&pool, org_id).await;
        let swarm_project = create_test_swarm_project(&pool, org_id).await;
        let np_id = create_test_swarm_project_node(&pool, swarm_project, node_a).await;
        let task_id = create_test_shared_task(&pool, org_id).await;

        let claim = repo
            .try_claim(task_id, node_a, np_id, chrono::Duration::seconds(30))
            .await
            .unwrap();
        let claim = claim.expect("claim should succeed on an available task");
        assert_eq!(claim.node_id, node_a);
        assert!(
            claim.fencing_token > 0,
            "a granted token is from nextval (>0)"
        );
        assert!(
            claim.lease_expires_at > chrono::Utc::now(),
            "lease expiry is in the future"
        );

        cleanup_org(&pool, org_id).await;
    }

    #[tokio::test]
    async fn try_claim_fails_for_a_second_node_while_lease_is_live() {
        skip_without_db!();
        let pool = create_pool().await;
        let repo = TaskAssignmentRepository::new(&pool);

        let org_id = create_test_organization(&pool).await;
        let node_a = create_test_node(&pool, org_id).await;
        let node_b = create_test_node(&pool, org_id).await;
        let swarm_project = create_test_swarm_project(&pool, org_id).await;
        let np_id = create_test_swarm_project_node(&pool, swarm_project, node_a).await;
        let task_id = create_test_shared_task(&pool, org_id).await;

        let first = repo
            .try_claim(task_id, node_a, np_id, chrono::Duration::seconds(300))
            .await
            .unwrap()
            .expect("a wins");

        let second = repo
            .try_claim(task_id, node_b, np_id, chrono::Duration::seconds(300))
            .await
            .unwrap();
        assert!(second.is_none(), "a live lease blocks a second claimant");

        let _ = first;
        cleanup_org(&pool, org_id).await;
    }

    #[tokio::test]
    async fn try_claim_reclaims_an_expired_lease_with_a_strictly_higher_token() {
        skip_without_db!();
        let pool = create_pool().await;
        let repo = TaskAssignmentRepository::new(&pool);

        let org_id = create_test_organization(&pool).await;
        let node_a = create_test_node(&pool, org_id).await;
        let node_b = create_test_node(&pool, org_id).await;
        let swarm_project = create_test_swarm_project(&pool, org_id).await;
        let np_id = create_test_swarm_project_node(&pool, swarm_project, node_a).await;
        let task_id = create_test_shared_task(&pool, org_id).await;

        let a = repo
            .try_claim(task_id, node_a, np_id, chrono::Duration::seconds(-1))
            .await
            .unwrap()
            .expect("a wins");

        let b = repo
            .try_claim(task_id, node_b, np_id, chrono::Duration::seconds(300))
            .await
            .unwrap()
            .expect("b reclaims expired");
        assert_eq!(b.node_id, node_b);
        assert!(
            b.fencing_token > a.fencing_token,
            "a reassigned lease MUST get a strictly higher fencing token (the SC3 basis)"
        );

        cleanup_org(&pool, org_id).await;
    }

    #[tokio::test]
    async fn renew_lease_extends_expiry_without_changing_the_token() {
        skip_without_db!();
        let pool = create_pool().await;
        let repo = TaskAssignmentRepository::new(&pool);

        let org_id = create_test_organization(&pool).await;
        let node_a = create_test_node(&pool, org_id).await;
        let node_b = create_test_node(&pool, org_id).await;
        let swarm_project = create_test_swarm_project(&pool, org_id).await;
        let np_id = create_test_swarm_project_node(&pool, swarm_project, node_a).await;
        let task_id = create_test_shared_task(&pool, org_id).await;

        let a = repo
            .try_claim(task_id, node_a, np_id, chrono::Duration::seconds(30))
            .await
            .unwrap()
            .expect("a wins");

        let renewed = repo
            .renew_lease(a.assignment_id, node_a, chrono::Duration::seconds(120))
            .await
            .unwrap()
            .expect("renew succeeds for the lease holder");
        assert_eq!(
            renewed.fencing_token, a.fencing_token,
            "renew does NOT bump the token"
        );
        assert!(
            renewed.lease_expires_at > a.lease_expires_at,
            "renew extends the expiry"
        );

        let stolen = repo
            .renew_lease(a.assignment_id, node_b, chrono::Duration::seconds(120))
            .await
            .unwrap();
        assert!(
            stolen.is_none(),
            "renew is scoped to the current lease holder"
        );

        cleanup_org(&pool, org_id).await;
    }
}

#[cfg(test)]
mod sweep_tests {
    use super::*;
    use sqlx::PgPool;

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

    async fn create_test_organization(pool: &PgPool) -> Uuid {
        let org_id = Uuid::new_v4();
        let now = Utc::now();

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
        let now = Utc::now();

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
        let now = Utc::now();

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
        let now = Utc::now();

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

    #[tokio::test]
    async fn reclaim_expired_leases_bumps_token_and_returns_reclaimed_assignments() {
        skip_without_db!();
        let pool = create_pool().await;
        let repo = TaskAssignmentRepository::new(&pool);

        let org_id = create_test_organization(&pool).await;
        let node_a = create_test_node(&pool, org_id).await;
        let swarm_project = create_test_swarm_project(&pool, org_id).await;
        let np_id = create_test_swarm_project_node(&pool, swarm_project, node_a).await;
        let task_id = create_test_shared_task(&pool, org_id).await;

        // node_a claims a task with an ALREADY-PAST TTL (lease_expires_at < now).
        let a = repo
            .try_claim(task_id, node_a, np_id, chrono::Duration::seconds(-1))
            .await
            .unwrap()
            .expect("a wins");

        // Sweep reclaims expired leases.
        let reclaimed = repo.reclaim_expired_leases().await.unwrap();
        assert!(
            reclaimed.iter().any(|r| r.assignment_id == a.assignment_id),
            "an expired lease is reclaimed by the sweep"
        );

        let r = reclaimed
            .iter()
            .find(|r| r.assignment_id == a.assignment_id)
            .unwrap();
        assert!(
            r.fencing_token > a.fencing_token,
            "reclaim bumps the fencing token strictly higher (so the old holder's late ops are stale)"
        );

        cleanup_org(&pool, org_id).await;
    }

    #[tokio::test]
    async fn sweep_does_not_touch_live_leases() {
        skip_without_db!();
        let pool = create_pool().await;
        let repo = TaskAssignmentRepository::new(&pool);

        let org_id = create_test_organization(&pool).await;
        let node_a = create_test_node(&pool, org_id).await;
        let swarm_project = create_test_swarm_project(&pool, org_id).await;
        let np_id = create_test_swarm_project_node(&pool, swarm_project, node_a).await;
        let task_id = create_test_shared_task(&pool, org_id).await;

        let _live = repo
            .try_claim(task_id, node_a, np_id, chrono::Duration::seconds(300))
            .await
            .unwrap()
            .expect("a wins");

        let reclaimed = repo.reclaim_expired_leases().await.unwrap();
        assert!(
            reclaimed.iter().all(|r| r.task_id != task_id),
            "a live lease is not reclaimed"
        );

        cleanup_org(&pool, org_id).await;
    }
}
