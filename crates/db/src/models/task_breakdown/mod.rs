//! Task breakdown proposals: node-local draft decompositions of a task.
//!
//! A proposal is a reviewable draft set of child-task items for a parent task.
//! Proposals never sync to the hive; only acceptance creates real tasks (which
//! sync via the existing `task.upsert` outbox path).

mod queries;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use ts_rs::TS;
use uuid::Uuid;

pub use queries::{
    accept_proposal, create, find_by_execution_process_id, find_by_id, find_by_task_id,
    find_dependencies, find_items, link_execution_process, replace_items, update_status,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "breakdown_status", rename_all = "lowercase")]
pub enum BreakdownStatus {
    Draft,
    Accepted,
    Discarded,
    Failed,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct TaskBreakdownProposal {
    pub id: Uuid,
    pub task_id: Uuid,
    pub status: BreakdownStatus,
    pub execution_process_id: Option<Uuid>,
    pub error: Option<String>,
    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
    #[ts(type = "Date")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct TaskBreakdownProposalItem {
    pub id: Uuid,
    pub proposal_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub sort_order: i64,
    pub depends_on_item_ids: String, // JSON array of item Uuids
    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct UpsertProposalItems {
    pub items: Vec<ProposalItemInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ProposalItemInput {
    pub title: String,
    pub description: Option<String>,
    pub sort_order: i64,
    pub depends_on_indices: Vec<i64>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct TaskDependency {
    pub task_id: Uuid,
    pub depends_on_task_id: Uuid,
    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        project::{CreateProject, Project},
        task::{CreateTask, Task},
    };
    use crate::test_utils::create_test_pool;
    use sqlx::SqlitePool;

    async fn create_project(pool: &SqlitePool) -> Uuid {
        let project_id = Uuid::new_v4();
        let project_data = CreateProject {
            name: "Test Project".to_string(),
            git_repo_path: format!("/tmp/test-repo-{}", project_id),
            use_existing_repo: true,
            clone_url: None,
            setup_script: None,
            dev_script: None,
            cleanup_script: None,
            copy_files: None,
        };
        Project::create(pool, &project_data, project_id)
            .await
            .expect("Failed to create project");
        project_id
    }

    async fn create_task(pool: &SqlitePool, project_id: Uuid) -> Uuid {
        let task_id = Uuid::new_v4();
        let task_data = CreateTask::from_title_description(
            project_id,
            "Parent Task".to_string(),
            Some("Description".to_string()),
        );
        Task::create(pool, &task_data, task_id)
            .await
            .expect("Failed to create task");
        task_id
    }

    async fn create_test_attempt(pool: &SqlitePool, task_id: Uuid) -> Uuid {
        let id = Uuid::new_v4();
        sqlx::query(
            r#"INSERT INTO task_attempts (id, task_id, executor, branch, target_branch, container_ref)
               VALUES ($1, $2, 'CLAUDE_CODE', 'test-branch', 'main', '/tmp/test-worktree')"#,
        )
        .bind(id)
        .bind(task_id)
        .execute(pool)
        .await
        .expect("create attempt");
        id
    }

    async fn create_execution_process(pool: &SqlitePool, attempt_id: Uuid) -> Uuid {
        let exec_id = Uuid::new_v4();
        sqlx::query(
            r#"INSERT INTO execution_processes (id, task_attempt_id, status, run_reason, executor_action)
               VALUES ($1, $2, 'completed', 'codingagent', '{}')"#,
        )
        .bind(exec_id)
        .bind(attempt_id)
        .execute(pool)
        .await
        .expect("create execution");
        exec_id
    }

    fn item(title: &str, sort_order: i64, depends_on_indices: Vec<i64>) -> ProposalItemInput {
        ProposalItemInput {
            title: title.to_string(),
            description: None,
            sort_order,
            depends_on_indices,
        }
    }

    #[tokio::test]
    async fn test_proposal_crud_and_one_draft_constraint() {
        let (pool, _temp_dir) = create_test_pool().await;
        let project_id = create_project(&pool).await;
        let task_id = create_task(&pool, project_id).await;

        let proposal = create(&pool, task_id).await.expect("create proposal");
        assert_eq!(proposal.task_id, task_id);
        assert_eq!(proposal.status, BreakdownStatus::Draft);

        let found = find_by_task_id(&pool, task_id)
            .await
            .expect("find_by_task_id")
            .expect("proposal not found");
        assert_eq!(found.id, proposal.id);

        // Second draft for the same task violates the one-draft unique index.
        let second = create(&pool, task_id).await;
        assert!(second.is_err(), "second draft for same task must error");

        // Discard the first; a new draft is then allowed.
        update_status(&pool, proposal.id, BreakdownStatus::Discarded, None)
            .await
            .expect("update_status");
        let new_draft = create(&pool, task_id)
            .await
            .expect("new draft after discard");
        assert_eq!(new_draft.status, BreakdownStatus::Draft);
    }

    #[tokio::test]
    async fn test_cascade_delete() {
        let (pool, _temp_dir) = create_test_pool().await;
        let project_id = create_project(&pool).await;
        let task_id = create_task(&pool, project_id).await;

        let proposal = create(&pool, task_id).await.expect("create proposal");
        replace_items(
            &pool,
            proposal.id,
            vec![item("A", 0, vec![]), item("B", 1, vec![])],
        )
        .await
        .expect("replace_items");
        assert_eq!(find_items(&pool, proposal.id).await.unwrap().len(), 2);

        // Delete the parent task row; cascade removes proposal and items.
        Task::delete(&pool, task_id).await.expect("delete task");

        let gone = find_by_id(&pool, proposal.id).await.expect("find_by_id");
        assert!(gone.is_none(), "proposal must be cascade-deleted");
        assert!(
            find_items(&pool, proposal.id).await.unwrap().is_empty(),
            "items must be cascade-deleted"
        );
    }

    #[tokio::test]
    async fn test_accept_transaction_atomic() {
        let (pool, _temp_dir) = create_test_pool().await;
        let project_id = create_project(&pool).await;
        let task_id = create_task(&pool, project_id).await;

        let proposal = create(&pool, task_id).await.expect("create proposal");
        replace_items(
            &pool,
            proposal.id,
            vec![item("A", 0, vec![]), item("B", 1, vec![0])],
        )
        .await
        .expect("replace_items");

        let children = accept_proposal(&pool, proposal.id)
            .await
            .expect("accept_proposal");
        assert_eq!(children.len(), 2);
        for child in &children {
            assert_eq!(child.parent_task_id, Some(task_id));
            assert_eq!(child.project_id, project_id);
        }
        let a_task = children.iter().find(|t| t.title == "A").unwrap();
        let b_task = children.iter().find(|t| t.title == "B").unwrap();

        // Exactly one dependency edge: B -> A.
        let deps = find_dependencies(&pool, b_task.id)
            .await
            .expect("find_dependencies");
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].task_id, b_task.id);
        assert_eq!(deps[0].depends_on_task_id, a_task.id);
        assert!(
            find_dependencies(&pool, a_task.id)
                .await
                .unwrap()
                .is_empty(),
            "A has no dependencies"
        );

        let accepted = find_by_id(&pool, proposal.id)
            .await
            .unwrap()
            .expect("proposal exists");
        assert_eq!(accepted.status, BreakdownStatus::Accepted);

        // node_outbox contains a task.upsert row for EACH child (filtered by
        // entity_id; parent task setup also enqueues a row — no absolute count).
        for child in &children {
            let count: (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM node_outbox WHERE op_type = 'task.upsert' AND entity_id = ?",
            )
            .bind(child.id)
            .fetch_one(&pool)
            .await
            .unwrap();
            assert_eq!(count.0, 1, "one task.upsert outbox row per child");
        }

        // Rollback proof: a proposal whose item references a NON-EXISTENT item id.
        let bad_proposal = create(&pool, task_id).await.expect("new draft");
        let bad_item_id = Uuid::new_v4();
        let dangling_ref = serde_json::to_string(&vec![Uuid::new_v4()]).unwrap();
        sqlx::query(
            r#"INSERT INTO task_breakdown_proposal_items (id, proposal_id, title, sort_order, depends_on_item_ids)
               VALUES ($1, $2, 'Dangling', 0, $3)"#,
        )
        .bind(bad_item_id)
        .bind(bad_proposal.id)
        .bind(&dangling_ref)
        .execute(&pool)
        .await
        .unwrap();

        let tasks_before: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM tasks")
            .fetch_one(&pool)
            .await
            .unwrap();
        let edges_before: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM task_dependencies")
            .fetch_one(&pool)
            .await
            .unwrap();
        let outbox_before: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM node_outbox")
            .fetch_one(&pool)
            .await
            .unwrap();

        let result = accept_proposal(&pool, bad_proposal.id).await;
        assert!(result.is_err(), "dangling item ref must abort accept");

        let tasks_after: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM tasks")
            .fetch_one(&pool)
            .await
            .unwrap();
        let edges_after: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM task_dependencies")
            .fetch_one(&pool)
            .await
            .unwrap();
        let outbox_after: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM node_outbox")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(tasks_before.0, tasks_after.0, "no new tasks (rollback)");
        assert_eq!(edges_before.0, edges_after.0, "no new edges (rollback)");
        assert_eq!(
            outbox_before.0, outbox_after.0,
            "no new outbox rows (rollback)"
        );
    }

    #[tokio::test]
    async fn test_find_by_execution_process_id_exact() {
        let (pool, _temp_dir) = create_test_pool().await;
        let project_id = create_project(&pool).await;
        let task_id = create_task(&pool, project_id).await;
        let attempt_id = create_test_attempt(&pool, task_id).await;
        let p1 = create_execution_process(&pool, attempt_id).await;
        let p2 = create_execution_process(&pool, attempt_id).await;

        // Historical discarded proposal linked to P1.
        let historical = create(&pool, task_id).await.expect("create historical");
        link_execution_process(&pool, historical.id, p1)
            .await
            .expect("link P1");
        update_status(&pool, historical.id, BreakdownStatus::Discarded, None)
            .await
            .expect("discard");

        // Current draft linked to P2.
        let draft = create(&pool, task_id).await.expect("create draft");
        link_execution_process(&pool, draft.id, p2)
            .await
            .expect("link P2");

        let by_p2 = find_by_execution_process_id(&pool, p2)
            .await
            .unwrap()
            .expect("P2 proposal");
        assert_eq!(by_p2.id, draft.id);
        assert_eq!(by_p2.status, BreakdownStatus::Draft);

        let by_p1 = find_by_execution_process_id(&pool, p1)
            .await
            .unwrap()
            .expect("P1 proposal");
        assert_eq!(by_p1.id, historical.id);
        assert_eq!(by_p1.status, BreakdownStatus::Discarded);

        let none = find_by_execution_process_id(&pool, Uuid::new_v4())
            .await
            .unwrap();
        assert!(none.is_none());
    }

    #[tokio::test]
    async fn test_replace_items_rejects_dependency_cycle() {
        let (pool, _temp_dir) = create_test_pool().await;
        let project_id = create_project(&pool).await;
        let task_id = create_task(&pool, project_id).await;
        let proposal = create(&pool, task_id).await.expect("create proposal");

        // Every index is in range and none is self-referential, so the range and
        // self-reference checks pass. Only the cycle check rejects this pair —
        // accept_proposal would otherwise write both task_dependencies edges.
        let result = replace_items(
            &pool,
            proposal.id,
            vec![item("A", 0, vec![1]), item("B", 1, vec![0])],
        )
        .await;
        assert!(result.is_err(), "a mutual dependency pair must be rejected");
        assert!(
            find_items(&pool, proposal.id).await.unwrap().is_empty(),
            "nothing is written when validation rejects the batch"
        );

        // A diamond is a DAG, not a cycle, and must still be accepted.
        replace_items(
            &pool,
            proposal.id,
            vec![
                item("A", 0, vec![]),
                item("B", 1, vec![0]),
                item("C", 2, vec![0]),
                item("D", 3, vec![1, 2]),
            ],
        )
        .await
        .expect("a diamond dependency graph is acyclic");
    }

    #[tokio::test]
    async fn test_replace_items_dedupes_duplicate_dependencies() {
        let (pool, _temp_dir) = create_test_pool().await;
        let project_id = create_project(&pool).await;
        let task_id = create_task(&pool, project_id).await;
        let proposal = create(&pool, task_id).await.expect("create proposal");

        // [0, 0] is one edge expressed twice on an acyclic set. Seeding Kahn's
        // in-degree from the raw length made this unresolvable and reported it as a
        // cycle; a duplicate surviving to accept would violate the
        // task_dependencies primary key.
        let stored = replace_items(
            &pool,
            proposal.id,
            vec![item("A", 0, vec![]), item("B", 1, vec![0, 0])],
        )
        .await
        .expect("a duplicated dependency is not a cycle");

        let b = stored
            .iter()
            .find(|i| i.title == "B")
            .expect("item B was stored");
        let deps: Vec<uuid::Uuid> =
            serde_json::from_str(&b.depends_on_item_ids).expect("depends_on_item_ids is JSON");
        assert_eq!(deps.len(), 1, "the duplicate edge is collapsed to one");
    }

    #[tokio::test]
    async fn test_accept_with_duplicated_dependency_writes_one_edge() {
        let (pool, _temp_dir) = create_test_pool().await;
        let project_id = create_project(&pool).await;
        let task_id = create_task(&pool, project_id).await;
        let proposal = create(&pool, task_id).await.expect("create proposal");

        replace_items(
            &pool,
            proposal.id,
            vec![item("A", 0, vec![]), item("B", 1, vec![0, 0])],
        )
        .await
        .expect("replace_items accepts the duplicate");

        let created = accept_proposal(&pool, proposal.id)
            .await
            .expect("accept must not abort on a UNIQUE violation");
        assert_eq!(created.len(), 2, "both child tasks are created");

        let child_b = created
            .iter()
            .find(|t| t.title == "B")
            .expect("child task B exists");
        let edges = find_dependencies(&pool, child_b.id)
            .await
            .expect("dependency edges are queryable");
        assert_eq!(edges.len(), 1, "exactly one dependency edge is persisted");
    }

    #[tokio::test]
    async fn test_update_status_is_compare_and_swap_under_concurrency() {
        let (pool, _temp_dir) = create_test_pool().await;
        let project_id = create_project(&pool).await;
        let task_id = create_task(&pool, project_id).await;
        let proposal = create(&pool, task_id).await.expect("create proposal");

        // A user's Discard racing a late fail_proposal: both read Draft and both pass
        // the legal-transition check. Without the status predicate on the UPDATE the
        // second write silently wins, overwriting the user's decision.
        let (discard, fail) = tokio::join!(
            update_status(&pool, proposal.id, BreakdownStatus::Discarded, None),
            update_status(
                &pool,
                proposal.id,
                BreakdownStatus::Failed,
                Some("late run result".into())
            ),
        );

        let winners = [discard.is_ok(), fail.is_ok()]
            .iter()
            .filter(|ok| **ok)
            .count();
        assert_eq!(
            winners, 1,
            "exactly one concurrent transition out of Draft may succeed"
        );

        let final_status = find_by_id(&pool, proposal.id)
            .await
            .unwrap()
            .unwrap()
            .status;
        let expected = if discard.is_ok() {
            BreakdownStatus::Discarded
        } else {
            BreakdownStatus::Failed
        };
        assert_eq!(
            final_status, expected,
            "the stored status is the winner's, not the last writer's"
        );
    }

    #[tokio::test]
    async fn test_update_status_enforces_state_machine() {
        let (pool, _temp_dir) = create_test_pool().await;
        let project_id = create_project(&pool).await;
        let task_id = create_task(&pool, project_id).await;
        let proposal = create(&pool, task_id).await.expect("create proposal");

        // Draft -> Failed -> Draft (the retry path) is legal.
        update_status(
            &pool,
            proposal.id,
            BreakdownStatus::Failed,
            Some("boom".into()),
        )
        .await
        .expect("draft -> failed");
        update_status(&pool, proposal.id, BreakdownStatus::Draft, None)
            .await
            .expect("failed -> draft (retry)");

        // Discard is terminal: a late executor completion must not overwrite it.
        update_status(&pool, proposal.id, BreakdownStatus::Discarded, None)
            .await
            .expect("draft -> discarded");
        let late = update_status(
            &pool,
            proposal.id,
            BreakdownStatus::Failed,
            Some("late run result".into()),
        )
        .await;
        assert!(
            late.is_err(),
            "a discarded proposal must not be reopened by a late completion"
        );
        assert_eq!(
            find_by_id(&pool, proposal.id)
                .await
                .unwrap()
                .unwrap()
                .status,
            BreakdownStatus::Discarded,
            "the user's discard survives"
        );
    }

    #[tokio::test]
    async fn test_replace_items_rejects_non_draft() {
        let (pool, _temp_dir) = create_test_pool().await;
        let project_id = create_project(&pool).await;
        let task_id = create_task(&pool, project_id).await;
        let proposal = create(&pool, task_id).await.expect("create proposal");

        update_status(&pool, proposal.id, BreakdownStatus::Accepted, None)
            .await
            .expect("update_status to accepted");

        let result = replace_items(&pool, proposal.id, vec![item("A", 0, vec![])]).await;
        assert!(
            result.is_err(),
            "replace_items on a non-draft proposal must error"
        );
    }

    #[tokio::test]
    async fn test_accept_rejects_non_draft() {
        let (pool, _temp_dir) = create_test_pool().await;
        let project_id = create_project(&pool).await;
        let task_id = create_task(&pool, project_id).await;
        let proposal = create(&pool, task_id).await.expect("create proposal");
        replace_items(&pool, proposal.id, vec![item("A", 0, vec![])])
            .await
            .expect("replace_items");

        let children = accept_proposal(&pool, proposal.id)
            .await
            .expect("first accept succeeds");
        assert_eq!(children.len(), 1);

        let tasks_before: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM tasks")
            .fetch_one(&pool)
            .await
            .unwrap();

        let second = accept_proposal(&pool, proposal.id).await;
        assert!(second.is_err(), "second accept on same proposal must error");

        let tasks_after: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM tasks")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(
            tasks_before.0, tasks_after.0,
            "no double child creation on re-accept"
        );
    }

