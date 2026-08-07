---
doc_type: spec
status: active
workstream: vk-swarm-task-breakdown
change_kind: behaviour
verify_cmd: "sqlite3 ${VK_DATABASE_PATH:-$HOME/.local/share/vibe-kanban/db.sqlite} 'select status from task_breakdown_proposals' | grep -q accepted"
---

# vk-swarm-task-breakdown — AI task breakdown harness (P3 / SC3)

## Intent
Phase 3 of the vk-swarm-refactor program (docs/superpowers/specs/2026-06-25-vk-swarm-refactor.md). Owns umbrella success criterion SC3: a high-level goal can be turned by an AI harness into correctly-scoped, independently-executable subtasks.

Give the operator a kanban-driven way to turn a high-level goal card into a reviewed set of correctly-scoped, independently-executable child tasks — produced by an AI harness running node-local through the existing executor abstraction (the same enum_dispatch CLI-agent layer that runs task attempts today).

Why now: P1/P2 delivered a durable, offline-first, crash-resumable node + hive substrate. The next bottleneck to actually managing development work cycles in vk-swarm is that decomposing work is still manual and lives outside the board (chat sessions, WAI docs). P3 closes that gap and is — together with P4 (event bus) — the gate for P5 (conflict/priority/dependency automation), P6 (AI management agent), and P7 (MCP/ACP connectivity).

Decisions settled at intent time (interview 2026-08-07): placement is node-local and executor-driven; triggers are (1) a Break down card action in the node UI, (2) an MCP tool on the node's existing MCP server, (3) an automatic trigger on task creation behind a per-project opt-in; output is native child tasks plus persisted dependency edges; every trigger is gated by review-then-accept — nothing becomes ready work without operator acceptance.


## User stories
- **US1:** As the operator, I can invoke Break down on a goal card, review and edit the proposed subtasks, and accept them so they become real, independently-executable child tasks on the board.
- **US2:** As an external agent (MCP client), I can trigger and accept a breakdown programmatically with exactly the same semantics and review gate as the UI path.
- **US3:** As the operator of a project with auto-breakdown enabled, newly created qualifying tasks get a proposal set queued for my review without my asking, and never land as tasks directly.
- **US4:** As a downstream consumer (P5/P6), I can query persisted dependency edges between accepted tasks via the node API.

## Success criteria
SC1: On a running node, invoking Break down on a task whose description states a multi-part goal yields a proposal set of at least 2 items, each with title, description, and any dependency references, visibly marked as proposed and not actionable.
→ US1
SC2: Proposed items can be edited, deleted, and reordered; no attempt can be started from a proposed item; nothing reaches the hive (no outbox op rows for proposals) before acceptance.
→ US1
SC3: Accepting the (possibly edited) set creates real child tasks under the parent via the existing hierarchy, each immediately able to host a task attempt; dependency edges are persisted in task_dependencies and returned by GET /api/tasks/{id}/dependencies.
→ US1
SC4: Calling the break_down_task MCP tool against the same task produces the same proposal artifact observable via the node API and UI, subject to the same review gate; accept_breakdown lands the same child tasks as the UI accept.
→ US2
SC5: With auto_breakdown_enabled on, creating a qualifying task yields a draft proposal (never directly-landed child tasks); with it off (default), task creation behaviour is byte-for-byte unchanged.
→ US3
SC6: SC1 through SC3 succeed with the hive unreachable; after reconnect, accepted child tasks reconcile to the hive with zero silent loss via the existing outbox path.
→ US1
SC7: When the executor run fails or emits unparseable output, the proposal is marked failed with a visible localized error and a retry action on the card; zero partial proposal items or child tasks exist.
→ US1

## Users
Primary — the operator driving multiple coding agents across nodes (today /wai + /dr with Claude Code): gains in-board decomposition instead of out-of-band chat/docs planning.

External agents (MCP clients, later the P7 MCP/ACP fabric): gain a programmatic breakdown entry point with identical semantics to the UI path.

Downstream phases as consumers: P5 needs the dependency edges this produces; P6 needs correctly-scoped ready tasks to select from; P4's bus will carry this feature's lifecycle events (proposal created / accepted) once both exist.


## Constraints
Reuse the proven core: the executor abstraction (enum_dispatch over CLI agents, crates/executors/src/executors/mod.rs:88), local-SQLite-as-node-of-record, existing task parent/child hierarchy (tasks.parent_task_id, task->task since migration 20251215000000), ApiResponse<T> route patterns, ts-rs typegen with manual registration in crates/server/src/bin/generate_types.rs, i18n for all new UI strings in all four locales (en, ja, ko, es).

Offline-first is non-negotiable (umbrella principle 1): no hive round-trip anywhere on the breakdown path.

