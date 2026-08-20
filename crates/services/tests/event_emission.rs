//! Integration tests for cross-site emission of node events.
//!
//! This suite verifies that exactly-one-event-per-state-change property holds across all lifecycle sites
//! that emit to the event journal. Tests are organized by emission site (Task CRUD, breakdown acceptance,
//! remote upsert, execution process lifecycle), with a shared regression guard against double-emission.
//!
//! **Connectivity delegation (item 3 of task 015):** Connectivity state transitions
//! (`HiveConnected` / `HiveDisconnected`) are tested via `node_runner.rs`'s EIGHT colocated
//! `connectivity_event_tests`, not here. The `ConnectivityJournal` is PRIVATE to `node_runner`
//! (by design), and an integration test in `crates/services/` cannot access it without inversion
//! of that privacy boundary. The single-emission-per-transition property is already pinned by
//! `node_runner`'s direct unit tests; upstream `hive_client.rs` clean-close behavior is verifiable
//! only live (task 012's SC3 check). This suite asserts the ONE property that *can* be checked
//! here: the five primary lifecycle sites all emit correctly and only once.

use db::models::event::NodeEvent;
use db::models::execution_process::{
    CreateExecutionProcess, ExecutionProcessRunReason, ExecutionProcessStatus,
};
use db::models::task::{CreateTask, Task, TaskStatus};
use db::models::task_breakdown;
use executors::actions::{
    ExecutorAction, ExecutorActionType, coding_agent_initial::CodingAgentInitialRequest,
};
use executors::executors::BaseCodingAgent;
use executors::profile::ExecutorProfileId;
use uuid::Uuid;

// ==================
// Test 1: task_crud_emits_exactly_one_event_each
// ==================

#[tokio::test]
async fn task_crud_emits_exactly_one_event_each() {
    let (pool, _temp_dir) = db::test_utils::create_test_pool_with_migrations().await;

    // Setup: create a test project
    let project_id = Uuid::new_v4();
    sqlx::query("INSERT INTO projects (id, name) VALUES (?, ?)")
        .bind(project_id)
        .bind("Test Project")
        .execute(&pool)
        .await
        .unwrap();

    // Count events before any operations
    let initial_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM event_journal")
        .fetch_one(&pool)
        .await
        .unwrap();

    // Phase 1: Task::create emits exactly one TaskCreated
    let task_id = Uuid::new_v4();
    let create_data = CreateTask {
        project_id,
        title: "Test Task".to_string(),
        description: None,
        status: None,
        parent_task_id: None,
        image_ids: None,
        shared_task_id: None,
    };
    let task = Task::create(&pool, &create_data, task_id)
        .await
        .expect("create failed");

    let after_create_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM event_journal")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        after_create_count - initial_count,
        1,
        "Task::create should emit exactly 1 event"
    );

    // Verify it's a TaskCreated event
    let created_events: Vec<String> = sqlx::query_scalar(
        "SELECT event_type FROM event_journal WHERE event_type = 'task_created' AND seq > ?",
    )
    .bind(initial_count)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(created_events.len(), 1, "exactly 1 task_created event");

    let before_status_change_count = after_create_count;

    // Phase 2: Task::update_status to a new status emits exactly one TaskStatusChanged
    Task::update_status(&pool, task.id, TaskStatus::InProgress)
        .await
        .expect("update_status failed");

    let after_status_change_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM event_journal")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        after_status_change_count - before_status_change_count,
        1,
        "Task::update_status should emit exactly 1 event when status changes"
    );

    // Verify it's a TaskStatusChanged event
    let status_changed_events: Vec<(String, String)> = sqlx::query_as(
        "SELECT event_type, payload FROM event_journal WHERE event_type = 'task_status_changed' AND seq > ?",
    )
    .bind(before_status_change_count)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        status_changed_events.len(),
        1,
        "exactly 1 task_status_changed event"
    );

    let before_delete_count = after_status_change_count;

    // Phase 3: Task::delete emits exactly one TaskDeleted
    Task::delete(&pool, task.id).await.expect("delete failed");

    let after_delete_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM event_journal")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        after_delete_count - before_delete_count,
        1,
        "Task::delete should emit exactly 1 event"
    );

    // Verify it's a TaskDeleted event
    let deleted_events: Vec<String> = sqlx::query_scalar(
        "SELECT event_type FROM event_journal WHERE event_type = 'task_deleted' AND seq > ?",
    )
    .bind(before_delete_count)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(deleted_events.len(), 1, "exactly 1 task_deleted event");
}

