---
id: "202"
phase: 2
title: "BreakdownService: prompt template + BreakdownResult parser + persistence"
status: passed
depends_on: ["102","201"]
parallel: false
conflicts_with: []
files:
  - "crates/services/src/services/breakdown.rs"
  - "crates/services/src/services/mod.rs"
siblings: ["crates/services/src/services/git.rs","crates/executors/src/executors/claude/protocol.rs"]
irreversible: false
scope_test: "crates/services"
allowed_change: mixed
covers_criteria: []
covers_tests: ["TS2"]
---
## Failing test (write first)
In crates/services/src/services/breakdown.rs `#[cfg(test)] mod tests` (unit tests, no DB needed for the parser):
1. test_parse_last_fenced_json_block — input: a Vec<String> of stdout lines containing prose, one malformed ```json block, then one valid block {"subtasks":[{"title":"A","description":"a","depends_on":[]},{"title":"B","description":null,"depends_on":[0]}]}; assert parse_breakdown_result returns Ok with 2 subtasks and B depends on index 0.
2. test_parse_missing_block_errs — lines with no fenced json → Err(BreakdownError::NoResult).
3. test_parse_rejects_bad_indices — depends_on:[5] with 2 subtasks → Err(BreakdownError::InvalidDependency); depends_on:[1] on subtask index 1 (self) → same.
4. test_parse_rejects_empty — {"subtasks":[]} → Err(BreakdownError::Empty); empty title → Err(BreakdownError::EmptyTitle); exactly one subtask → Err(BreakdownError::TooFew); 11 subtasks → Ok (upper bound deliberately lenient).
5. test_prompt_contains_contract — breakdown_prompt(&task_title, &task_description) contains the literal schema line and the read-only instruction.
6. test_parse_stream_json_stdout — fixture lines are Claude stream-JSON (each stdout line is a JSON object; the fenced block exists ONLY escaped inside the final {"type":"result","result":"...```json...```..."} line, per crates/executors/src/executors/claude/protocol.rs:113-147); assert the parser extracts the result text, fence-scans it, and returns the 2 subtasks (tournament R1 F3: line-based fence scanning alone never matches production Claude output).


## Change
**File:** crates/services/src/services/breakdown.rs (new) — stateless `#[derive(Clone)] pub struct BreakdownService;` per CLAUDE.md service conventions (read sibling git.rs for the shape). Contents:
- `#[derive(Debug, Deserialize, Serialize)] pub struct BreakdownResult { pub subtasks: Vec<BreakdownSubtask> }` and `BreakdownSubtask { pub title: String, pub description: Option<String>, #[serde(default)] pub depends_on: Vec<usize> }`.
- `#[derive(Debug, thiserror::Error)] pub enum BreakdownError { NoResult, Empty, TooFew, EmptyTitle, InvalidDependency, Db(#[from] sqlx::Error), Json(#[from] serde_json::Error) }` (thiserror per CLAUDE.md).
- `pub fn breakdown_prompt(title: &str, description: &str) -> String` — exact template:
```text
You are decomposing a development goal into independently executable subtasks.\nGOAL TITLE: {title}\nGOAL DESCRIPTION: {description}\n\nRules: propose 2-10 subtasks, each independently executable; use depends_on (array of zero-based indices into your own list) only for true prerequisites; DO NOT modify, create, or delete any files — this is read-only analysis.\n\nRespond with EXACTLY ONE fenced json code block as the FINAL element of your reply, matching:\n{\"subtasks\":[{\"title\":\"...\",\"description\":\"...\",\"depends_on\":[0]}]}
```
- `pub fn parse_breakdown_result(stdout_lines: &[String]) -> Result<BreakdownResult, BreakdownError>` — TWO-STAGE (tournament R1 F3): (stage A) for each line that parses as a JSON object with "type":"result", extract its result text (claude/protocol.rs ResultMessage precedent :113-147) and substitute it for that line; (stage B) over the resulting text, scan for fenced ```json blocks (open fence with language json, closing ```) and take the LAST block that deserializes into BreakdownResult. This handles both stream-JSON executors (Claude) and plain-markdown stdout (other executors). Validate: at least 2 subtasks (a single-subtask result is a non-breakdown — reject as BreakdownError::TooFew, new variant; CodeRabbit PR470 — the >10 upper bound is deliberately NOT enforced: an 11-item result is still usable and hard-failing it wastes a paid run; record in ledger), non-empty titles, every depends_on index in range and != self. Descriptions REMAIN Option<String> — a null/absent description is valid (tournament R1 rejected the mandatory-description finding: SC1 is a live-acceptance observable, not a parser contract, and the schema is deliberately nullable).
- `pub async fn persist_result(pool: &SqlitePool, proposal_id: Uuid, result: &BreakdownResult) -> Result<(), BreakdownError>` — map subtasks to ProposalItemInput (sort_order = index; depends_on_indices = depends_on.iter().map(|&i| i as i64).collect::<Vec<i64>>() (elementwise — a Vec cast does not compile; CodeRabbit PR470)) and call task_breakdown::replace_items, then update_status(.., Draft-stays-draft — items landing does NOT change status).
- `pub async fn fail_proposal(pool, proposal_id, error_text)` — update_status(Failed, Some(error_text)).
- `pub async fn extract_stdout_lines(pool: &SqlitePool, execution_process_id: Uuid) -> Result<Vec<String>, BreakdownError>` — load ExecutionProcessLogs::find_by_execution_id, parse_logs(), keep LogMsg::Stdout payloads split into lines (read execution_process_logs.rs:40-63 first).

**File:** crates/services/src/services/mod.rs — Anchor: the `pub mod` list; add `pub mod breakdown;` alphabetically.


## Allowed moves
Create breakdown.rs exactly as specified plus the single mod line. No edits to container.rs (that is task 203), no executor crate changes.


## STOP triggers
ExecutionProcessLogs API differs from the researched shape (find_by_execution_id/parse_logs absent); LogMsg variants don't expose stdout text lines; services crate cannot depend on a needed type without a Cargo.toml change (unlisted file — STOP).


## Manual verification (record in decisions-ledger)



## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-task-breakdown 202` exits 0
