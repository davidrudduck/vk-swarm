//! Task breakdown service: decomposing goals into independently executable subtasks.
//!
//! This service manages Claude-based breakdown proposals, including prompt generation,
//! result parsing, and persistence to the database.

use db::models::task_breakdown::{self, BreakdownStatus, ProposalItemInput, TaskBreakdownProposal};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use thiserror::Error;
use utils::log_msg::LogMsg;
use uuid::Uuid;

/// Stateless service for task breakdown operations.
#[derive(Clone)]
pub struct BreakdownService;

/// A breakdown result deserialized from Claude's fenced JSON block.
#[derive(Debug, Serialize, Deserialize)]
pub struct BreakdownResult {
    pub subtasks: Vec<BreakdownSubtask>,
}

/// A single subtask within a breakdown result.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BreakdownSubtask {
    pub title: String,
    pub description: Option<String>,
    #[serde(default)]
    pub depends_on: Vec<usize>,
}

/// Number of distinct values in `values`. Dependency lists are tiny (a handful
/// of indices), so the quadratic scan is cheaper than allocating a set.
fn distinct_count(values: &[usize]) -> usize {
    values
        .iter()
        .enumerate()
        .filter(|(i, v)| !values[..*i].contains(v))
        .count()
}

/// Drop repeated indices from `values`, preserving first-seen order.
fn dedupe_preserving_order(values: &mut Vec<usize>) {
    let mut seen: Vec<usize> = Vec::with_capacity(values.len());
    values.retain(|v| {
        if seen.contains(v) {
            false
        } else {
            seen.push(*v);
            true
        }
    });
}

/// True when the `depends_on` edges contain a cycle.
///
/// Kahn's algorithm: repeatedly remove a node with no outstanding dependencies.
/// If any node is left unremoved, it sits on (or behind) a cycle. Assumes every
/// index in `depends_on` is already in range — callers validate that first.
///
/// The in-degree is the count of DISTINCT dependencies, not `depends_on.len()`:
/// the decrement below is driven by `.contains()`, which fires once per node
/// however many times that node is listed. Seeding from the raw length would
/// leave a duplicate edge (`[0, 0]`) permanently outstanding and report an
/// acyclic set as cyclic. Callers dedupe before persisting, so this is the
/// belt-and-braces half of that fix.
fn has_dependency_cycle(subtasks: &[BreakdownSubtask]) -> bool {
    let len = subtasks.len();
    let mut remaining: Vec<usize> = subtasks
        .iter()
        .map(|s| distinct_count(&s.depends_on))
        .collect();
    let mut queue: Vec<usize> = (0..len).filter(|&i| remaining[i] == 0).collect();
    let mut resolved = 0usize;

    while let Some(node) = queue.pop() {
        resolved += 1;
        for (i, subtask) in subtasks.iter().enumerate() {
            if subtask.depends_on.contains(&node) {
                remaining[i] -= 1;
                if remaining[i] == 0 {
                    queue.push(i);
                }
            }
        }
    }

    resolved != len
}

/// Errors that can occur during breakdown operations.
#[derive(Debug, Error)]
pub enum BreakdownError {
    #[error("No JSON result block found in Claude's output")]
    NoResult,

    #[error("Breakdown result contains zero subtasks")]
    Empty,

    #[error("Breakdown result contains fewer than 2 subtasks")]
    TooFew,

    #[error("At least one subtask has an empty title")]
    EmptyTitle,

    #[error("Invalid dependency reference in subtask")]
    InvalidDependency,

    #[error("Subtask dependencies form a cycle")]
    CyclicDependency,

    #[error("Database error: {0}")]
    Db(#[from] sqlx::Error),

    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),
}

impl BreakdownService {
    /// Generate the prompt template for Claude to decompose a goal into subtasks.
    ///
    /// The prompt enforces:
    /// - 2–10 subtasks (read-only analysis, no file modifications)
    /// - Dependencies via zero-based indices
    /// - JSON block as the final output element
    pub fn breakdown_prompt(title: &str, description: &str) -> String {
        format!(
            "You are decomposing a development goal into independently executable subtasks.\n\
             GOAL TITLE: {}\n\
             GOAL DESCRIPTION: {}\n\n\
             Rules: propose 2-10 subtasks, each independently executable; use depends_on (array of zero-based indices into your own list) only for true prerequisites; DO NOT modify, create, or delete any files — this is read-only analysis.\n\n\
             Respond with EXACTLY ONE fenced json code block as the FINAL element of your reply, matching:\n\
             {{\"subtasks\":[{{\"title\":\"...\",\"description\":\"...\",\"depends_on\":[0]}}]}}",
            title, description
        )
    }