// ==================
// Test 1b: breakdown_acceptance_emits_one_event_per_child
// ==================

#[tokio::test]
async fn breakdown_acceptance_emits_one_event_per_child() {
    let (pool, _temp_dir) = db::test_utils::create_test_pool_with_migrations().await;

    // Setup: create project and parent task
    let project_id = Uuid::new_v4();
    sqlx::query("INSERT INTO projects (id, name) VALUES (?, ?)")
        .bind(project_id)
        .bind("Test Project")
        .execute(&pool)
        .await
        .unwrap();

    let parent_task_id = Uuid::new_v4();
    let create_data = CreateTask {
        project_id,
        title: "Parent Task".to_string(),
        description: None,
        status: None,
        parent_task_id: None,
        image_ids: None,
        shared_task_id: None,
    };
    Task::create(&pool, &create_data, parent_task_id)
        .await
        .expect("parent task create failed");

    // Count events before proposal acceptance
    let initial_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM event_journal")
        .fetch_one(&pool)
        .await
        .unwrap();

    // Create a draft proposal with 3 items
    let proposal = task_breakdown::create(&pool, parent_task_id)
        .await
        .expect("proposal create failed");

    let items = vec![
        task_breakdown::ProposalItemInput {
            title: "Child 1".to_string(),
            description: None,
            sort_order: 0,
            depends_on_indices: vec![],
        },
        task_breakdown::ProposalItemInput {
            title: "Child 2".to_string(),
            description: None,
            sort_order: 1,
            depends_on_indices: vec![],
        },
        task_breakdown::ProposalItemInput {
            title: "Child 3".to_string(),
            description: None,
            sort_order: 2,
            depends_on_indices: vec![],
        },
    ];

    task_breakdown::replace_items(&pool, proposal.id, items)
        .await
        .expect("replace_items failed");

    // Accept the proposal
    let created_tasks = task_breakdown::accept_proposal(&pool, proposal.id)
        .await
        .expect("accept_proposal failed");

    // Verify exactly 3 child tasks were created
    assert_eq!(created_tasks.len(), 3, "proposal should create 3 children");

    let after_acceptance_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM event_journal")
        .fetch_one(&pool)
        .await
        .unwrap();

    // Accept emits exactly 3 events (one per child)
    assert_eq!(
        after_acceptance_count - initial_count,
        3,
        "breakdown acceptance should emit exactly 3 events (one per child)"
    );

    // Verify all 3 are TaskCreated events with the correct child IDs
    let created_child_ids: Vec<Uuid> = created_tasks.iter().map(|t| t.id).collect();
    let event_payloads: Vec<(String, String)> = sqlx::query_as(
        "SELECT event_type, payload FROM event_journal WHERE event_type = 'task_created' AND seq > ?",
    )
    .bind(initial_count)
    .fetch_all(&pool)
    .await
    .unwrap();

    let mut event_child_ids: Vec<(String, Uuid)> = Vec::new();
    for (event_type, payload) in event_payloads {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&payload)
            && let Some(task_id_str) = json.get("task_id").and_then(|v| v.as_str())
            && let Ok(task_id) = Uuid::parse_str(task_id_str)
        {
            event_child_ids.push((event_type, task_id));
        }
    }

    assert_eq!(
        event_child_ids.len(),
        3,
        "should have 3 task_created events"
    );
    for (event_type, event_task_id) in &event_child_ids {
        assert_eq!(event_type, "task_created");
        assert!(
            created_child_ids.contains(event_task_id),
            "event task_id should match one of the created children"
        );
    }

    // Verify the set of child IDs matches exactly
    let mut event_ids: Vec<Uuid> = event_child_ids.into_iter().map(|(_, id)| id).collect();
    event_ids.sort();
    let mut expected_ids = created_child_ids;
    expected_ids.sort();
    assert_eq!(
        event_ids, expected_ids,
        "child task IDs in events should match created tasks as a SET"
    );
}