    #[tokio::test]
    async fn test_accept_outbox_row_shape() {
        let (pool, _temp_dir) = create_test_pool().await;
        let project_id = create_project(&pool).await;
        let task_id = create_task(&pool, project_id).await;
        let proposal = create(&pool, task_id).await.expect("create proposal");
        replace_items(&pool, proposal.id, vec![item("A", 0, vec![])])
            .await
            .expect("replace_items");

        let children = accept_proposal(&pool, proposal.id)
            .await
            .expect("accept_proposal");
        let child_id = children[0].id;

        // Mirror the literal values enqueue_task_upsert_op writes (task/queries.rs):
        // op_type='task.upsert', entity_type='task', idempotency_key="task:{id}:{uuid}",
        // payload = serde_json::to_value(&task).
        let row: (String, String, String) = sqlx::query_as(
            "SELECT entity_type, idempotency_key, payload FROM node_outbox
             WHERE op_type = 'task.upsert' AND entity_id = ?",
        )
        .bind(child_id)
        .fetch_one(&pool)
        .await
        .expect("outbox row for accepted child");

        assert_eq!(row.0, "task", "entity_type mirrors enqueue_task_upsert_op");
        assert!(
            row.1.starts_with(&format!("task:{}:", child_id)),
            "idempotency_key must start with task:{{child_id}}:, got {}",
            row.1
        );
        assert!(!row.2.is_empty(), "payload must be non-empty");
        let payload: serde_json::Value =
            serde_json::from_str(&row.2).expect("payload parses as JSON");
        assert_eq!(
            payload["id"].as_str(),
            Some(child_id.to_string().as_str()),
            "payload id field is the serialized child task id"
        );
    }