    /// Parse Claude's output to extract and validate a breakdown result.
    ///
    /// Two-stage parsing:
    /// 1. Substitute lines: each line that is a JSON object with "type":"result" is replaced
    ///    with its "result" field (unwrapping stream-JSON format).
    /// 2. Scan for fenced blocks: collect ALL ```json...``` blocks and deserialize the
    ///    LAST one that successfully parses into a BreakdownResult.
    ///
    /// Validation:
    /// - ≥2 subtasks (0 → Empty, 1 → TooFew)
    /// - Non-empty titles (→ EmptyTitle)
    /// - All depends_on indices in range and != self (→ InvalidDependency)
    /// - No dependency cycles (→ CyclicDependency)
    /// - >10 subtasks allowed (lenient upper bound)
    pub fn parse_breakdown_result(
        stdout_lines: &[String],
    ) -> Result<BreakdownResult, BreakdownError> {
        // Stage 1: Substitute stream-JSON lines.
        // The Claude executor's protocol reader breaks on the final {"type":"result"} line
        // WITHOUT forwarding it to the log store (protocol.rs), so in production the fenced
        // JSON block is only reachable inside {"type":"assistant"} events, JSON-escaped in
        // message.content[].text. Substitute both shapes (DV-4).
        let mut text = String::new();
        for line in stdout_lines {
            if let Ok(obj) = serde_json::from_str::<serde_json::Value>(line) {
                match obj.get("type").and_then(|v| v.as_str()) {
                    Some("result") => {
                        if let Some(result) = obj.get("result").and_then(|v| v.as_str()) {
                            text.push_str(result);
                            text.push('\n');
                            continue;
                        }
                    }
                    Some("assistant") => {
                        if let Some(content) = obj
                            .get("message")
                            .and_then(|m| m.get("content"))
                            .and_then(|c| c.as_array())
                        {
                            for part in content {
                                if part.get("type").and_then(|v| v.as_str()) == Some("text")
                                    && let Some(t) = part.get("text").and_then(|v| v.as_str())
                                {
                                    text.push_str(t);
                                    text.push('\n');
                                }
                            }
                            continue;
                        }
                    }
                    _ => {}
                }
            }
            text.push_str(line);
            text.push('\n');
        }

        // Stage 2: Collect ALL fenced ```json blocks; use the LAST one that deserializes
        let mut blocks: Vec<String> = Vec::new();
        let mut in_block = false;
        let mut current_block = String::new();

        for line in text.lines() {
            if line.trim().starts_with("```json") {
                if in_block {
                    // Close current block if we encounter another opening (shouldn't happen, but defensive)
                    blocks.push(current_block.clone());
                    current_block.clear();
                }
                in_block = true;
            } else if in_block && line.trim().starts_with("```") {
                in_block = false;
                blocks.push(current_block.clone());
                current_block.clear();
            } else if in_block {
                current_block.push_str(line);
                current_block.push('\n');
            }
        }

        if blocks.is_empty() {
            return Err(BreakdownError::NoResult);
        }

        // Iterate from the last block backwards, taking the first that deserializes.
        // If none deserializes, surface the Json error from the last attempted block.
        let mut last_err: Option<serde_json::Error> = None;
        let mut parsed: Option<BreakdownResult> = None;
        for block in blocks.iter().rev() {
            match serde_json::from_str::<BreakdownResult>(block) {
                Ok(r) => {
                    parsed = Some(r);
                    break;
                }
                Err(e) => {
                    if last_err.is_none() {
                        last_err = Some(e);
                    }
                }
            }
        }
        let mut result = match parsed {
            Some(r) => r,
            None => return Err(BreakdownError::Json(last_err.expect("blocks is non-empty"))),
        };

        // Validate
        if result.subtasks.is_empty() {
            return Err(BreakdownError::Empty);
        }
        if result.subtasks.len() < 2 {
            return Err(BreakdownError::TooFew);
        }
        for subtask in &result.subtasks {
            if subtask.title.trim().is_empty() {
                return Err(BreakdownError::EmptyTitle);
            }
        }
        let len = result.subtasks.len();
        for (i, subtask) in result.subtasks.iter().enumerate() {
            for &dep in &subtask.depends_on {
                if dep >= len || dep == i {
                    return Err(BreakdownError::InvalidDependency);
                }
            }
        }
        // Dedupe before the cycle check and before persistence. An agent listing
        // the same dependency twice is expressing one edge, not a defect: rejecting
        // the whole run would throw away a usable draft the operator could have
        // edited. task_dependencies is PRIMARY KEY (task_id, depends_on_task_id),
        // so a duplicate that reached accept_proposal would abort the accept
        // transaction on a UNIQUE violation.
        for subtask in &mut result.subtasks {
            dedupe_preserving_order(&mut subtask.depends_on);
        }

        // Range and self-reference checks above do not catch a mutual pair
        // (0 -> 1, 1 -> 0). accept_proposal writes every depends_on edge into
        // task_dependencies, so a cycle here becomes a cyclic graph on real tasks.
        if has_dependency_cycle(&result.subtasks) {
            return Err(BreakdownError::CyclicDependency);
        }

        Ok(result)
    }