// ==================
// Test 1c: remote_upsert_emits_exactly_one_event_each
// ==================

#[tokio::test]
async fn remote_upsert_emits_exactly_one_event_each() {
    let (pool, _temp_dir) = db::test_utils::create_test_pool_with_migrations().await;

    // Setup: create project
    let project_id = Uuid::new_v4();
    sqlx::query("INSERT INTO projects (id, name) VALUES (?, ?)")
        .bind(project_id)
        .bind("Test Project")
        .execute(&pool)
        .await
        .unwrap();

    // Case 1: Fresh shared_task_id upsert → exactly one TaskCreated
    let shared_task_id_1 = Uuid::new_v4();
    let initial_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM event_journal")
        .fetch_one(&pool)
        .await
        .unwrap();

    Task::upsert_remote_task(
        &pool,
        Uuid::new_v4(),
        project_id,
        shared_task_id_1,
        "Fresh Remote Task".to_string(),
        None,
        TaskStatus::Todo,
        None,
        None,
        None,
        1,
        None,
        None,
    )
    .await
    .expect("fresh upsert failed");

    let after_fresh_upsert: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM event_journal")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        after_fresh_upsert - initial_count,
        1,
        "fresh upsert should emit exactly 1 event"
    );

    // Verify it's a TaskCreated
    let fresh_events: Vec<String> = sqlx::query_scalar(
        "SELECT event_type FROM event_journal WHERE event_type = 'task_created' AND seq > ?",
    )
    .bind(initial_count)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(fresh_events.len(), 1, "exactly 1 task_created event");

    // Case 2: Version-bumped upsert with changed status → exactly one TaskStatusChanged
    let before_status_upsert = after_fresh_upsert;
    Task::upsert_remote_task(
        &pool,
        Uuid::new_v4(),
        project_id,
        shared_task_id_1,
        "Fresh Remote Task".to_string(),
        None,
        TaskStatus::InProgress,
        None,
        None,
        None,
        2,
        None,
        None,
    )
    .await
    .expect("status-changed upsert failed");

    let after_status_upsert: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM event_journal")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        after_status_upsert - before_status_upsert,
        1,
        "status-changed upsert should emit exactly 1 event"
    );

    // Verify it's a TaskStatusChanged and has correct old/new status
    let status_events: Vec<(String, String)> = sqlx::query_as(
        "SELECT event_type, payload FROM event_journal WHERE event_type = 'task_status_changed' AND seq > ?",
    )
    .bind(before_status_upsert)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        status_events.len(),
        1,
        "exactly 1 task_status_changed event"
    );

    // Parse payload to verify old/new status
    if let Ok(payload_json) = serde_json::from_str::<serde_json::Value>(&status_events[0].1) {
        let old_status = payload_json
            .get("old_status")
            .and_then(|v| v.as_str())
            .expect("old_status");
        let new_status = payload_json
            .get("new_status")
            .and_then(|v| v.as_str())
            .expect("new_status");
        assert_eq!(old_status, "todo", "old_status should be todo");
        assert_eq!(new_status, "inprogress", "new_status should be inprogress");
    } else {
        panic!("failed to parse payload JSON");
    }

    // Case 3: Same-version stale upsert → zero new events
    let before_stale_upsert = after_status_upsert;
    Task::upsert_remote_task(
        &pool,
        Uuid::new_v4(),
        project_id,
        shared_task_id_1,
        "Stale".to_string(),
        None,
        TaskStatus::Done,
        None,
        None,
        None,
        2, // Same version, should be rejected
        None,
        None,
    )
    .await
    .expect("stale upsert failed");

    let after_stale_upsert: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM event_journal")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        after_stale_upsert - before_stale_upsert,
        0,
        "stale upsert should emit zero events"
    );
}

