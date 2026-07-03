### Finding 1: [BLOCKING] Legacy `handle_task_status` bypasses active lease / node-ownership invariants (SC3 violation)
- Location: `crates/remote/src/nodes/ws/session.rs:677`
- Evidence: `if let Ok(Some(assignment)) = assignment_repo.find_by_id(status.assignment_id).await { ... }`
- Problem: The legacy task status path looks up the assignment purely by ID and applies the status update without verifying that the assignment is currently held by the reporting node (`assignment.node_id == node_id`) or that it is still active (`assignment.completed_at.is_none()`). This allows any node to send a `TaskStatusMessage` and update the execution status and shared status of ANY assignment. 
- Impact if shipped: A partitioned node whose lease has expired and been reassigned can still clobber execution and shared task statuses. This reintroduces the exact status-clobbering defect that Phase 2 fencing (ADR-0009) was designed to eliminate, breaking SC3's at-most-once commit effect for the legacy channel.
- Remediation: Move the assignment lookup block *above* the `service.update_assignment_local_ids` and `update_assignment_status` calls (so execution status is also protected), and enforce node ownership and active status before applying any updates:
```rust
    let assignment_repo = TaskAssignmentRepository::new(pool);
    let assignment = match assignment_repo.find_by_id(status.assignment_id).await {
        Ok(Some(a)) => a,
        Ok(None) | Err(_) => {
            tracing::warn!(assignment_id = %status.assignment_id, "assignment not found");
            return Ok(());
        }
    };

    if assignment.node_id != node_id || assignment.completed_at.is_some() {
        tracing::warn!(
            node_id = %node_id,
            assignment_id = %status.assignment_id,
            "rejected status update: assignment not held by this node or already completed"
        );
        return Ok(());
    }

    let service = NodeServiceImpl::new(pool.clone());
    // ... proceed with the existing handler updates using the validated `assignment`
```

### Finding 2: [BLOCKING] Op-log fencing guard incorrectly rejects updates to node-owned tasks
- Location: `crates/remote/src/nodes/ws/session.rs:1964-2017`
- Evidence: `if let Some(shared_id) = shared_id { let assignment = sqlx::query_as( ... SELECT ... FROM node_task_assignments ... )`
- Problem: The code assumes that if `shared_task_id` is present in the `task.upsert` payload, the task is hive-managed and must have a `node_task_assignments` row. However, ALL tasks (including node-owned ones) receive a `shared_task_id` upon their first sync to the hive. Because node-owned tasks correctly lack an assignment row (ADR-0009), any subsequent update to a node-owned task will fail the assignment lookup and be permanently rejected (`"no active assignment for hive-managed task"`). This flawed predicate also causes the Phase 3 status matrix guard (`session.rs:2061`) to execute for node-owned tasks, incorrectly rejecting node-authored transitions like `Todo -> InProgress`.
- Impact if shipped: Nodes can never update their own tasks once they have synced to the hive. The op-log is effectively wedged for node-owned work, breaking fundamental synchronization.
- Remediation: Query `shared_tasks` to check `owner_node_id` instead of relying solely on the presence of `shared_task_id`. If `owner_node_id == node_id`, the task is node-owned and both the fencing check and status matrix guard should be bypassed.
```rust
        let is_node_owned = if let Some(sid) = shared_id {
            let task_owner: Option<(Option<Uuid>,)> = sqlx::query_as(
                "SELECT owner_node_id FROM shared_tasks WHERE id = $1"
            )
            .bind(sid)
            .fetch_optional(pool)
            .await
            .map_err(|e| HandleError::Database(e.to_string()))?;
            matches!(task_owner, Some((Some(owner_id),)) if owner_id == node_id)
        } else {
            true // No shared_task_id means first sync of a node-created task
        };

        if !is_node_owned {
            // Apply fencing check for hive-managed tasks
            if let Some(shared_id) = shared_id {
                let assignment: Option<(Uuid, i64)> = sqlx::query_as( ... )
                // ... existing assignment lookup and stale-token rejection block
            }
        }
```
*Note: Wrap the status matrix guard (`session.rs:2061`) in the exact same `if !is_node_owned { ... }` block.*

VERDICT: REVISE
TOTAL FINDINGS: 2 (2 [BLOCKING], 0 [SHOULD-FIX], 0 [INFO])