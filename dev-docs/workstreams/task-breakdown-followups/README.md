---
workstream: task-breakdown-followups
doc_type: readme
status: draft
title: "Task-breakdown follow-ups deferred from PR #475 review"
depends_on: []
adrs: []
staging_pointers:
  - docs/plans/vk-swarm-task-breakdown/decisions-ledger.md
---

# task-breakdown-followups

**Origin:** CodeRabbit review of PR #475 (2026-08-10). These findings are real but too large for a
review pass — each needs its own design decision. Declined there, tracked here, with the full
evidence recorded under `## Post-review known issues` in the workstream's decisions-ledger.

## What this workstream owns

1. **Proposal status on the task-list response.** `TaskCard` calls `useBreakdownProposal` per card,
   so an N-task board issues N `GET /api/tasks/{id}/breakdown` requests. Carry the proposal status
   on the task-list payload (or add a project-scoped batch query) and render the badge from that.
   Touches the Rust model, `generate-types`, and the frontend data flow.
   Anchor: `frontend/src/components/tasks/TaskCard.tsx:99`.

2. **Typed `BreakdownDbError`.** The breakdown queries signal domain failures as
   `sqlx::Error::Protocol(String)`, and `map_proposal_error` maps every one of them to
   `ApiError::Conflict` — so an invalid dependency index returns 409 where it should return 400,
   and no caller can tell failure kinds apart without parsing message text. Introduce a typed error
   with `#[from]` conversions and per-variant route mappings.
   Anchor: `crates/db/src/models/task_breakdown/queries.rs`.

3. **Bound concurrent breakdown runs.** Each run creates a task attempt and a git worktree, and
   with `auto_breakdown_enabled` every eligible task creation spawns one detached executor. Nothing
   bounds the concurrency. Add a semaphore or small worker pool plus an in-flight metric.
   Anchor: `crates/server/src/routes/breakdown.rs:86-170`.

4. **Directory-module split of the breakdown route domain.** 674 lines, seven handlers, two shared
   stage functions, a response type, and a test module in one file — the repo convention is
   `mod.rs` + `types.rs` + handler submodules.
   Anchor: `crates/server/src/routes/breakdown.rs`.

5. **De-duplicate prompt preparation.** The image-canonicalisation and variable-expansion block in
   `start_breakdown_attempt` repeats `start_attempt` almost exactly; only the log messages differ.
   Extract a shared helper so future changes to variable expansion apply to both paths.
   Anchor: `crates/services/src/services/container.rs:1406-1455` vs `1235-1277`.

6. **Close the remaining routes to a stuck `Draft` proposal.** Code-review round 1 (finding 7)
   fixed the swallowed panic in the detached auto-breakdown spawn — its `JoinHandle` is now
   supervised and a panic marks the proposal `Failed`. Two routes to the same state remain, both
   needing recovery design rather than a local fix:
   - a process restart between the committed `create_draft_proposal` and `link_execution_process`
     leaves a draft with no execution process (`crates/server/src/routes/breakdown.rs:167`);
   - the silent `Err` arm of `if let Ok(ctx) = ExecutionProcess::load_context(...)`
     (`crates/local-deployment/src/container.rs:889`) drops the completion handler entirely.

   A stuck draft cannot be re-triggered (409, one draft per task) or retried (retry requires
   `Failed`). It is recoverable by hand — the badge renders and Discard stays enabled — so this is
   a robustness item, not a data-loss one. Likely shape: a startup sweep that fails drafts whose
   execution process is absent or terminal.

## Status

Not started. Filed 2026-08-10; item 6 added from code-review round 1 (2026-08-10).