// ==================
// Test 2: attempt_lifecycle_emits_exactly_one_event_each
// ==================

#[tokio::test]
async fn attempt_lifecycle_emits_exactly_one_event_each() {
    let (pool, _temp_dir) = db::test_utils::create_test_pool_with_migrations().await;

    // Setup: create project
    let project_id = Uuid::new_v4();
    sqlx::query("INSERT INTO projects (id, name) VALUES (?, ?)")
        .bind(project_id)
        .bind("Test Project")
        .execute(&pool)
        .await
        .unwrap();

    // Create task
    let task_id = Uuid::new_v4();
    let create_task_data = CreateTask {
        project_id,
        title: "Execution Test".to_string(),
        description: None,
        status: None,
        parent_task_id: None,
        image_ids: None,
        shared_task_id: None,
    };
    Task::create(&pool, &create_task_data, task_id)
        .await
        .expect("task create failed");

    // Create task attempt
    let attempt_id = Uuid::new_v4();
    sqlx::query(
        r#"INSERT INTO task_attempts
           (id, task_id, executor, branch, target_branch, container_ref)
           VALUES (?, ?, ?, ?, ?, ?)"#,
    )
    .bind(attempt_id)
    .bind(task_id)
    .bind("test_executor")
    .bind("test")
    .bind("main")
    .bind("/tmp/test")
    .execute(&pool)
    .await
    .unwrap();

    // Count before execution process creation
    let before_ep_create: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM event_journal")
        .fetch_one(&pool)
        .await
        .unwrap();

    // Create execution process (should emit AttemptStarted)
    let ep_id = Uuid::new_v4();
    let ep_data = CreateExecutionProcess {
        task_attempt_id: attempt_id,
        run_reason: ExecutionProcessRunReason::CodingAgent,
        executor_action: ExecutorAction::new(
            ExecutorActionType::CodingAgentInitialRequest(CodingAgentInitialRequest {
                prompt: "test".to_string(),
                executor_profile_id: ExecutorProfileId::new(BaseCodingAgent::ClaudeCode),
            }),
            None,
        ),
    };
    db::models::execution_process::ExecutionProcess::create(
        &pool,
        &ep_data,
        ep_id,
        Some("abc123"),
        None,
    )
    .await
    .expect("execution process create failed");

    let after_ep_create: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM event_journal")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        after_ep_create - before_ep_create,
        1,
        "ExecutionProcess::create should emit exactly 1 event (attempt_started)"
    );

    // Verify it's an attempt_started event
    let started_events: Vec<String> = sqlx::query_scalar(
        "SELECT event_type FROM event_journal WHERE event_type = 'attempt_started' AND seq > ?",
    )
    .bind(before_ep_create)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(started_events.len(), 1, "exactly 1 attempt_started event");

    // Test terminal state: Completed with exit code → AttemptFinished
    let before_completion: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM event_journal")
        .fetch_one(&pool)
        .await
        .unwrap();

    db::models::execution_process::ExecutionProcess::update_completion(
        &pool,
        ep_id,
        ExecutionProcessStatus::Completed,
        Some(0),
        None,
        None,
    )
    .await
    .expect("update_completion failed");

    let after_completion: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM event_journal")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        after_completion - before_completion,
        1,
        "update_completion should emit exactly 1 event"
    );

    // Verify it's an attempt_finished event
    let finished_events: Vec<(String, String)> = sqlx::query_as(
        "SELECT event_type, payload FROM event_journal WHERE event_type = 'attempt_finished' AND seq > ?",
    )
    .bind(before_completion)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(finished_events.len(), 1, "exactly 1 attempt_finished event");

    // Verify exit_code in payload
    if let Ok(payload_json) = serde_json::from_str::<serde_json::Value>(&finished_events[0].1) {
        let exit_code = payload_json
            .get("exit_code")
            .and_then(|v| v.as_i64())
            .expect("exit_code");
        assert_eq!(exit_code, 0, "exit_code should be 0");
    } else {
        panic!("failed to parse payload JSON");
    }
}