    /// Persist a parsed breakdown result to the database.
    ///
    /// Maps each subtask to a ProposalItemInput (sort_order = index as i64,
    /// depends_on_indices from the parsed depends_on vector) and calls
    /// task_breakdown::replace_items. Does not change proposal status (stays draft).
    pub async fn persist_result(
        pool: &SqlitePool,
        proposal_id: Uuid,
        result: &BreakdownResult,
    ) -> Result<(), BreakdownError> {
        let items: Vec<ProposalItemInput> = result
            .subtasks
            .iter()
            .enumerate()
            .map(|(index, subtask)| ProposalItemInput {
                title: subtask.title.clone(),
                description: subtask.description.clone(),
                sort_order: index as i64,
                depends_on_indices: subtask.depends_on.iter().map(|&i| i as i64).collect(),
            })
            .collect();

        task_breakdown::replace_items(pool, proposal_id, items)
            .await
            .map(|_| ())
            .map_err(BreakdownError::Db)
    }

    /// Mark a proposal as failed with an error message.
    ///
    /// Updates the proposal status to Failed and stores the error text.
    pub async fn fail_proposal(
        pool: &SqlitePool,
        proposal_id: Uuid,
        error_text: String,
    ) -> Result<TaskBreakdownProposal, BreakdownError> {
        task_breakdown::update_status(pool, proposal_id, BreakdownStatus::Failed, Some(error_text))
            .await
            .map_err(BreakdownError::Db)
    }

    /// Convert raw stdout CHUNKS into logical lines.
    ///
    /// LogMsg::Stdout payloads are arbitrary chunks whose boundaries need not align
    /// with line boundaries, so we concatenate everything and split on '\n'.
    pub fn chunks_to_lines(chunks: Vec<String>) -> Vec<String> {
        let mut buffer = String::new();
        for chunk in chunks {
            buffer.push_str(&chunk);
        }
        buffer.split('\n').map(|s| s.to_string()).collect()
    }

