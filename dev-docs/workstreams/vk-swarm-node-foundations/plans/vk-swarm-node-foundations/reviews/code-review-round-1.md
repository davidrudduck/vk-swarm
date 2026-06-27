---
round: 1
date: 2026-06-27
effort: high
target: docs/phase1-analysis vs main
context: pre-graduation close gate (/wai:close); adversarial rounds 1 and 2 already completed
---

# Code-Review Round 1 — Pre-Graduation Gate

**Target:** branch `docs/phase1-analysis` vs `main`
**Effort:** high (3 parallel finder subagents)
**Models run:** general-purpose (×3), each covering a distinct review dimension

## Dimensions covered

| Dimension | Reviewer | Files |
|-----------|----------|-------|
| container.rs recovery logic | Agent A | `crates/services/src/services/container.rs` |
| DB queries, migration, tests | Agent B | `crates/db/src/models/execution_process/queries.rs`, migration, `task_visibility_discriminator.rs`, `task/queries.rs` |
| process_fence integration | Agent C | `crates/services/src/services/process_fence.rs`, `process_inspector/mock.rs`, `container.rs` |

## Findings

| ID | Severity | Confidence | Description | Citation | Actionable |
|----|----------|------------|-------------|----------|------------|
| CR-1 | medium | high | `NotOurProcess` arm leaves parent task stuck in `InProgress` forever. The arm sets `resume_state='abandoned'` and `continue`s; the blanket `mark_orphaned_as_failed` then sets `execution_processes.status='failed'` but never updates the parent task status (no call to `mark_process_failed_with_task_update`). All other terminal paths (no session_id, resume failure) explicitly call `mark_process_failed_with_task_update`, creating a visible inconsistency: a PID-reuse event leaves the task stuck in InProgress while every other failure path correctly transitions to InReview. | `container.rs:347-359`, `queries.rs:119-128` | YES |
| CR-2 | low | high | Comment "Fallback: create a minimal action; resume_execution will fail gracefully" is inaccurate. `build_resume_action` matches `CodingAgentInitialRequest` and always returns `Some`, so `resume_execution` does NOT fail — it proceeds with a `ClaudeCode:DEFAULT` profile, silently dropping any original variant (e.g. PLAN). The continuation prompt itself is correct (the minimal prompt is used regardless). Only the executor profile is degraded. Affects only the rare case where `executor_action` JSON is unparseable. | `container.rs:395` | YES |

## Non-actionable

| ID | Reason |
|----|--------|
| D-state re-fence loop | D-state processes are re-fenced (with no-op `resume_state='pending'` rewrite) on every restart. Acknowledged in the comment at `container.rs:368`. Not a bug — intentional retry. |
| `get_resume_state` double-Option flatten | `Option<Option<String>>.flatten()` correctly collapses both "row not found" and "row found, value NULL" to `Ok(None)`. Callers only care whether a non-NULL state string exists. |
| `mark_orphaned_as_failed` 'abandoned' sweep | `'abandoned' NOT IN ('pending', 'resumed')` = TRUE; abandoned rows are correctly included in the blanket sweep. |
| EXISTS-branch test | `remote_mirrored_task_with_local_attempt_is_visible_via_exists_branch` correctly sets `remote_last_synced_at` non-NULL, making the EXISTS clause the only path to visibility. |
| Partial index coverage | Index `WHERE status = 'running'` covers `mark_orphaned_as_failed`'s access pattern. |
| `$1`-syntax for SQLite | SQLx 0.8.6 SQLite driver handles `$NNN`-prefixed params correctly (strips `$`, parses as 1-based positional). |
| FenceOutcome match exhaustiveness | All 4 arms present (`AlreadyGone`, `Fenced`, `NotOurProcess`, `CouldNotKill`). |
| `next_action` clone | `stored.next_action().map(|next| Box::new(next.clone()))` is type-correct; `next_action()` returns `Option<&ExecutorAction>`. |
| `find_running_with_pids` scope | No server_instance_id filter, but safe because `cleanup_orphan_executions` runs at startup before any executions begin. |

## Verdict

Two actionable findings. Remediating before graduation.

Actionable: [CR-1, CR-2]
