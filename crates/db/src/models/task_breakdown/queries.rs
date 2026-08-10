//! Query operations for task breakdown proposals.

use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use uuid::Uuid;

use super::{
    BreakdownStatus, ProposalItemInput, TaskBreakdownProposal, TaskBreakdownProposalItem,
    TaskDependency,
};
use crate::models::task::{Task, TaskStatus};

/// Insert a new draft proposal for a task.
pub async fn create(
    pool: &SqlitePool,
    task_id: Uuid,
) -> Result<TaskBreakdownProposal, sqlx::Error> {
    let id = Uuid::new_v4();
    sqlx::query_as!(
        TaskBreakdownProposal,
        r#"INSERT INTO task_breakdown_proposals (id, task_id)
           VALUES ($1, $2)
           RETURNING id as "id!: Uuid", task_id as "task_id!: Uuid", status as "status!: BreakdownStatus", execution_process_id as "execution_process_id: Uuid", error, created_at as "created_at!: DateTime<Utc>", updated_at as "updated_at!: DateTime<Utc>""#,
        id,
        task_id
    )
    .fetch_one(pool)
    .await
}

/// Latest proposal for a task (by created_at).
pub async fn find_by_task_id(
    pool: &SqlitePool,
    task_id: Uuid,
) -> Result<Option<TaskBreakdownProposal>, sqlx::Error> {
    sqlx::query_as!(
        TaskBreakdownProposal,
        r#"SELECT id as "id!: Uuid", task_id as "task_id!: Uuid", status as "status!: BreakdownStatus", execution_process_id as "execution_process_id: Uuid", error, created_at as "created_at!: DateTime<Utc>", updated_at as "updated_at!: DateTime<Utc>"
           FROM task_breakdown_proposals
           WHERE task_id = $1
           ORDER BY created_at DESC
           LIMIT 1"#,
        task_id
    )
    .fetch_optional(pool)
    .await
}

/// Proposal linked to an execution process (exact match).
pub async fn find_by_execution_process_id(
    pool: &SqlitePool,
    execution_process_id: Uuid,
) -> Result<Option<TaskBreakdownProposal>, sqlx::Error> {
    sqlx::query_as!(
        TaskBreakdownProposal,
        r#"SELECT id as "id!: Uuid", task_id as "task_id!: Uuid", status as "status!: BreakdownStatus", execution_process_id as "execution_process_id: Uuid", error, created_at as "created_at!: DateTime<Utc>", updated_at as "updated_at!: DateTime<Utc>"
           FROM task_breakdown_proposals
           WHERE execution_process_id = $1"#,
        execution_process_id
    )
    .fetch_optional(pool)
    .await
}

pub async fn find_by_id(
    pool: &SqlitePool,
    id: Uuid,
) -> Result<Option<TaskBreakdownProposal>, sqlx::Error> {
    sqlx::query_as!(
        TaskBreakdownProposal,
        r#"SELECT id as "id!: Uuid", task_id as "task_id!: Uuid", status as "status!: BreakdownStatus", execution_process_id as "execution_process_id: Uuid", error, created_at as "created_at!: DateTime<Utc>", updated_at as "updated_at!: DateTime<Utc>"
           FROM task_breakdown_proposals
           WHERE id = $1"#,
        id
    )
    .fetch_optional(pool)
    .await
}

/// Items for a proposal, ordered by sort_order.
pub async fn find_items(
    pool: &SqlitePool,
    proposal_id: Uuid,
) -> Result<Vec<TaskBreakdownProposalItem>, sqlx::Error> {
    sqlx::query_as!(
        TaskBreakdownProposalItem,
        r#"SELECT id as "id!: Uuid", proposal_id as "proposal_id!: Uuid", title, description, sort_order as "sort_order!: i64", depends_on_item_ids, created_at as "created_at!: DateTime<Utc>"
           FROM task_breakdown_proposal_items
           WHERE proposal_id = $1
           ORDER BY sort_order"#,
        proposal_id
    )
    .fetch_all(pool)
    .await
}

/// True when the `depends_on_indices` edges contain a cycle.
///
/// Kahn's algorithm: repeatedly remove a node with no outstanding dependencies;
/// anything left over sits on (or behind) a cycle. Assumes indices are already
/// range-validated by the caller.
fn has_index_cycle(items: &[ProposalItemInput]) -> bool {
    let len = items.len();
    let mut remaining: Vec<usize> = items.iter().map(|i| i.depends_on_indices.len()).collect();
    let mut queue: Vec<usize> = (0..len).filter(|&i| remaining[i] == 0).collect();
    let mut resolved = 0usize;

    while let Some(node) = queue.pop() {
        resolved += 1;
        for (i, item) in items.iter().enumerate() {
            if item.depends_on_indices.contains(&(node as i64)) {
                remaining[i] -= 1;
                if remaining[i] == 0 {
                    queue.push(i);
                }
            }
        }
    }

    resolved != len
}