    /// Extract and parse stdout lines from an execution process's logs.
    ///
    /// Retrieves logs via ExecutionProcessLogs::find_by_execution_id, parses them
    /// into LogMsg entries, concatenates all Stdout chunk payloads, and splits the
    /// combined buffer on '\n' (chunk boundaries need not align with line boundaries).
    pub async fn extract_stdout_lines(
        pool: &SqlitePool,
        execution_process_id: Uuid,
    ) -> Result<Vec<String>, BreakdownError> {
        use db::models::execution_process_logs::ExecutionProcessLogs;

        let records = ExecutionProcessLogs::find_by_execution_id(pool, execution_process_id)
            .await
            .map_err(BreakdownError::Db)?;

        let messages = ExecutionProcessLogs::parse_logs(&records)?;

        let mut chunks = Vec::new();
        for msg in messages {
            if let LogMsg::Stdout(chunk) = msg {
                chunks.push(chunk);
            }
        }

        Ok(Self::chunks_to_lines(chunks))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use db::models::execution_process_logs::ExecutionProcessLogs;
    use db::models::project::{CreateProject, Project};
    use db::models::task::{CreateTask, Task};
    use db::test_utils::create_test_pool;

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

    async fn create_proposal(pool: &SqlitePool) -> Uuid {
        let project_id = create_project(pool).await;
        let task_id = create_task(pool, project_id).await;
        task_breakdown::create(pool, task_id)
            .await
            .expect("create proposal")
            .id
    }

    async fn create_execution_process(pool: &SqlitePool) -> Uuid {
        let project_id = create_project(pool).await;
        let task_id = create_task(pool, project_id).await;
        let attempt_id = Uuid::new_v4();
        sqlx::query(
            r#"INSERT INTO task_attempts (id, task_id, executor, branch, target_branch, container_ref)
               VALUES ($1, $2, 'CLAUDE_CODE', 'test-branch', 'main', '/tmp/test-worktree')"#,
        )
        .bind(attempt_id)
        .bind(task_id)
        .execute(pool)
        .await
        .expect("create attempt");

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

    fn subtask(title: &str, depends_on: Vec<usize>) -> BreakdownSubtask {
        BreakdownSubtask {
            title: title.to_string(),
            description: Some(format!("{title} description")),
            depends_on,
        }
    }

    #[tokio::test]
    async fn test_persist_result_happy_path_preserves_order_and_deps() {
        let (pool, _temp_dir) = create_test_pool().await;
        let proposal_id = create_proposal(&pool).await;

        let result = BreakdownResult {
            subtasks: vec![
                subtask("First", vec![]),
                subtask("Second", vec![0]),
                subtask("Third", vec![0, 1]),
            ],
        };

        BreakdownService::persist_result(&pool, proposal_id, &result)
            .await
            .expect("persist_result");

        let items = task_breakdown::find_items(&pool, proposal_id)
            .await
            .expect("find_items");
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].title, "First");
        assert_eq!(items[0].description.as_deref(), Some("First description"));
        assert_eq!(items[0].sort_order, 0);
        assert_eq!(items[1].title, "Second");
        assert_eq!(items[1].sort_order, 1);
        assert_eq!(items[2].title, "Third");
        assert_eq!(items[2].sort_order, 2);

        // depends_on_item_ids stores resolved item UUIDs, not raw indices.
        let dep_ids: Vec<Uuid> = serde_json::from_str(&items[1].depends_on_item_ids).unwrap();
        assert_eq!(dep_ids, vec![items[0].id]);
        let dep_ids_third: Vec<Uuid> = serde_json::from_str(&items[2].depends_on_item_ids).unwrap();
        assert_eq!(dep_ids_third, vec![items[0].id, items[1].id]);
    }

    #[tokio::test]
    async fn test_persist_result_replaces_existing_items() {
        let (pool, _temp_dir) = create_test_pool().await;
        let proposal_id = create_proposal(&pool).await;

        let first = BreakdownResult {
            subtasks: vec![subtask("Old A", vec![]), subtask("Old B", vec![])],
        };
        BreakdownService::persist_result(&pool, proposal_id, &first)
            .await
            .expect("first persist_result");
        assert_eq!(
            task_breakdown::find_items(&pool, proposal_id)
                .await
                .unwrap()
                .len(),
            2
        );

        let second = BreakdownResult {
            subtasks: vec![subtask("New A", vec![])],
        };
        BreakdownService::persist_result(&pool, proposal_id, &second)
            .await
            .expect("second persist_result replaces items");

        let items = task_breakdown::find_items(&pool, proposal_id)
            .await
            .unwrap();
        assert_eq!(items.len(), 1, "old items are replaced, not appended");
        assert_eq!(items[0].title, "New A");
    }

    #[tokio::test]
    async fn test_persist_result_nonexistent_proposal_errors_row_not_found() {
        let (pool, _temp_dir) = create_test_pool().await;
        let result = BreakdownResult {
            subtasks: vec![subtask("A", vec![])],
        };

        let err = BreakdownService::persist_result(&pool, Uuid::new_v4(), &result)
            .await
            .expect_err("nonexistent proposal must error");
        assert!(matches!(err, BreakdownError::Db(sqlx::Error::RowNotFound)));
    }