Node-scoped: a breakdown operates on one project on one node. Cross-node/global goals are a hive concern deferred to later phases.

Structured output contract: the executor must return a machine-parseable breakdown (BreakdownResult JSON, defined in this design); free-text parsing heuristics are not acceptable as the contract.

TaskStatus is untouched on node and hive — see ADR-0016; the proposal lifecycle lives in its own tables.

New card actions must be added to BOTH branches of frontend/src/components/ui/actions-dropdown.tsx (desktop DropdownMenu tree and the separate mobile bottom-sheet implementation).

Reference systems read-only: paperclip (/data/Code/reference/agents/paperclip) for governance/decomposition prior art; upstream vibe-kanban for hierarchy patterns.

GitHub targeting: PRs only against davidrudduck/vk-swarm.


## Out of scope
Hive-side breakdown or any hive-initiated assignment of breakdown work to nodes.

Conflict/priority/dependency computation and visualisation (P5) — this workstream only persists dependency edges; it does not schedule, rank, or graph them. Hive-side sync/aggregation of task_dependencies is likewise deferred.

Autonomous execution of accepted subtasks (P6) — acceptance creates ready tasks; no agent auto-starts them.

Event-bus emission (P4) — proposal/acceptance lifecycle events are designed so P4 can carry them later, but no bus integration ships here.

Recursive breakdown (auto-decomposing generated subtasks) — single level only.

WednesdayAI integration (P8).


## Approach
Ride the existing task-attempt execution machinery instead of inventing a parallel runner. A breakdown run is a real executor invocation: the node spawns the configured coding agent (via the enum_dispatch executor layer) with a breakdown prompt against the parent task, using the already-supported skip_worktree_creation path so no worktree is created, captures the agent's final structured JSON from the durable execution log (the same pattern as Claude's ResultMessage extraction in crates/executors/src/executors/claude/protocol.rs:124-137), and materialises the result as a proposal set in new node-local tables (ADR-0016). Acceptance is a single transaction that converts proposal items into ordinary child tasks (existing Task::create + parent_task_id hierarchy, syncing to the hive through the existing outbox) and writes dependency edges into a new task_dependencies table. All three triggers (card action, MCP tool, auto-on-create) converge on one REST endpoint so semantics are identical; the MCP tool follows the established task_server.rs proxy pattern and adds no DB access. UI is a review panel wired through the existing NiceModal/defineModal dialog convention with react-query mutations that invalidate the tasks and proposals keys.


## Design
Data model (crates/db, additive migrations; ADR-0016):
- task_breakdown_proposals: id UUID pk, task_id UUID fk->tasks ON DELETE CASCADE, status TEXT CHECK (draft|accepted|discarded|failed), execution_process_id UUID nullable fk, error TEXT nullable, created_at/updated_at. One active draft per task enforced by partial unique index.
- task_breakdown_proposal_items: id UUID pk, proposal_id fk ON DELETE CASCADE, title TEXT, description TEXT, sort_order INTEGER, depends_on_item_ids JSON (intra-set references by item id).
- task_dependencies: task_id fk, depends_on_task_id fk, created_at, pk(task_id, depends_on_task_id), both ON DELETE CASCADE; written only at acceptance; queryable via GET endpoints for P5.
Directory-module pattern: crates/db/src/models/task_breakdown/ (mod.rs, queries.rs) mirroring existing models; all public structs derive TS and are registered in generate_types.rs.

Execution vehicle (crates/services + crates/executors):
- New ExecutionProcessRunReason::Breakdown variant (additive TEXT value in execution_processes.run_reason).
- BreakdownService (stateless, Clone) composes the prompt from the parent task title/description plus a fixed instruction block demanding a final fenced JSON object matching BreakdownResult { subtasks: [{ title, description, depends_on: [index] }] }.
- The run uses a dedicated task attempt on the parent task created through the normal start_attempt path (ContainerService::start_attempt, crates/services/src/services/container.rs:1193) with a worktree created as usual; the breakdown prompt is read-only analysis, so the completion path's try_commit_changes finds a clean tree and commits nothing. (Note: skip_worktree_creation exists but means reuse-the-parent-attempt's-container, which a fresh goal task does not have.)
- On process completion, BreakdownService parses the durable log (ExecutionProcessLogs::find_by_execution_id + parse_logs, crates/db/src/models/execution_process_logs.rs) extracting the last valid BreakdownResult JSON block; success writes proposal items, failure (missing/malformed JSON, non-zero exit) marks the proposal failed with a stored error string — no partial items land.

