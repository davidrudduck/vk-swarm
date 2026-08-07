---
id: "401"
phase: 4
title: "MCP tools: break_down_task, get_breakdown, accept_breakdown"
status: ready
depends_on: ["301"]
parallel: false
conflicts_with: []
files:
  - "crates/server/src/mcp/task_server.rs"
irreversible: false
scope_test: "crates/server"
allowed_change: edit
covers_criteria: []
covers_tests: ["TS5"]
---
## Failing test (write first)
Mirror the file's existing test conventions (read its #[cfg(test)] mod if present; if the file has NO tests today, add param-struct serde tests: BreakDownTaskRequest/AcceptBreakdownRequest deserialize from the documented JSON shapes, and the three tools appear in the tool router's list — assert via ToolRouter introspection if the rmcp API exposes it, else record the boundary in the ledger and rely on 701's live MCP check).


## Change
**File:** crates/server/src/mcp/task_server.rs
**Anchor:** the #[tool_router] impl block (~line 702) — append after list_nodes (~line 1386), following EXACTLY the create_task pattern (params struct with schemars derives at ~28-45; handler at ~792-840; self.send_json + ApiResponseEnvelope; TaskServer::success helper).
Add:
- `BreakDownTaskRequest { task_id: String }` → `#[tool(description = "Start an AI breakdown of a task into proposed subtasks; returns the draft proposal. The proposal must be reviewed and accepted before subtasks become real.")] async fn break_down_task` → POST self.url(&format!("/api/tasks/{}/breakdown", task_id)).
- `GetBreakdownRequest { task_id: String }` → GET .../breakdown → returns proposal + items JSON.
- `AcceptBreakdownRequest { proposal_id: String }` → POST /api/breakdown-proposals/{id}/accept → returns created tasks.
Error handling identical to create_task (bubble the envelope message).


## Allowed moves
Only appending the three param structs + three tool methods (and tests). No changes to existing tools, transport, or router plumbing.


## STOP triggers
The tool_router macro pattern differs from researched (e.g. tools must also be registered in a manual list); send_json signature mismatch; any need to touch mcp_task_server.rs (unlisted).


## Manual verification (record in decisions-ledger)



## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-task-breakdown 401` exits 0