    #[tokio::test]
    async fn test_replace_items_rejects_self_and_dangling_refs() {
        let (pool, _temp_dir) = create_test_pool().await;
        let project_id = create_project(&pool).await;
        let task_id = create_task(&pool, project_id).await;
        let proposal = create(&pool, task_id).await.expect("create proposal");

        // Seed valid items.
        replace_items(
            &pool,
            proposal.id,
            vec![item("A", 0, vec![]), item("B", 1, vec![0])],
        )
        .await
        .expect("initial replace_items");
        let before = find_items(&pool, proposal.id).await.unwrap();
        assert_eq!(before.len(), 2);

        // Self-reference: item 0 depends on index 0.
        let result = replace_items(&pool, proposal.id, vec![item("Selfish", 0, vec![0])]).await;
        assert!(result.is_err(), "self-reference must be rejected");
        let after_self = find_items(&pool, proposal.id).await.unwrap();
        assert_eq!(
            after_self.len(),
            2,
            "previous items remain after self-ref rejection"
        );
        assert_eq!(after_self[0].title, "A");
        assert_eq!(after_self[1].title, "B");

        // Out-of-range index.
        let result = replace_items(&pool, proposal.id, vec![item("Dangler", 0, vec![5])]).await;
        assert!(result.is_err(), "out-of-range index must be rejected");
        let after_dangling = find_items(&pool, proposal.id).await.unwrap();
        assert_eq!(
            after_dangling.len(),
            2,
            "previous items remain after dangling rejection"
        );

        // updated_at refreshes on a successful replace_items.
        let updated_at_before = find_by_id(&pool, proposal.id)
            .await
            .unwrap()
            .unwrap()
            .updated_at;
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        replace_items(&pool, proposal.id, vec![item("C", 0, vec![])])
            .await
            .expect("successful replace_items");
        let updated_at_after = find_by_id(&pool, proposal.id)
            .await
            .unwrap()
            .unwrap()
            .updated_at;
        assert!(
            updated_at_after > updated_at_before,
            "updated_at must refresh on replace_items"
        );
    }

