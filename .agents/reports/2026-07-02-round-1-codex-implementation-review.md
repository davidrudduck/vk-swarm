# Codex Implementation Review — hive-redesign

### Finding 1: [BLOCKING] legacy-task-status-bypasses-lease-fencing
- Location: crates/remote/src/nodes/ws/session.rs:677
- Evidence: `assignment_repo.find_by_id(status.assignment_id).await` loads by assignment id only; `crates/remote/src/db/task_assignments.rs:221-236` shows `find_by_id` has only `WHERE id = $1`, while `session.rs:694-702` writes `update_status_from_node` after only `node_may_author`. The task spec requires active lease context: `docs/plans/vk-swarm-hive-redesign/phase-3/304-gate-legacy-task-status-path.md:102-106` says the node-authored branch must be gated on the assignment being active. ADR-0009 requires stale writers to be bounced: `dev-docs/adr/0009-lease-checkout-fencing.md:37-42`.
- Problem: `handle_task_status` gates status authorship but does not verify that the reporting node still owns an active, unexpired lease. A partitioned node can retain an old `assignment_id`, have its lease reclaimed, then send `TaskStatusMessage { status: Completed }`; if the current shared task is `InProgress`, `node_may_author(InProgress, InReview)` passes and the stale node writes `shared_tasks.status`.
- Impact if shipped: SC3's at-most-once commit effect is not enforced on the legacy inbound path. The fixed `OpBatch` fence can reject stale tokens, but `TaskStatus` can still clobber status without a fencing token or active lease check.
- Remediation: Replace the legacy `find_by_id` path with an active-lease lookup scoped to the sender before updating shared status, e.g. query `node_task_assignments` with `id = $1 AND node_id = $2 AND completed_at IS NULL AND lease_expires_at IS NOT NULL AND lease_expires_at >= now()`, then use the returned `task_id`. If no row is returned, log and skip the shared-task status write. Add a regression test where node A's expired assignment is reclaimed by node B, then node A sends `TaskStatusMessage::Completed`; assert the shared task remains unchanged.

VERDICT: REVISE
TOTAL FINDINGS: 1 (1 [BLOCKING], 0 [SHOULD-FIX], 0 [INFO])