    #[tokio::test]
    async fn test_fail_proposal_happy_path_stores_status_and_error() {
        let (pool, _temp_dir) = create_test_pool().await;
        let proposal_id = create_proposal(&pool).await;

        let updated =
            BreakdownService::fail_proposal(&pool, proposal_id, "Claude timed out".to_string())
                .await
                .expect("fail_proposal");

        assert_eq!(updated.status, BreakdownStatus::Failed);
        assert_eq!(updated.error.as_deref(), Some("Claude timed out"));

        let reread = task_breakdown::find_by_id(&pool, proposal_id)
            .await
            .unwrap()
            .expect("proposal exists");
        assert_eq!(reread.status, BreakdownStatus::Failed);
        assert_eq!(reread.error.as_deref(), Some("Claude timed out"));
    }

    #[tokio::test]
    async fn test_fail_proposal_nonexistent_proposal_errors_row_not_found() {
        let (pool, _temp_dir) = create_test_pool().await;
        let err = BreakdownService::fail_proposal(&pool, Uuid::new_v4(), "boom".to_string())
            .await
            .expect_err("nonexistent proposal must error");
        assert!(matches!(err, BreakdownError::Db(sqlx::Error::RowNotFound)));
    }

    #[tokio::test]
    async fn test_extract_stdout_lines_reassembles_split_line_across_chunks() {
        let (pool, _temp_dir) = create_test_pool().await;
        let execution_id = create_execution_process(&pool).await;

        // Two log rows (as would be inserted by separate append_log_line calls),
        // whose Stdout payloads split a logical line mid-way.
        let first_chunk =
            ExecutionProcessLogs::serialize_logs(&[LogMsg::Stdout("part-of-line".to_string())])
                .unwrap();
        ExecutionProcessLogs::append_log_line(&pool, execution_id, &first_chunk)
            .await
            .expect("append first chunk");

        let second_chunk = ExecutionProcessLogs::serialize_logs(&[LogMsg::Stdout(
            "-completed\nfull second line".to_string(),
        )])
        .unwrap();
        ExecutionProcessLogs::append_log_line(&pool, execution_id, &second_chunk)
            .await
            .expect("append second chunk");

        let lines = BreakdownService::extract_stdout_lines(&pool, execution_id)
            .await
            .expect("extract_stdout_lines");

        assert_eq!(lines[0], "part-of-line-completed");
        assert_eq!(lines[1], "full second line");
    }

    #[tokio::test]
    async fn test_extract_stdout_lines_no_logs_returns_single_empty_line() {
        let (pool, _temp_dir) = create_test_pool().await;

        // No rows exist for this execution id: find_by_execution_id returns an
        // empty Vec, so chunks are empty and "".split('\n') yields one empty
        // element — extract_stdout_lines does NOT error, and does NOT return
        // an empty Vec; it returns a single empty-string line.
        let lines = BreakdownService::extract_stdout_lines(&pool, Uuid::new_v4())
            .await
            .expect("no logs is not an error");
        assert_eq!(lines, vec!["".to_string()]);
    }

    #[tokio::test]
    async fn test_extract_stdout_lines_ignores_non_stdout_entries() {
        let (pool, _temp_dir) = create_test_pool().await;
        let execution_id = create_execution_process(&pool).await;

        let chunk = ExecutionProcessLogs::serialize_logs(&[
            LogMsg::Stdout("keep me\n".to_string()),
            LogMsg::Stderr("drop me (stderr)".to_string()),
            LogMsg::SessionId("sess-1".to_string()),
            LogMsg::Finished,
        ])
        .unwrap();
        ExecutionProcessLogs::append_log_line(&pool, execution_id, &chunk)
            .await
            .expect("append chunk");

        let lines = BreakdownService::extract_stdout_lines(&pool, execution_id)
            .await
            .expect("extract_stdout_lines");

        assert_eq!(lines, vec!["keep me".to_string(), "".to_string()]);
        assert!(!lines.iter().any(|l| l.contains("drop me")));
    }