    #[tokio::test]
    async fn accepting_a_proposal_journals_one_task_created_per_child() {
        let (pool, _temp_dir) = create_test_pool().await;
        let project_id = create_project(&pool).await;
        let task_id = create_task(&pool, project_id).await;

        let proposal = create(&pool, task_id).await.expect("create proposal");
        replace_items(
            &pool,
            proposal.id,
            vec![
                item("A", 0, vec![]),
                item("B", 1, vec![]),
                item("C", 2, vec![]),
            ],
        )
        .await
        .expect("replace_items");

        // Clear the journal before accepting so we measure only the acceptance's events
        sqlx::query("DELETE FROM event_journal")
            .execute(&pool)
            .await
            .expect("clear journal");

        let children = accept_proposal(&pool, proposal.id)
            .await
            .expect("accept_proposal");
        assert_eq!(children.len(), 3, "should create 3 children");

        // Build expected child task id set
        let expected_child_ids: std::collections::HashSet<String> =
            children.iter().map(|t| t.id.to_string()).collect();

        // Query event_journal for TaskCreated events (filter to just task_created rows)
        let journal_rows: Vec<(i64, String)> = sqlx::query_as(
            "SELECT seq, payload FROM event_journal WHERE event_type = 'task_created' ORDER BY seq ASC",
        )
        .fetch_all(&pool)
        .await
        .expect("query event_journal");

        // Assert exactly 3 TaskCreated rows (catches duplicate appends)
        assert_eq!(
            journal_rows.len(),
            3,
            "event_journal must contain exactly 3 TaskCreated rows"
        );

        // Extract task_ids from the payloads into a HashSet
        let journaled_task_ids: std::collections::HashSet<String> = journal_rows
            .iter()
            .map(|(_seq, payload)| {
                let event_value: serde_json::Value =
                    serde_json::from_str(payload).expect("event payload should parse as JSON");
                event_value["task_id"]
                    .as_str()
                    .expect("task_id should be present in payload")
                    .to_string()
            })
            .collect();

        // Assert strict HashSet equality: journaled ids must match expected ids exactly
        assert_eq!(
            journaled_task_ids, expected_child_ids,
            "journaled task_ids must be exactly the 3 child ids"
        );
    }