// ==================
// Test 4: no_duplicate_events_for_a_single_state_change
// ==================

#[tokio::test]
async fn no_duplicate_events_for_a_single_state_change() {
    let (pool, _temp_dir) = db::test_utils::create_test_pool_with_migrations().await;

    // Setup: project
    let project_id = Uuid::new_v4();
    sqlx::query("INSERT INTO projects (id, name) VALUES (?, ?)")
        .bind(project_id)
        .bind("Test Project")
        .execute(&pool)
        .await
        .unwrap();

    // Regression guard: verify that EACH emission site emits EXACTLY once per operation.
    // This guards against a site being instrumented at two layers simultaneously.

    // Task::create → exactly 1
    let before_create: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM event_journal")
        .fetch_one(&pool)
        .await
        .unwrap();
    let task_id = Uuid::new_v4();
    Task::create(
        &pool,
        &CreateTask {
            project_id,
            title: "Test".to_string(),
            description: None,
            status: None,
            parent_task_id: None,
            image_ids: None,
            shared_task_id: None,
        },
        task_id,
    )
    .await
    .unwrap();
    let after_create: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM event_journal")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        after_create - before_create,
        1,
        "Task::create delta must be 1"
    );

    // Task::update_status → exactly 1 (when status changes)
    let before_update: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM event_journal")
        .fetch_one(&pool)
        .await
        .unwrap();
    Task::update_status(&pool, task_id, TaskStatus::InProgress)
        .await
        .unwrap();
    let after_update: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM event_journal")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        after_update - before_update,
        1,
        "Task::update_status delta must be 1"
    );

    // Task::delete → exactly 1
    let before_delete: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM event_journal")
        .fetch_one(&pool)
        .await
        .unwrap();
    Task::delete(&pool, task_id).await.unwrap();
    let after_delete: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM event_journal")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        after_delete - before_delete,
        1,
        "Task::delete delta must be 1"
    );

    // breakdown accept_proposal → exactly 3 (one per child)
    let parent_task_id = Uuid::new_v4();
    Task::create(
        &pool,
        &CreateTask {
            project_id,
            title: "Parent".to_string(),
            description: None,
            status: None,
            parent_task_id: None,
            image_ids: None,
            shared_task_id: None,
        },
        parent_task_id,
    )
    .await
    .unwrap();

    let proposal = task_breakdown::create(&pool, parent_task_id).await.unwrap();
    task_breakdown::replace_items(
        &pool,
        proposal.id,
        vec![
            task_breakdown::ProposalItemInput {
                title: "C1".to_string(),
                description: None,
                sort_order: 0,
                depends_on_indices: vec![],
            },
            task_breakdown::ProposalItemInput {
                title: "C2".to_string(),
                description: None,
                sort_order: 1,
                depends_on_indices: vec![],
            },
            task_breakdown::ProposalItemInput {
                title: "C3".to_string(),
                description: None,
                sort_order: 2,
                depends_on_indices: vec![],
            },
        ],
    )
    .await
    .unwrap();

    let before_accept: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM event_journal")
        .fetch_one(&pool)
        .await
        .unwrap();
    task_breakdown::accept_proposal(&pool, proposal.id)
        .await
        .unwrap();
    let after_accept: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM event_journal")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        after_accept - before_accept,
        3,
        "breakdown accept delta must be 3 (one per child)"
    );

    // Task::upsert_remote_task (fresh) → exactly 1
    let shared_task_id = Uuid::new_v4();
    let before_remote_fresh: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM event_journal")
        .fetch_one(&pool)
        .await
        .unwrap();
    Task::upsert_remote_task(
        &pool,
        Uuid::new_v4(),
        project_id,
        shared_task_id,
        "Remote".to_string(),
        None,
        TaskStatus::Todo,
        None,
        None,
        None,
        1,
        None,
        None,
    )
    .await
    .unwrap();
    let after_remote_fresh: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM event_journal")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        after_remote_fresh - before_remote_fresh,
        1,
        "Task::upsert_remote_task (fresh) delta must be 1"
    );

    // ExecutionProcess::create → exactly 1
    let attempt_id = Uuid::new_v4();
    sqlx::query(
        r#"INSERT INTO task_attempts
           (id, task_id, executor, branch, target_branch, container_ref)
           VALUES (?, ?, ?, ?, ?, ?)"#,
    )
    .bind(attempt_id)
    .bind(parent_task_id)
    .bind("test_executor")
    .bind("test")
    .bind("main")
    .bind("/tmp/test")
    .execute(&pool)
    .await
    .unwrap();

    let before_ep_create: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM event_journal")
        .fetch_one(&pool)
        .await
        .unwrap();
    db::models::execution_process::ExecutionProcess::create(
        &pool,
        &CreateExecutionProcess {
            task_attempt_id: attempt_id,
            run_reason: ExecutionProcessRunReason::CodingAgent,
            executor_action: ExecutorAction::new(
                ExecutorActionType::CodingAgentInitialRequest(CodingAgentInitialRequest {
                    prompt: "test".to_string(),
                    executor_profile_id: ExecutorProfileId::new(BaseCodingAgent::ClaudeCode),
                }),
                None,
            ),
        },
        Uuid::new_v4(),
        Some("abc"),
        None,
    )
    .await
    .unwrap();
    let after_ep_create: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM event_journal")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        after_ep_create - before_ep_create,
        1,
        "ExecutionProcess::create delta must be 1"
    );

    // ExecutionProcess::update_completion (terminal) → exactly 1
    let ep_row: (Uuid,) =
        sqlx::query_as("SELECT id FROM execution_processes WHERE task_attempt_id = ? LIMIT 1")
            .bind(attempt_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    let ep_id = ep_row.0;

    let before_completion: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM event_journal")
        .fetch_one(&pool)
        .await
        .unwrap();
    db::models::execution_process::ExecutionProcess::update_completion(
        &pool,
        ep_id,
        ExecutionProcessStatus::Completed,
        Some(0),
        None,
        None,
    )
    .await
    .unwrap();
    let after_completion: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM event_journal")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        after_completion - before_completion,
        1,
        "ExecutionProcess::update_completion delta must be 1"
    );
}

