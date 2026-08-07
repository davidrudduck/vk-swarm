---
id: "203"
phase: 2
title: "Exit-monitor completion hook: parse breakdown runs into proposal items"
status: ready
depends_on: ["202"]
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
N/A — covered by existing tests: crates/local-deployment suite must stay green; the parser/persistence behaviour is unit-tested in 202 and the end-to-end effect is proven live in 701 (SC7 failure-path evidence). If the exit-monitor region already has a test harness, add a case there; otherwise record the gap in the decisions ledger (do NOT build a new process-spawning harness in this task).


## Change
**File:** crates/local-deployment/src/container.rs
**Anchor:** the exit-monitor completion block, lines ~711-782 — specifically after `let success = matches!(...) && exit_code == Some(0);` (line ~730) and BEFORE the `if success || cleanup_done` commit/next-action block.
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
        let Some(proposal) = /* task_breakdown::find_by_execution_process_id(pool, ctx.execution_process.id) — add this query in db if absent per 102's module (STOP if 102 omitted it: it is find_by_task_id on ctx.task_attempt.task_id filtered to draft + matching execution_process_id) */ else { return; };
        if !success {
            let _ = BreakdownService::fail_proposal(pool, proposal.id, "executor run failed".into()).await;
            return;
        }
        match BreakdownService::extract_stdout_lines(pool, ctx.execution_process.id).await
            .and_then(|lines| services::services::breakdown::parse_breakdown_result(&lines).map(|r| (lines, r)))
        {
            Ok((_lines, result)) => {
                if let Err(e) = BreakdownService::persist_result(pool, proposal.id, &result).await {
                    tracing::error!(proposal_id = %proposal.id, error = ?e, "breakdown persist failed");
                    let _ = BreakdownService::fail_proposal(pool, proposal.id, e.to_string()).await;
                }
            }
            Err(e) => {
                tracing::warn!(proposal_id = %proposal.id, error = ?e, "breakdown output unusable");
                let _ = BreakdownService::fail_proposal(pool, proposal.id, e.to_string()).await;
            }
        }
    }
```
Breakdown runs must NOT enter the commit/next-action path: extend the `if success || cleanup_done` condition to `if (success || cleanup_done) && !matches!(ctx.execution_process.run_reason, ExecutionProcessRunReason::Breakdown)`.


## Allowed moves
Only the insertion at the stated anchor, the one condition extension, and the private helper fn. Match the surrounding tracing/error-handling idiom. No other behaviour change in the exit monitor.


## STOP triggers
The anchor block has moved/been refactored (verify the `success` binding and `if success || cleanup_done` exist as researched); ExecutionContext lacks the fields used; the proposal-by-execution-process lookup does not exist in 102's module and cannot be expressed via find_by_task_id (escalate to amend 102 rather than inventing SQL here — unlisted file).


## Manual verification (record in decisions-ledger)



## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-task-breakdown 203` exits 0