    #[test]
    fn test_parse_last_fenced_json_block() {
        let lines = vec![
            "Some prose before the block".to_string(),
            "```json".to_string(),
            "{\"malformed\": invalid json}".to_string(),
            "```".to_string(),
            "More prose".to_string(),
            "```json".to_string(),
            "{\"subtasks\":[{\"title\":\"A\",\"description\":\"a\",\"depends_on\":[]},{\"title\":\"B\",\"description\":null,\"depends_on\":[0]}]}".to_string(),
            "```".to_string(),
        ];

        let result = BreakdownService::parse_breakdown_result(&lines).unwrap();
        assert_eq!(result.subtasks.len(), 2);
        assert_eq!(result.subtasks[0].title, "A");
        assert_eq!(result.subtasks[1].title, "B");
        assert_eq!(result.subtasks[1].depends_on, vec![0]);
    }

    #[test]
    fn test_parse_missing_block_errs() {
        let lines = vec![
            "Some prose without any fenced JSON block".to_string(),
            "No JSON here".to_string(),
        ];

        let result = BreakdownService::parse_breakdown_result(&lines);
        assert!(matches!(result, Err(BreakdownError::NoResult)));
    }

    #[test]
    fn test_parse_rejects_bad_indices() {
        // Test out-of-range dependency
        let lines = vec![
            "```json".to_string(),
            "{\"subtasks\":[{\"title\":\"A\",\"description\":\"a\",\"depends_on\":[5]},{\"title\":\"B\",\"description\":null,\"depends_on\":[]}]}".to_string(),
            "```".to_string(),
        ];
        let result = BreakdownService::parse_breakdown_result(&lines);
        assert!(matches!(result, Err(BreakdownError::InvalidDependency)));

        // Test self-reference
        let lines = vec![
            "```json".to_string(),
            "{\"subtasks\":[{\"title\":\"A\",\"description\":\"a\",\"depends_on\":[0]},{\"title\":\"B\",\"description\":null,\"depends_on\":[]}]}".to_string(),
            "```".to_string(),
        ];
        let result = BreakdownService::parse_breakdown_result(&lines);
        assert!(matches!(result, Err(BreakdownError::InvalidDependency)));
    }

    #[test]
    fn test_parse_rejects_dependency_cycle() {
        // A mutual pair: every index is in range and none is self-referential, so the
        // range/self checks pass. Only the cycle check catches this.
        let lines = vec![
            "```json".to_string(),
            "{\"subtasks\":[{\"title\":\"A\",\"description\":null,\"depends_on\":[1]},{\"title\":\"B\",\"description\":null,\"depends_on\":[0]}]}".to_string(),
            "```".to_string(),
        ];
        let result = BreakdownService::parse_breakdown_result(&lines);
        assert!(matches!(result, Err(BreakdownError::CyclicDependency)));
    }

    #[test]
    fn test_parse_rejects_longer_dependency_cycle() {
        // 0 -> 1 -> 2 -> 0, plus an acyclic node that must not mask the cycle.
        let lines = vec![
            "```json".to_string(),
            "{\"subtasks\":[{\"title\":\"A\",\"description\":null,\"depends_on\":[2]},{\"title\":\"B\",\"description\":null,\"depends_on\":[0]},{\"title\":\"C\",\"description\":null,\"depends_on\":[1]},{\"title\":\"D\",\"description\":null,\"depends_on\":[]}]}".to_string(),
            "```".to_string(),
        ];
        let result = BreakdownService::parse_breakdown_result(&lines);
        assert!(matches!(result, Err(BreakdownError::CyclicDependency)));
    }

    #[test]
    fn test_parse_accepts_diamond_dependencies() {
        // A diamond (0 -> 1, 0 -> 2, 1&2 -> 3) is a DAG, not a cycle: it must pass.
        let lines = vec![
            "```json".to_string(),
            "{\"subtasks\":[{\"title\":\"A\",\"description\":null,\"depends_on\":[]},{\"title\":\"B\",\"description\":null,\"depends_on\":[0]},{\"title\":\"C\",\"description\":null,\"depends_on\":[0]},{\"title\":\"D\",\"description\":null,\"depends_on\":[1,2]}]}".to_string(),
            "```".to_string(),
        ];
        let result = BreakdownService::parse_breakdown_result(&lines).expect("diamond is a DAG");
        assert_eq!(result.subtasks.len(), 4);
    }

