---
id: "102"
phase: 1
title: "db model module task_breakdown: structs, queries, accept transaction, tests"
status: passed
depends_on: ["101"]
parallel: false
conflicts_with: []
files:
  - "crates/db/src/models/task_breakdown/mod.rs"
  - "crates/db/src/models/task_breakdown/queries.rs"
  - "crates/db/src/models/mod.rs"
  - "crates/db/.sqlx/query-0bb9ead0e0e3820b77fa20400f179a167f7de364afbe0ffb2241a731468b8fff.json"
  - "crates/db/.sqlx/query-108969fc6a6dd7be4ca43daf811827153f98cacf8d716cf67596eb8e86ac31d2.json"
  - "crates/db/.sqlx/query-136d82bd62df6963ac1cfd32ec6d1307375802b886cc490ab0e3437bbefe4b53.json"
  - "crates/db/.sqlx/query-3f4aca696664a6b33beb2eb00dde5fae10055c9fe2643a16709d895d0749b294.json"
  - "crates/db/.sqlx/query-3fa7af7dfc29b4c985804e2b5e6ca37a49c195aac01793a5e021b46564e617d4.json"
  - "crates/db/.sqlx/query-498c82eeb5730d9a5d4b6b6067fd214515717562b7ed01d66b9e750fa6bbc349.json"
  - "crates/db/.sqlx/query-5b42033dc4fa1e0481e08b9c059d5f3ded542bb7cd5fc9d439356588b7f33a1b.json"
  - "crates/db/.sqlx/query-6d409da081468120ef812a5a263d998dcb6889cf1c83bd0f73a3e8f90647905a.json"
  - "crates/db/.sqlx/query-ac7a7ce390a91df610e1aa9918378235cb322a6993ae5c4d87ac557172aee233.json"
  - "crates/db/.sqlx/query-b5d61df8b6f9110ab51b753dc3dbc2d0e5f3ee495c9af1a6e8edee5ee085cf35.json"
  - "crates/db/.sqlx/query-d288dfac333b6b78db647fd3ce59fc272a9fb24b07bbd22f9d147e701316671b.json"
  - "crates/db/.sqlx/query-d5f07b38d7bb0201a2e17491bfab6775e601702992c254e74dfd2a7c055f3e80.json"
  - "crates/db/.sqlx/query-d9756beb1a856cc0ca520913a7751112520ba6e8079c34b26bc6963a809113da.json"
  - "crates/db/.sqlx/query-e4eaaaac91b7cf15f1d23f34a0736c63092c334fd6f53c6d4858677d38b37071.json"
  - "crates/db/.sqlx/query-f902b5d6f2b9d31e07f2166b808e0a8addd5e79dabf5eb886215946ef66ef9f9.json"
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
5. test_replace_items_rejects_self_and_dangling_refs — replace_items with an item whose depends_on_indices contains its own index → Err and the previous items remain (rollback); same for an out-of-range index. Also assert the proposal's updated_at changed after a successful replace_items (mutations must refresh it; DEFAULT is insert-only).


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
- replace_items(pool, proposal_id, Vec<ProposalItemInput>) — delete+reinsert inside one transaction; only legal while status='draft' (return error otherwise). VALIDATE FIRST (CodeRabbit PR470): every depends_on_indices element must be in range 0..items.len() AND != its own item's index — reject self and dangling references with an error before any write. Touch the parent proposal's updated_at in the same transaction.
- update_status(pool, id, BreakdownStatus, error: Option<String>) — every UPDATE in this module (update_status, link_execution_process, replace_items' proposal touch, accept's status flip) must also SET updated_at = datetime('now','subsec'): the column DEFAULT fires only on INSERT (CodeRabbit PR470).
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
