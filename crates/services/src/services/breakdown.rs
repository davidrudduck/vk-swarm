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
    /// - >10 subtasks allowed (lenient upper bound)
    pub fn parse_breakdown_result(
        stdout_lines: &[String],
    ) -> Result<BreakdownResult, BreakdownError> {
        // Stage 1: Substitute stream-JSON lines
        let mut text = String::new();
        for line in stdout_lines {
            if let Ok(obj) = serde_json::from_str::<serde_json::Value>(line)
                && obj.get("type").and_then(|v| v.as_str()) == Some("result")
                && let Some(result) = obj.get("result").and_then(|v| v.as_str())
            {
                text.push_str(result);
                text.push('\n');
                continue;
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
        let result = match parsed {
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
}