    #[test]
    fn test_parse_dedupes_duplicated_dependency() {
        // An agent listing the same dependency twice is expressing one edge on an
        // acyclic set. Seeding Kahn's in-degree from `depends_on.len()` while
        // decrementing on `.contains()` left it permanently outstanding, so the whole
        // run was rejected as cyclic — and a duplicate reaching accept_proposal would
        // violate the task_dependencies primary key.
        let lines = vec![
            "```json".to_string(),
            "{\"subtasks\":[{\"title\":\"A\",\"description\":null,\"depends_on\":[]},{\"title\":\"B\",\"description\":null,\"depends_on\":[0,0]}]}".to_string(),
            "```".to_string(),
        ];
        let result = BreakdownService::parse_breakdown_result(&lines)
            .expect("a duplicated dependency is not a cycle");
        assert_eq!(
            result.subtasks[1].depends_on,
            vec![0],
            "the duplicate is collapsed to a single edge before persistence"
        );
    }

    #[test]
    fn test_has_dependency_cycle_counts_distinct_dependencies() {
        // Pins the in-degree fix directly: dedupe at ingest makes this unreachable
        // from parse_breakdown_result, so without this test the counting change would
        // be unpinned.
        let subtasks = vec![
            BreakdownSubtask {
                title: "A".to_string(),
                description: None,
                depends_on: vec![],
            },
            BreakdownSubtask {
                title: "B".to_string(),
                description: None,
                depends_on: vec![0, 0],
            },
        ];
        assert!(
            !has_dependency_cycle(&subtasks),
            "a repeated edge on an acyclic set is not a cycle"
        );
    }

