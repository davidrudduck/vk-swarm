---
id: "204"
phase: 2
title: "Services-layer start_breakdown_attempt + finalize exclusion for Breakdown runs"
status: ready
depends_on: ["201"]
parallel: false
conflicts_with: []
files:
  - "crates/services/src/services/container.rs"
irreversible: false
scope_test: "crates/services"
allowed_change: edit
covers_criteria: []
covers_tests: []
---
## Failing test (write first)
In the existing crates/services container test module (or a new #[cfg(test)] block colocated in container.rs if none covers this trait): a unit test asserting should_finalize returns false for an ExecutionContext whose run_reason is Breakdown (build the ctx the way existing should_finalize tests do; if NO test constructs an ExecutionContext today, record the harness gap in the decisions ledger and rely on the compile-time exhaustiveness of the added guard + 701's live SC evidence).


## Change
**File:** crates/services/src/services/container.rs
(Tournament R1 CRITICAL: start_attempt at :1193 cannot inject a prompt — it derives it from task.to_prompt() at :1228 — and hard-codes ExecutionProcessRunReason::CodingAgent at :1305/:1348; the public trait method start_execution(attempt, action, run_reason) at :1356 is the injection seam.)

1. Add a trait method with default implementation, adjacent to start_attempt:
```rust
    async fn start_breakdown_attempt(
        &self,
        task_attempt: &TaskAttempt,
        executor_profile_id: ExecutorProfileId,
        prompt: String,
    ) -> Result<ExecutionProcess, ContainerError> { ... }
```
Mirror start_attempt's body EXACTLY for: container/worktree ensure, image-path canonicalisation, and task-variable expansion applied to the provided `prompt` (read :1193-1260 and reuse the same calls) — but build a bare `ExecutorAction::new(ExecutorActionType::CodingAgentInitialRequest(CodingAgentInitialRequest { prompt, executor_profile_id }), None)` (NO setup-script chaining, NO cleanup next_action — a breakdown is read-only analysis) and call `self.start_execution(task_attempt, &action, &ExecutionProcessRunReason::Breakdown)`. If CodingAgentInitialRequest carries additional required fields, copy their derivation from the start_attempt body verbatim.

2. **Anchor:** should_finalize (:236-241, the DevServer early-return). Extend the first matches! to also return false for Breakdown:
```rust
        if matches!(
            ctx.execution_process.run_reason,
            ExecutionProcessRunReason::DevServer | ExecutionProcessRunReason::Breakdown
        ) {
            return false;
        }
```
(Without this, a successful breakdown with no next_action reaches finalize_task at :268, which flips the parent task to InReview and pushes a hive shared-task update — violating the review gate and offline-first; tournament R1 F2/F7.)


## Allowed moves
Only the new trait method (default impl) and the one-line should_finalize guard extension. No changes to start_attempt, start_execution, or finalize_task bodies.


## STOP triggers
start_attempt's worktree-ensure/variable-expansion calls are not reusable from a sibling default method (private helpers on a different impl — escalate); CodingAgentInitialRequest requires fields whose derivation does not appear in start_attempt's body; start_execution's signature differs from researched (:1356).


## Manual verification (record in decisions-ledger)



## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-task-breakdown 204` exits 0