// ==================
// Test 5: every_emitted_event_type_round_trips_from_the_journal
// ==================

#[tokio::test]
async fn every_emitted_event_type_round_trips_from_the_journal() {
    let (pool, _temp_dir) = db::test_utils::create_test_pool_with_migrations().await;

    // Setup: project
    let project_id = Uuid::new_v4();
    sqlx::query("INSERT INTO projects (id, name) VALUES (?, ?)")
        .bind(project_id)
        .bind("Test Project")
        .execute(&pool)
        .await
        .unwrap();

    // Drive all emission sites once each to populate the journal
    // (reusing setup from earlier tests)

    // Task::create
    let task_id = Uuid::new_v4();
    Task::create(
        &pool,
        &CreateTask {
            project_id,
            title: "Test".to_string(),
            description: None,
            status: None,
            parent_task_id: None,
            image_ids: None,
            shared_task_id: None,
        },
        task_id,
    )
    .await
    .unwrap();

    // Task::update_status
    Task::update_status(&pool, task_id, TaskStatus::InProgress)
        .await
        .unwrap();

    // Task::delete
    Task::delete(&pool, task_id).await.unwrap();

    // breakdown accept_proposal
    let parent_task_id = Uuid::new_v4();
    Task::create(
        &pool,
        &CreateTask {
            project_id,
            title: "Parent".to_string(),
            description: None,
            status: None,
            parent_task_id: None,
            image_ids: None,
            shared_task_id: None,
        },
        parent_task_id,
    )
    .await
    .unwrap();

    let proposal = task_breakdown::create(&pool, parent_task_id).await.unwrap();
    task_breakdown::replace_items(
        &pool,
        proposal.id,
        vec![
            task_breakdown::ProposalItemInput {
                title: "C1".to_string(),
                description: None,
                sort_order: 0,
                depends_on_indices: vec![],
            },
            task_breakdown::ProposalItemInput {
                title: "C2".to_string(),
                description: None,
                sort_order: 1,
                depends_on_indices: vec![],
            },
        ],
    )
    .await
    .unwrap();
    task_breakdown::accept_proposal(&pool, proposal.id)
        .await
        .unwrap();

    // Task::upsert_remote_task
    Task::upsert_remote_task(
        &pool,
        Uuid::new_v4(),
        project_id,
        Uuid::new_v4(),
        "Remote".to_string(),
        None,
        TaskStatus::Todo,
        None,
        None,
        None,
        1,
        None,
        None,
    )
    .await
    .unwrap();

    // ExecutionProcess lifecycle
    let attempt_id = Uuid::new_v4();
    sqlx::query(
        r#"INSERT INTO task_attempts
           (id, task_id, executor, branch, target_branch, container_ref)
           VALUES (?, ?, ?, ?, ?, ?)"#,
    )
    .bind(attempt_id)
    .bind(parent_task_id)
    .bind("test_executor")
    .bind("test")
    .bind("main")
    .bind("/tmp/test")
    .execute(&pool)
    .await
    .unwrap();

    let ep_id = Uuid::new_v4();
    db::models::execution_process::ExecutionProcess::create(
        &pool,
        &CreateExecutionProcess {
            task_attempt_id: attempt_id,
            run_reason: ExecutionProcessRunReason::CodingAgent,
            executor_action: ExecutorAction::new(
                ExecutorActionType::CodingAgentInitialRequest(CodingAgentInitialRequest {
                    prompt: "test".to_string(),
                    executor_profile_id: ExecutorProfileId::new(BaseCodingAgent::ClaudeCode),
                }),
                None,
            ),
        },
        ep_id,
        Some("abc"),
        None,
    )
    .await
    .unwrap();

    db::models::execution_process::ExecutionProcess::update_completion(
        &pool,
        ep_id,
        ExecutionProcessStatus::Completed,
        Some(0),
        None,
        None,
    )
    .await
    .unwrap();

    // Now read every journal row and verify round-trip
    let rows: Vec<(String, String)> =
        sqlx::query_as("SELECT event_type, payload FROM event_journal")
            .fetch_all(&pool)
            .await
            .unwrap();

    assert!(!rows.is_empty(), "should have emitted at least one event");

    for (stored_event_type, payload_str) in rows {
        // Parse the payload back into a NodeEvent
        let node_event: NodeEvent = serde_json::from_str(&payload_str)
            .unwrap_or_else(|_| panic!("failed to parse payload: {}", payload_str));

        // Verify the event_type() method matches the stored event_type
        let computed_event_type = node_event.event_type();
        assert_eq!(
            computed_event_type, stored_event_type,
            "event_type() mismatch: computed={}, stored={}",
            computed_event_type, stored_event_type
        );
    }
}