    #[test]
    fn test_parse_rejects_empty() {
        // Empty subtasks list
        let lines = vec![
            "```json".to_string(),
            "{\"subtasks\":[]}".to_string(),
            "```".to_string(),
        ];
        let result = BreakdownService::parse_breakdown_result(&lines);
        assert!(matches!(result, Err(BreakdownError::Empty)));

        // Empty title
        let lines = vec![
            "```json".to_string(),
            "{\"subtasks\":[{\"title\":\"\",\"description\":\"a\",\"depends_on\":[]},{\"title\":\"B\",\"description\":null,\"depends_on\":[]}]}".to_string(),
            "```".to_string(),
        ];
        let result = BreakdownService::parse_breakdown_result(&lines);
        assert!(matches!(result, Err(BreakdownError::EmptyTitle)));

        // Single subtask
        let lines = vec![
            "```json".to_string(),
            "{\"subtasks\":[{\"title\":\"A\",\"description\":\"a\",\"depends_on\":[]}]}"
                .to_string(),
            "```".to_string(),
        ];
        let result = BreakdownService::parse_breakdown_result(&lines);
        assert!(matches!(result, Err(BreakdownError::TooFew)));

        // 11 subtasks (allowed, upper bound is lenient)
        let subtasks = (0..11)
            .map(|i| format!(r#"{{"title":"T{}","description":null,"depends_on":[]}}"#, i))
            .collect::<Vec<_>>()
            .join(",");
        let lines = vec![
            "```json".to_string(),
            format!(r#"{{"subtasks":[{}]}}"#, subtasks),
            "```".to_string(),
        ];
        let result = BreakdownService::parse_breakdown_result(&lines);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().subtasks.len(), 11);
    }

    #[test]
    fn test_prompt_contains_contract() {
        let prompt = BreakdownService::breakdown_prompt("Goal Title", "Goal Description");
        assert!(prompt.contains("GOAL TITLE: Goal Title"));
        assert!(prompt.contains("GOAL DESCRIPTION: Goal Description"));
        assert!(
            prompt.contains(
                "DO NOT modify, create, or delete any files — this is read-only analysis"
            )
        );
        assert!(prompt.contains(
            "{\"subtasks\":[{\"title\":\"...\",\"description\":\"...\",\"depends_on\":[0]}]}"
        ));
        assert!(prompt.contains("propose 2-10 subtasks"));
    }

    #[test]
    fn test_valid_block_followed_by_malformed_block_still_ok() {
        // A VALID block followed by a MALFORMED one: the last-deserializing-block
        // fallback must still succeed.
        let lines = vec![
            "```json".to_string(),
            "{\"subtasks\":[{\"title\":\"A\",\"description\":\"a\",\"depends_on\":[]},{\"title\":\"B\",\"description\":null,\"depends_on\":[0]}]}".to_string(),
            "```".to_string(),
            "Trailing prose".to_string(),
            "```json".to_string(),
            "{\"malformed\": invalid json}".to_string(),
            "```".to_string(),
        ];

        let result = BreakdownService::parse_breakdown_result(&lines).unwrap();
        assert_eq!(result.subtasks.len(), 2);
        assert_eq!(result.subtasks[0].title, "A");
        assert_eq!(result.subtasks[1].depends_on, vec![0]);
    }

    #[test]
    fn test_chunks_to_lines_handles_split_line_boundaries() {
        // Chunk boundaries deliberately do NOT align with line boundaries: the first
        // chunk ends mid-line, the second completes it and carries a full result line.
        let chunks = vec![
            "part-of-line".to_string(),
            "-completed\n{\"type\":\"result\",\"result\":\"```json\\n{\\\"subtasks\\\":[{\\\"title\\\":\\\"T1\\\",\\\"description\\\":null,\\\"depends_on\\\":[]},{\\\"title\\\":\\\"T2\\\",\\\"description\\\":null,\\\"depends_on\\\":[0]}]}\\n```\"}\n".to_string(),
        ];

        let lines = BreakdownService::chunks_to_lines(chunks);
        assert_eq!(lines[0], "part-of-line-completed");
        assert!(lines[1].starts_with("{\"type\":\"result\""));

        let result = BreakdownService::parse_breakdown_result(&lines).unwrap();
        assert_eq!(result.subtasks.len(), 2);
        assert_eq!(result.subtasks[0].title, "T1");
        assert_eq!(result.subtasks[1].title, "T2");
        assert_eq!(result.subtasks[1].depends_on, vec![0]);
    }

    #[test]
    fn test_parse_stream_json_stdout() {
        // Simulate Claude stream-JSON format: each line is a JSON object; the fenced block
        // exists only escaped inside a final {"type":"result","result":"..."} line
        let lines = vec![
            r#"{"type":"text","text":"Let me break this down..."}"#.to_string(),
            r#"{"type":"result","result":"Analyzing the goal.\n\n```json\n{\"subtasks\":[{\"title\":\"Task1\",\"description\":null,\"depends_on\":[]},{\"title\":\"Task2\",\"description\":null,\"depends_on\":[0]}]}\n```\n"}"#.to_string(),
        ];

        let result = BreakdownService::parse_breakdown_result(&lines).unwrap();
        assert_eq!(result.subtasks.len(), 2);
        assert_eq!(result.subtasks[0].title, "Task1");
        assert_eq!(result.subtasks[1].title, "Task2");
        assert_eq!(result.subtasks[1].depends_on, vec![0]);
    }

    #[test]
    fn test_parse_assistant_event_without_result_line() {
        // Real production seam (DV-4): the executor's protocol reader breaks on the final
        // result line without logging it, so the ONLY place the fenced block appears is
        // JSON-escaped inside {"type":"assistant"} message content. Shape captured from a
        // live claude-code 2.1.114 breakdown run on 2026-08-09.
        let lines = vec![
            r#"{"type":"system","subtype":"init","session_id":"s1"}"#.to_string(),
            r#"{"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"```json"}}}"#.to_string(),
            r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"Here is the breakdown.\n\n```json\n{\"subtasks\":[{\"title\":\"TaskA\",\"description\":null,\"depends_on\":[]},{\"title\":\"TaskB\",\"description\":null,\"depends_on\":[0]}]}\n```\n"}]},"session_id":"s1"}"#.to_string(),
            r#"{"type":"stream_event","event":{"type":"message_stop"},"session_id":"s1"}"#.to_string(),
        ];

        let result = BreakdownService::parse_breakdown_result(&lines).unwrap();
        assert_eq!(result.subtasks.len(), 2);
        assert_eq!(result.subtasks[0].title, "TaskA");
        assert_eq!(result.subtasks[1].depends_on, vec![0]);
    }
}
