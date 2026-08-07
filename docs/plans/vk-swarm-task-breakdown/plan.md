# vk-swarm-task-breakdown Plan

## Spec
docs/superpowers/specs/2026-08-07-vk-swarm-task-breakdown.md

## Approach
Build bottom-up along the proven layering: data substrate first (migration + db model module + ts-rs types), then the execution vehicle (new run reason + BreakdownService with the structured-output parser + the exit-monitor completion hook), then the REST API that all three triggers converge on, then the MCP proxy tools, then the frontend (api client/hook, review dialog, card action + badge + i18n), then the opt-in auto trigger (project column + create_task hook + settings toggle), and finally full gates + live deploy acceptance evidence.

Every structural decision is pre-made per the frozen spec and ADR-0016: proposals live in task_breakdown_proposals/_items and never touch TaskStatus, never enqueue outbox ops, and never sync; acceptance creates ordinary child tasks inside one transaction that mirrors Task::create (including enqueue_task_upsert_op) and writes task_dependencies edges. The breakdown run is a normal task attempt (normal worktree) with run_reason Breakdown; its result is the LAST fenced json block parsed from the durable ExecutionProcessLogs JSONL (ResultMessage precedent).

Traps pre-empted from prior ledgers: strict-TS test snippets (no unused imports, null-safe queries), stale anchors (every anchor re-verified against main at authoring time, 2026-08-07), new card actions must be added to BOTH branches of actions-dropdown.tsx, and typegen-touching tasks each own their shared/types.ts regeneration to avoid cross-task drift.


## Phases
- **Phase 1: data-substrate** — Proposal + dependency tables exist with a typed, tested db module and generated TS types
- **Phase 2: execution-vehicle** — A breakdown run can be spawned as a normal attempt and its structured result parsed durably into proposal items
- **Phase 3: api** — All breakdown lifecycle endpoints live under /api with review-gate invariants enforced
- **Phase 4: mcp** — External agents reach the same endpoints through MCP tools
- **Phase 5: frontend** — Operator can trigger, review, edit, accept/discard breakdowns from the board
- **Phase 6: auto-trigger** — Opt-in per-project auto breakdown on qualifying task creation
- **Phase 7: ship** — Full repo gates green + live deploy acceptance evidence recorded

## Tasks

| id | phase | title | deps | conflicts |
|---|---:|---|---|---|
| 101 | 1 | Migration: task_breakdown_proposals, task_breakdown_proposal_items, task_dependencies | dep: none | conflicts: none |
| 102 | 1 | db model module task_breakdown: structs, queries, accept transaction, tests | dep: 101 | conflicts: none |
| 103 | 1 | Register breakdown types in generate_types.rs and regenerate shared/types.ts | dep: 102 | conflicts: none |
| 201 | 2 | Add ExecutionProcessRunReason::Breakdown variant | dep: none | conflicts: none |
| 202 | 2 | BreakdownService: prompt template + BreakdownResult parser + persistence | dep: 102 201 | conflicts: none |
| 203 | 2 | Exit-monitor completion hook: parse breakdown runs into proposal items | dep: 202 | conflicts: none |
| 301 | 3 | REST API: breakdown lifecycle endpoints + dependencies query | dep: 103 203 | conflicts: none |
| 401 | 4 | MCP tools: break_down_task, get_breakdown, accept_breakdown | dep: 301 | conflicts: none |
| 501 | 5 | Frontend api client + hooks for breakdown | dep: 301 | conflicts: none |
| 502 | 5 | BreakdownReviewDialog (NiceModal) with edit/reorder/accept/discard/retry | dep: 501 | conflicts: none |
| 503 | 5 | Card action (both dropdown branches) + proposed badge + i18n keys (en/ja/ko/es) | dep: 502 | conflicts: none |
| 601 | 6 | Project auto_breakdown_enabled: migration + model + typegen | dep: 103 | conflicts: none |
| 602 | 6 | create_task auto-trigger hook (fire-and-forget, opt-in) | dep: 601 301 | conflicts: none |
| 603 | 6 | Project settings toggle for auto breakdown (+ i18n) | dep: 601 | conflicts: none |
| 701 | 7 | Full repo gates (exit 0) + live deploy acceptance evidence (SC1–SC7) | dep: 401 503 602 603 | conflicts: none |