    #[tokio::test]
    async fn a_failed_acceptance_journals_nothing() {
        let (pool, _temp_dir) = create_test_pool().await;
        let project_id = create_project(&pool).await;
        let task_id = create_task(&pool, project_id).await;

        // Build a Draft proposal with a dependency: B depends on A.
        // This forces accept_proposal's SECOND pass (task_dependencies insert at queries.rs:480-516)
        // to run AFTER the first pass has inserted all children AND appended all events.
        let proposal = create(&pool, task_id).await.expect("create proposal");
        replace_items(
            &pool,
            proposal.id,
            vec![item("A", 0, vec![]), item("B", 1, vec![0])],
        )
        .await
        .expect("replace_items");

        // Clear the journal before the fault injection so we can measure only the acceptance's events
        sqlx::query("DELETE FROM event_journal")
            .execute(&pool)
            .await
            .expect("clear journal");

        // Fault-inject: rename task_dependencies away BEFORE accepting.
        // This is a plain statement, outside any transaction.
        sqlx::query("ALTER TABLE task_dependencies RENAME TO task_dependencies_hidden")
            .execute(&pool)
            .await
            .expect("rename task_dependencies away");

        // Try to accept: must fail when the second pass tries to INSERT into the missing table.
        let result = accept_proposal(&pool, proposal.id).await;
        assert!(
            result.is_err(),
            "accept must fail when task_dependencies is missing"
        );

        // Restore the table (fault injection cleanup).
        sqlx::query("ALTER TABLE task_dependencies_hidden RENAME TO task_dependencies")
            .execute(&pool)
            .await
            .expect("rename task_dependencies back");

        // Assert BOTH:
        // (1) Journal is empty: no task_created rows (rollback took the appended events)
        let journal_count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM event_journal WHERE event_type = 'task_created'")
                .fetch_one(&pool)
                .await
                .expect("query journal count");
        assert_eq!(
            journal_count.0, 0,
            "journal must be empty after failed acceptance (rollback took events)"
        );

        // (2) No children were created: zero tasks with parent_task_id pointing to the parent
        let child_count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM tasks WHERE parent_task_id = ?")
                .bind(task_id)
                .fetch_one(&pool)
                .await
                .expect("query child count");
        assert_eq!(
            child_count.0, 0,
            "no children should exist after failed acceptance (rollback removed them)"
        );
    }
}
