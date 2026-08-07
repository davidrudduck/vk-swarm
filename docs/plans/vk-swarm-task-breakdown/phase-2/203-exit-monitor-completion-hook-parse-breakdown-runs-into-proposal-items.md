---
id: "203"
phase: 2
title: "Exit-monitor completion hook: parse breakdown runs into proposal items"
status: ready
depends_on: ["202","204"]
parallel: false
conflicts_with: []
files:
  - "crates/local-deployment/src/container.rs"
irreversible: false
scope_test: "crates/local-deployment"
allowed_change: edit
covers_criteria: []
covers_tests: []
---
## Failing test (write first)
N/A — covered by existing tests: crates/local-deployment suite must stay green; the parser/persistence behaviour is unit-tested in 202 and the end-to-end effect is proven live in 701 (SC7 failure-path evidence). If the exit-monitor region already has a test harness, add a case there; otherwise record the gap in the decisions ledger AND as a tracked follow-up item in dev-docs/workstreams/vk-swarm-task-breakdown/README.md with the re-enable condition (per CLAUDE.md a ledger note alone is not a tracked follow-up); do NOT build a new process-spawning harness in this task.


## Change
**File:** crates/local-deployment/src/container.rs
**Anchor:** the exit-monitor, AFTER the durable-log flush — locate the block that finishes the log batcher and completes normalization (`log_batcher.finish` ~:797-799 and the push_finished/normalization await ~:806-810) and insert the hook AFTER it, NOT at the earlier `success` binding (~:730): parsing before the flush reads incomplete ExecutionProcessLogs and falsely fails good runs (tournament R1 F-codex4). The `success` value computed at ~:730 (`ExecutionProcessStatus::Completed && exit_code == Some(0)`) must be captured/recomputed for use here.
Insert:
```rust
                if matches!(
                    ctx.execution_process.run_reason,
                    ExecutionProcessRunReason::Breakdown
                ) {
                    Self::handle_breakdown_completion(&db.pool, &ctx, success).await;
                }
```
Add a private async fn on the impl (near the other completion helpers):
```rust
    async fn handle_breakdown_completion(pool: &SqlitePool, ctx: &ExecutionContext, success: bool) {
        use services::services::breakdown::BreakdownService;
        let proposal = match db::models::task_breakdown::find_by_execution_process_id(pool, ctx.execution_process.id).await {
            Ok(Some(p)) => p,
            Ok(None) => return,
            Err(e) => { tracing::error!(execution_process_id = %ctx.execution_process.id, error = ?e, "breakdown proposal lookup failed"); return; }
        };
        if !success {
            if let Err(fe) = BreakdownService::fail_proposal(pool, proposal.id, "executor run failed".into()).await {
                tracing::error!(proposal_id = %proposal.id, error = ?fe, "failed to mark breakdown proposal failed");
            }
            return;
        }
        match BreakdownService::extract_stdout_lines(pool, ctx.execution_process.id).await
            .and_then(|lines| services::services::breakdown::parse_breakdown_result(&lines).map(|r| (lines, r)))
        {
            Ok((_lines, result)) => {
                if let Err(e) = BreakdownService::persist_result(pool, proposal.id, &result).await {
                    tracing::error!(proposal_id = %proposal.id, error = ?e, "breakdown persist failed");
                    if let Err(fe) = BreakdownService::fail_proposal(pool, proposal.id, e.to_string()).await {
                        tracing::error!(proposal_id = %proposal.id, error = ?fe, "failed to mark breakdown proposal failed");
                    }
                }
            }
            Err(e) => {
                tracing::warn!(proposal_id = %proposal.id, error = ?e, "breakdown output unusable");
                if let Err(fe) = BreakdownService::fail_proposal(pool, proposal.id, e.to_string()).await {
                    tracing::error!(proposal_id = %proposal.id, error = ?fe, "failed to mark breakdown proposal failed");
                }
            }
        }
    }
```
Breakdown runs must NOT enter the commit/next-action path: extend the `if success || cleanup_done` condition (~:740) to `if (success || cleanup_done) && !matches!(ctx.execution_process.run_reason, ExecutionProcessRunReason::Breakdown)`. (The finalize_task exclusion is owned by 204's should_finalize guard — do not duplicate it here.)


## Allowed moves
Only the post-flush insertion, the one condition extension, and the private helper fn. Match the surrounding tracing/error-handling idiom. No other behaviour change in the exit monitor.


## STOP triggers
The log-batcher finish / normalization-completion block cannot be located near ~:797-810 (refactored — re-anchor by searching for log_batcher.finish, else escalate); the `success` binding or `if success || cleanup_done` condition is absent as researched; ExecutionContext lacks the fields used; 102's find_by_execution_process_id is missing (102 must be amended — do not invent SQL here).


## Manual verification (record in decisions-ledger)



## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-task-breakdown 203` exits 0
