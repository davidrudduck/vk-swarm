---
id: "303"
phase: 3
title: Reconstruct ExecutorAction + resume re-entry helper
status: passed
depends_on: []
parallel: false
conflicts_with: ["304"]
files:
  - crates/services/src/services/container.rs
irreversible: false
scope_test: "crates/services/src/services/container.rs"
allowed_change: edit
covers_criteria: [SC1]
---
## Failing test (write first)
Add a unit test (in `crates/services/src/services/container.rs` `#[cfg(test)]`, or a colocated test
module) that exercises ONLY the action-reconstruction logic (not the actual spawn):
```rust
// Given a stored execution_process whose executor_action carries a Claude profile,
// and a recovered session_id, build_resume_action(...) yields a CodingAgentFollowUpRequest
// with the SAME executor_profile_id and the recovered session_id.
#[test]
fn test_build_resume_action_preserves_profile_and_session() {
    let stored = sample_coding_agent_action_claude(); // helper: an ExecutorAction with a Claude initial req
    let action = build_resume_action(&stored, "sess-abc".to_string(), "continue".to_string())
        .expect("resumable");
    match action.typ {
        ExecutorActionType::CodingAgentFollowUpRequest(req) => {
            assert_eq!(req.session_id, "sess-abc");
            assert_eq!(req.executor_profile_id.executor, BaseCodingAgent::ClaudeCode);
        }
        _ => panic!("expected a follow-up resume action"),
    }
}
```

## Change
- **File:** `crates/services/src/services/container.rs`
- **Anchor:** add a helper near `cleanup_orphan_executions` (L239) / `start_execution_inner` decl
  (L445). The `ContainerService` trait already exposes `start_execution_inner(&self, task_attempt,
  execution_process, executor_action)` (L445-450) — the resume path re-enters through it.
- **Before:** no resume-reconstruction helper exists; recovery only marks failed.
- **After:** add a pure helper `build_resume_action(stored: &ExecutorAction, session_id: String,
  prompt: String) -> Option<ExecutorAction>` that:
  1. extracts the `executor_profile_id` from the stored coding-agent action (the stored
     `execution_processes.executor_action` JSON — its initial/follow-up request carries
     `executor_profile_id`; return `None` if the stored action is not a coding-agent action),
  2. constructs `CodingAgentFollowUpRequest { prompt, session_id, executor_profile_id }` (type at
     `crates/executors/src/actions/coding_agent_follow_up.rs:14-22`),
  3. wraps it in an `ExecutorAction` of type `ExecutorActionType::CodingAgentFollowUpRequest(..)`,
     preserving `next_action` from the stored action.
  Then add an async `resume_execution(&self, task_attempt, execution_process, session_id, prompt)`
  helper that builds the action via `build_resume_action` and calls
  `self.start_execution_inner(task_attempt, execution_process, &action)` into `container_ref`.
- **Resume-prompt (the one flagged judgment call — see ledger + task 301):** the `prompt` argument is
  decided by 301's capability audit. Default per the ledger: a minimal continuation prompt; 301 may
  instead choose to re-send `executor_sessions.prompt`. 303 takes `prompt` as a PARAMETER so 304
  supplies the value 301 settled — do NOT hard-code a policy here beyond the documented default in the
  test.

## Allowed moves
ONLY add the two helpers (`build_resume_action`, `resume_execution`) + their test. Do NOT modify
`cleanup_orphan_executions` itself (that is task 304, which CALLS these). Do NOT change
`start_execution_inner` or any executor type.

## STOP triggers
- The stored `ExecutorAction` / `ExecutorActionType` shape differs from the assumed
  `CodingAgentFollowUpRequest`/`executor_profile_id` structure (verify against
  `crates/executors/src/actions/`) → reconcile to the real types before writing.
- `ExecutorAction` has no public constructor for `CodingAgentFollowUpRequest` / `next_action` is not
  accessible → use the real construction path the codebase already uses for follow-ups
  (`grep -rn "CodingAgentFollowUpRequest {" crates/`) and mirror it.

## Done when
`WAI_TYPECHECK_CMD="cargo check -p services" WAI_TEST_CMD="cargo test -p services build_resume_action" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-node-foundations 303` exits 0