/// Replace a draft proposal's items (delete + reinsert) in one transaction.
///
/// Validates BEFORE any write: every `depends_on_indices` element must be in
/// range `0..items.len()` and must not equal its own item's index. Touches the
/// parent proposal's `updated_at` in the same transaction (the column DEFAULT
/// fires only on INSERT).
pub async fn replace_items(
    pool: &SqlitePool,
    proposal_id: Uuid,
    items: Vec<ProposalItemInput>,
) -> Result<Vec<TaskBreakdownProposalItem>, sqlx::Error> {
    // Validate references before any write.
    let len = items.len() as i64;
    for (index, item) in items.iter().enumerate() {
        for &dep in &item.depends_on_indices {
            if dep < 0 || dep >= len {
                return Err(sqlx::Error::Protocol(format!(
                    "item {index}: depends_on index {dep} is out of range 0..{len}"
                )));
            }
            if dep == index as i64 {
                return Err(sqlx::Error::Protocol(format!(
                    "item {index}: depends_on index {dep} references itself"
                )));
            }
        }
    }
    // Range and self-reference checks miss a mutual pair (0 -> 1, 1 -> 0).
    // accept_proposal turns every edge into a task_dependencies row, so a cycle
    // accepted here becomes a cyclic graph on real tasks.
    if has_index_cycle(&items) {
        return Err(sqlx::Error::Protocol(
            "depends_on indices form a cycle".to_string(),
        ));
    }

    let mut tx = pool.begin().await?;

    let proposal = sqlx::query_as!(
        TaskBreakdownProposal,
        r#"SELECT id as "id!: Uuid", task_id as "task_id!: Uuid", status as "status!: BreakdownStatus", execution_process_id as "execution_process_id: Uuid", error, created_at as "created_at!: DateTime<Utc>", updated_at as "updated_at!: DateTime<Utc>"
           FROM task_breakdown_proposals
           WHERE id = $1"#,
        proposal_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| sqlx::Error::RowNotFound)?;
    if proposal.status != BreakdownStatus::Draft {
        return Err(sqlx::Error::Protocol(
            "replace_items is only legal while the proposal is a draft".to_string(),
        ));
    }

    sqlx::query!(
        "DELETE FROM task_breakdown_proposal_items WHERE proposal_id = $1",
        proposal_id
    )
    .execute(&mut *tx)
    .await?;

    // Pre-assign ids so depends_on_indices can be resolved to item ids.
    let ids: Vec<Uuid> = items.iter().map(|_| Uuid::new_v4()).collect();
    let mut inserted = Vec::with_capacity(items.len());
    for (index, item) in items.iter().enumerate() {
        let id = ids[index];
        let dep_ids: Vec<Uuid> = item
            .depends_on_indices
            .iter()
            .map(|&dep| ids[dep as usize])
            .collect();
        let depends_on_item_ids = serde_json::to_string(&dep_ids)
            .map_err(|e| sqlx::Error::Protocol(format!("failed to serialize item deps: {e}")))?;
        let row = sqlx::query_as!(
            TaskBreakdownProposalItem,
            r#"INSERT INTO task_breakdown_proposal_items (id, proposal_id, title, description, sort_order, depends_on_item_ids)
               VALUES ($1, $2, $3, $4, $5, $6)
               RETURNING id as "id!: Uuid", proposal_id as "proposal_id!: Uuid", title, description, sort_order as "sort_order!: i64", depends_on_item_ids, created_at as "created_at!: DateTime<Utc>""#,
            id,
            proposal_id,
            item.title,
            item.description,
            item.sort_order,
            depends_on_item_ids
        )
        .fetch_one(&mut *tx)
        .await?;
        inserted.push(row);
    }

    sqlx::query!(
        "UPDATE task_breakdown_proposals SET updated_at = datetime('now','subsec') WHERE id = $1",
        proposal_id
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(inserted)
}

/// Update a proposal's status (and error), refreshing updated_at.
/// The proposal status state machine.
///
/// `Accepted` and `Discarded` are terminal — child tasks already exist for the
/// former, and the latter is an explicit user decision that a late-arriving
/// executor result must not overwrite. A repeat of the current status is a no-op
/// and stays legal so a retried failure path is not itself an error.
fn is_legal_transition(from: BreakdownStatus, to: BreakdownStatus) -> bool {
    use BreakdownStatus::*;
    match (from, to) {
        (a, b) if a == b => true,
        (Draft, Accepted | Discarded | Failed) => true,
        // Retry re-drafts a failed proposal; a failed run may also be discarded.
        (Failed, Draft | Discarded) => true,
        (Accepted | Discarded, _) => false,
        _ => false,
    }
}

pub async fn update_status(
    pool: &SqlitePool,
    id: Uuid,
    status: BreakdownStatus,
    error: Option<String>,
) -> Result<TaskBreakdownProposal, sqlx::Error> {
    // One state machine for every caller. Without this, a late executor completion
    // can overwrite a user's Discard with Failed, and a discard can be applied to an
    // already-Accepted proposal whose child tasks exist.
    let current = find_by_id(pool, id)
        .await?
        .ok_or(sqlx::Error::RowNotFound)?
        .status;
    if !is_legal_transition(current, status) {
        return Err(sqlx::Error::Protocol(format!(
            "illegal breakdown status transition {current:?} -> {status:?}"
        )));
    }

    sqlx::query_as!(
        TaskBreakdownProposal,
        r#"UPDATE task_breakdown_proposals
           SET status = $2, error = $3, updated_at = datetime('now','subsec')
           WHERE id = $1
           RETURNING id as "id!: Uuid", task_id as "task_id!: Uuid", status as "status!: BreakdownStatus", execution_process_id as "execution_process_id: Uuid", error, created_at as "created_at!: DateTime<Utc>", updated_at as "updated_at!: DateTime<Utc>""#,
        id,
        status,
        error
    )
    .fetch_one(pool)
    .await
}

/// Link the execution process that generated this proposal, refreshing updated_at.
pub async fn link_execution_process(
    pool: &SqlitePool,
    id: Uuid,
    execution_process_id: Uuid,
) -> Result<TaskBreakdownProposal, sqlx::Error> {
    sqlx::query_as!(
        TaskBreakdownProposal,
        r#"UPDATE task_breakdown_proposals
           SET execution_process_id = $2, updated_at = datetime('now','subsec')
           WHERE id = $1
           RETURNING id as "id!: Uuid", task_id as "task_id!: Uuid", status as "status!: BreakdownStatus", execution_process_id as "execution_process_id: Uuid", error, created_at as "created_at!: DateTime<Utc>", updated_at as "updated_at!: DateTime<Utc>""#,
        id,
        execution_process_id
    )
    .fetch_one(pool)
    .await
}

/// Accept a draft proposal: create real child tasks, dependency edges, and
/// per-child `task.upsert` outbox ops — all in ONE transaction.
///
/// DELIBERATE, PRE-AUTHORIZED divergence from `Task::create`'s documented
/// best-effort post-insert enqueue: acceptance requires all-or-nothing, so the
/// outbox INSERT runs against the transaction handle and errors are PROPAGATED
/// (aborting the accept). See the Task 102 decisions-ledger entry.
pub async fn accept_proposal(
    pool: &SqlitePool,
    proposal_id: Uuid,
) -> Result<Vec<Task>, sqlx::Error> {
    let mut tx = pool.begin().await?;

    let proposal = sqlx::query_as!(
        TaskBreakdownProposal,
        r#"SELECT id as "id!: Uuid", task_id as "task_id!: Uuid", status as "status!: BreakdownStatus", execution_process_id as "execution_process_id: Uuid", error, created_at as "created_at!: DateTime<Utc>", updated_at as "updated_at!: DateTime<Utc>"
           FROM task_breakdown_proposals
           WHERE id = $1"#,
        proposal_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| sqlx::Error::RowNotFound)?;
    if proposal.status != BreakdownStatus::Draft {
        return Err(sqlx::Error::Protocol(
            "only a draft proposal can be accepted".to_string(),
        ));
    }

    let parent = sqlx::query!(
        r#"SELECT id as "id!: Uuid", project_id as "project_id!: Uuid" FROM tasks WHERE id = $1"#,
        proposal.task_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| sqlx::Error::RowNotFound)?;

    let items = sqlx::query_as!(
        TaskBreakdownProposalItem,
        r#"SELECT id as "id!: Uuid", proposal_id as "proposal_id!: Uuid", title, description, sort_order as "sort_order!: i64", depends_on_item_ids, created_at as "created_at!: DateTime<Utc>"
           FROM task_breakdown_proposal_items
           WHERE proposal_id = $1
           ORDER BY sort_order"#,
        proposal_id
    )
    .fetch_all(&mut *tx)
    .await?;

    // First pass: insert child tasks + per-child outbox op; build item id -> task id map.
    let mut item_to_task: std::collections::HashMap<Uuid, Uuid> = std::collections::HashMap::new();
    let mut created_tasks = Vec::with_capacity(items.len());
    for item in &items {
        let child_id = Uuid::new_v4();
        let status = TaskStatus::Todo;
        let parent_task_id = Some(parent.id);
        let shared_task_id: Option<Uuid> = None;
        let task = sqlx::query_as!(
            Task,
            r#"INSERT INTO tasks (id, project_id, title, description, status, parent_task_id, shared_task_id)
               VALUES ($1, $2, $3, $4, $5, $6, $7)
               RETURNING id as "id!: Uuid", project_id as "project_id!: Uuid", title, description, status as "status!: TaskStatus", parent_task_id as "parent_task_id: Uuid", shared_task_id as "shared_task_id: Uuid", created_at as "created_at!: DateTime<Utc>", updated_at as "updated_at!: DateTime<Utc>",
                         remote_assignee_user_id as "remote_assignee_user_id: Uuid",
                         remote_assignee_name,
                         remote_assignee_username,
                         remote_version as "remote_version!: i64",
                         remote_last_synced_at as "remote_last_synced_at: DateTime<Utc>",
                         remote_stream_node_id as "remote_stream_node_id: Uuid",
                         remote_stream_url,
                         archived_at as "archived_at: DateTime<Utc>",
                         activity_at as "activity_at: DateTime<Utc>""#,
            child_id,
            parent.project_id,
            item.title,
            item.description,
            status,
            parent_task_id,
            shared_task_id
        )
        .fetch_one(&mut *tx)
        .await?;

        // Outbox enqueue INSIDE the transaction (errors propagated — aborts accept).
        // Columns / op_type / payload / idempotency-key derivation mirror
        // Task::enqueue_task_upsert_op + OutboxRepository::enqueue_op.
        let payload = serde_json::to_value(&task)
            .and_then(|v| serde_json::to_string(&v))
            .map_err(|e| sqlx::Error::Protocol(format!("failed to serialize payload: {e}")))?;
        let op_id = Uuid::new_v4();
        let idempotency_key = format!("task:{}:{}", task.id, Uuid::new_v4());
        let fencing_token: Option<i64> = None;
        sqlx::query!(
            r#"INSERT INTO node_outbox (id, seq, op_type, entity_type, entity_id, payload, idempotency_key, fencing_token)
               VALUES ($1, (SELECT COALESCE(MAX(seq),0)+1 FROM node_outbox), 'task.upsert', 'task', $2, $3, $4, $5)"#,
            op_id,
            task.id,
            payload,
            idempotency_key,
            fencing_token
        )
        .execute(&mut *tx)
        .await?;

        item_to_task.insert(item.id, task.id);
        created_tasks.push(task);
    }

    // Second pass: resolve depends_on_item_ids (JSON of item ids) to task_dependencies edges.
    for item in &items {
        let dep_item_ids: Vec<Uuid> =
            serde_json::from_str(&item.depends_on_item_ids).map_err(|e| {
                sqlx::Error::Protocol(format!(
                    "item {}: invalid depends_on_item_ids JSON: {e}",
                    item.id
                ))
            })?;
        let task_id = *item_to_task.get(&item.id).ok_or_else(|| {
            sqlx::Error::Protocol(format!("item {}: no child task was created", item.id))
        })?;
        for dep_item_id in dep_item_ids {
            let depends_on_task_id = *item_to_task.get(&dep_item_id).ok_or_else(|| {
                sqlx::Error::Protocol(format!(
                    "item {}: depends_on_item_ids references unknown item {dep_item_id}",
                    item.id
                ))
            })?;
            sqlx::query!(
                "INSERT INTO task_dependencies (task_id, depends_on_task_id) VALUES ($1, $2)",
                task_id,
                depends_on_task_id
            )
            .execute(&mut *tx)
            .await?;
        }
    }

    sqlx::query!(
        "UPDATE task_breakdown_proposals SET status = 'accepted', updated_at = datetime('now','subsec') WHERE id = $1",
        proposal_id
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    tracing::info!(
        proposal_id = %proposal_id,
        parent_task_id = %proposal.task_id,
        child_count = created_tasks.len(),
        "Accepted task breakdown proposal"
    );
    Ok(created_tasks)
}

/// Dependency edges for a task (edges where `task_id` depends on others).
pub async fn find_dependencies(
    pool: &SqlitePool,
    task_id: Uuid,
) -> Result<Vec<TaskDependency>, sqlx::Error> {
    sqlx::query_as!(
        TaskDependency,
        r#"SELECT task_id as "task_id!: Uuid", depends_on_task_id as "depends_on_task_id!: Uuid", created_at as "created_at!: DateTime<Utc>"
           FROM task_dependencies
           WHERE task_id = $1"#,
        task_id
    )
    .fetch_all(pool)
    .await
}
