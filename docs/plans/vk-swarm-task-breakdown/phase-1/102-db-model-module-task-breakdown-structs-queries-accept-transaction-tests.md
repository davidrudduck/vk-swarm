---
id: "102"
phase: 1
title: "db model module task_breakdown: structs, queries, accept transaction, tests"
status: ready
depends_on: ["101"]
parallel: false
conflicts_with: []
files:
  - "crates/db/src/models/task_breakdown/mod.rs"
  - "crates/db/src/models/task_breakdown/queries.rs"
  - "crates/db/src/models/mod.rs"
siblings: ["crates/db/src/models/task/mod.rs","crates/db/src/models/task/queries.rs"]
irreversible: false
scope_test: "crates/db"
allowed_change: mixed
covers_criteria: []
covers_tests: ["TS1"]
---
## Failing test (write first)
In crates/db/src/models/task_breakdown/mod.rs `#[cfg(test)] mod tests`, using db::test_utils::create_test_pool(). Write these tests FIRST (they fail to compile until the module exists, then fail until queries are implemented):

1. test_proposal_crud_and_one_draft_constraint — create a project+task (mirror helpers used in crates/db/src/models/task/mod.rs tests), create a draft proposal via TaskBreakdownProposal::create, assert find_by_task_id returns it; attempt a second create for the same task and assert it errors (unique index); mark the first 'discarded' via update_status and assert a new draft can then be created.
2. test_cascade_delete — create proposal + 2 items; delete the parent task row; assert proposal and items are gone.
3. test_accept_transaction_atomic — proposal with items A,B where B depends_on_item_ids=[A]; call accept_proposal; assert: two child tasks exist with parent_task_id = parent, one task_dependencies row (B_task -> A_task), proposal.status == 'accepted', and node_outbox contains task.upsert rows whose entity_id matches EACH of the two returned child task ids (filter by entity_id IN (child ids) — do NOT assert an absolute count: the parent-task setup via Task::create also enqueues a row; tournament R1 F6). Then: build a proposal whose item has depends_on_item_ids referencing a NON-EXISTENT item id, call accept_proposal, assert Err AND zero new tasks/edges/outbox rows for those children exist (rollback proof).
4. test_find_by_execution_process_id_exact — one task with a historical discarded proposal (execution_process_id P1) and a current draft (P2): find_by_execution_process_id(pool, P2) returns the draft, P1 returns the discarded one, a random uuid returns None (tournament R1: task 203 depends on this exact lookup).


## Change
**File:** crates/db/src/models/task_breakdown/mod.rs (new) — mirror the directory-module shape of crates/db/src/models/task/mod.rs. Read that sibling first; list its structural choices (FromRow derives, TS derives, #[ts(type="Date")] on DateTime fields, snake_case serde enums) and follow them. Define:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "breakdown_status", rename_all = "lowercase")]
pub enum BreakdownStatus { Draft, Accepted, Discarded, Failed }

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct TaskBreakdownProposal {
    pub id: Uuid,
    pub task_id: Uuid,
    pub status: BreakdownStatus,
    pub execution_process_id: Option<Uuid>,
    pub error: Option<String>,
    #[ts(type = "Date")] pub created_at: DateTime<Utc>,
    #[ts(type = "Date")] pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct TaskBreakdownProposalItem {
    pub id: Uuid,
    pub proposal_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub sort_order: i64,
    pub depends_on_item_ids: String, // JSON array of item Uuids
    #[ts(type = "Date")] pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct UpsertProposalItems { pub items: Vec<ProposalItemInput> }
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ProposalItemInput { pub title: String, pub description: Option<String>, pub sort_order: i64, pub depends_on_indices: Vec<i64> }

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct TaskDependency { pub task_id: Uuid, pub depends_on_task_id: Uuid, #[ts(type = "Date")] pub created_at: DateTime<Utc> }
```

Declare `mod queries;` and re-export its fns. Tests per Failing test.

**File:** crates/db/src/models/task_breakdown/queries.rs (new) — sqlx::query_as! style matching crates/db/src/models/task/queries.rs (read it; note the `as \"id: Uuid\"` column casts and that Task::create enqueues an outbox op at queries.rs:292 — the accept transaction below MUST mirror that enqueue INSIDE its transaction). Implement:
- create(pool, task_id) -> Result<TaskBreakdownProposal> (INSERT draft row)
- find_by_task_id(pool, task_id) -> Result<Option<TaskBreakdownProposal>> (latest by created_at)
- find_by_execution_process_id(pool, execution_process_id) -> Result<Option<TaskBreakdownProposal>> (exact match on the linked process — task 203's lookup; tournament R1)
- find_by_id(pool, id), find_items(pool, proposal_id) -> Vec<TaskBreakdownProposalItem> ordered by sort_order
- replace_items(pool, proposal_id, Vec<ProposalItemInput>) — delete+reinsert inside one transaction; only legal while status='draft' (return error otherwise)
- update_status(pool, id, BreakdownStatus, error: Option<String>)
- link_execution_process(pool, id, execution_process_id)
- accept_proposal(pool, proposal_id) -> Result<Vec<Task>> — ONE transaction: load draft proposal (error if not draft), load items ordered, first pass INSERT child tasks (id=Uuid::new_v4, project_id from parent task, parent_task_id=parent, status 'todo') using the same INSERT SQL shape as Task::create, PLUS an INSERT INTO node_outbox per child with the same columns / op_type ('task.upsert') / payload / idempotency-key derivation as enqueue_task_upsert_op (read queries.rs:333-367) — but executed AGAINST THE TRANSACTION HANDLE with errors PROPAGATED (abort accept). This is a deliberate, PRE-AUTHORIZED divergence from Task::create's documented best-effort post-insert enqueue (queries.rs:326-332): acceptance requires all-or-nothing (tournament R1 adjudication — do NOT refactor task/queries.rs to share helpers; record the divergence in the decisions ledger). Build an index->task_id map; second pass resolve each item's depends_on_item_ids (JSON of item ids) to task_dependencies INSERTs — an unresolvable reference aborts (rollback); finally UPDATE proposal status='accepted'. Commit. Return created tasks.
- find_dependencies(pool, task_id) -> Vec<TaskDependency>

**File:** crates/db/src/models/mod.rs — Anchor: the alphabetical `pub mod` list. Add `pub mod task_breakdown;` in order.


## Allowed moves
Create the two module files exactly as specified; the ONLY edit to an existing file is the single `pub mod task_breakdown;` line in models/mod.rs. No schema changes, no edits to task/queries.rs.


## STOP triggers
Task::create's INSERT/outbox shape in the sibling differs so much that mirroring inside a transaction is not mechanical (escalate — do not invent a divergent sync path); sqlx compile-time checks demand DATABASE_URL/prepare steps not already handled by the crate's build; the one-draft unique index rejects legitimate test flows.


## Manual verification (record in decisions-ledger)



## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-task-breakdown 102` exits 0