API (crates/server/src/routes/tasks/, ApiResponse<T> pattern):
- POST /api/tasks/{task_id}/breakdown -> creates proposal row (draft) + starts the breakdown run; 409 if a draft proposal already exists.
- GET /api/tasks/{task_id}/breakdown -> current proposal + items.
- PUT /api/breakdown-proposals/{id}/items -> replace/edit the item set (title/description/deps/order edits, deletions).
- POST /api/breakdown-proposals/{id}/accept -> transaction: create child tasks via Task::create (parent_task_id = parent; normal outbox sync fires here and only here), map intra-set depends_on indices to task_dependencies rows, mark proposal accepted. Returns created tasks.
- POST /api/breakdown-proposals/{id}/discard; POST /api/breakdown-proposals/{id}/retry (failed -> new run).
- GET /api/tasks/{task_id}/dependencies -> edges for P5/consumers.

MCP (crates/server/src/mcp/task_server.rs): new tools break_down_task (POST proxy), get_breakdown (GET proxy), accept_breakdown (accept proxy) following the existing #[tool] + Parameters + send_json envelope pattern; no direct DB handle.

Auto trigger: projects gain auto_breakdown_enabled BOOLEAN NOT NULL DEFAULT 0 (additive migration). In create_task handler (after insert, before response), if enabled and the task has a non-empty description and no parent_task_id, fire the same BreakdownService entry fire-and-forget; failures only log. Default off; UI toggle in project settings.

Frontend (frontend/src):
- Break down action added to both actions-dropdown.tsx branches; visible when task has no draft proposal.
- BreakdownReviewDialog via NiceModal/defineModal (frontend/src/lib/modals.ts) listing proposed items with inline edit/delete/reorder and dependency chips; Accept and Discard actions; running/failed states with retry.
- TaskCard shows a proposed badge when a draft proposal exists (data via a useBreakdownProposal(taskId) react-query hook; new breakdownApi namespace in frontend/src/lib/api/).
- Mutations follow useTaskMutations conventions and invalidate ['tasks', projectId] and ['breakdown', taskId].
- All strings via useTranslation('tasks') with keys added to en/ja/ko/es tasks.json.

Sync/offline: proposals and dependencies are node-local tables with no outbox ops — nothing crosses the wire until acceptance creates real tasks (which reuse the proven task.upsert outbox path). Every endpoint works with the hive unreachable.


## Decisions
D1 (irreversible — ADR dev-docs/adr/0016-breakdown-proposals-separate-entity.md): proposals are a separate node-local entity (task_breakdown_proposals/_items) plus a first-class task_dependencies edge table keyed on real task ids; TaskStatus is never extended. P5/P6 build on this contract.

D2 (reversible): the breakdown run rides a dedicated task attempt through the normal start_attempt/worktree path with a new additive ExecutionProcessRunReason::Breakdown value, reusing spawn/logging/durability machinery instead of a parallel runner or a nullable task_attempt_id migration; the prompt is read-only so no commits are produced.

D3 (reversible): the executor output contract is a final fenced BreakdownResult JSON object parsed from the durable execution log (ResultMessage precedent); malformed output fails the proposal atomically — no partial items.

D4 (reversible): all three triggers converge on the single POST /api/tasks/{id}/breakdown endpoint; the MCP tool is a pure HTTP proxy per the established task_server.rs pattern.

D5 (reversible): auto-breakdown is a per-project boolean (default off) with a minimal heuristic (non-empty description, top-level task); richer heuristics deferred until usage data exists.

D6 (reversible): dependency edges sync nowhere in this workstream; hive awareness of edges is deferred to a later phase together with P5.


## Test strategy
TS1: DB layer: sqlx tests via db::test_utils::create_test_pool() covering proposal CRUD, the one-active-draft constraint, cascade deletes, and the accept transaction (child tasks + edges + status flip atomically; rollback on any failure).
TS2: Parser: unit tests for BreakdownResult extraction from ExecutionProcessLogs JSONL fixtures — valid block, malformed JSON, missing block, multiple blocks (last wins), non-zero exit — asserting atomic failure semantics.
TS3: API: route tests for breakdown endpoints covering 409 on duplicate draft, review-gate invariants (no attempt creation from proposals, no outbox ops pre-accept), accept output, discard, retry.
TS4: Frontend: vitest for breakdownApi, useBreakdownProposal hook, and BreakdownReviewDialog (edit/reorder/accept/discard/failed+retry states); i18n keys present in all four locales.
TS5: MCP: tests asserting the three tools proxy the REST endpoints faithfully (params mapping, error envelope pass-through) per the existing task_server test conventions.
TS6: Live acceptance on a deployed node per the SC list: card-action run end-to-end with hive disconnected (SC6), MCP parity (SC4), auto-trigger opt-in (SC5), failure injection via a bad executor profile (SC7); evidence recorded in the decisions-ledger and verify_cmd green post-deploy.

