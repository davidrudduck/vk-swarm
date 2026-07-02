# Gemini Adversarial Review — vk-swarm-hive-redesign (P1+P2 implementation)

## Target

The merged code on branch `worktree-bridge-cse_01B8B52jjMEdikaLRoA25qhr` (HEAD `9c4f9acd`) vs `origin/main` (`648692a6`). Diff scope: 15 Rust files, +3724/-16 lines, spanning two phases: Phase 1 (op-log foundation, SC2) and Phase 2 (lease/fencing partition-safety, SC3).

### [BLOCKING] Non-transactional outbox enqueue breaks SC2 no-loss and ordering guarantees
- **Tag:** [BLOCKING]
- **Lens:** fidelity / mechanics
- **Claim:** The `enqueue_task_upsert_op` function is executed asynchronously after the entity `INSERT` / `UPDATE` commits, rather than within the same database transaction. This introduces a TOCTOU race condition where a crash between the two statements silently loses the outbox op (violating SC2c zero silent write loss). Additionally, concurrent task creations can result in child ops receiving lower outbox sequences than parent ops, breaking the SC2b parent-before-child ordering guarantee. While the decisions ledger notes this as a "tracer limitation", it violates the fundamental structural intent of SC2.
- **Citation:** `crates/db/src/models/task/queries.rs:339` (`async fn enqueue_task_upsert_op(pool: &SqlitePool, task: &Task)`) and `Task::create` at line 292 where it is called asynchronously after the `INSERT` completes.
- **Impact if shipped:** Node crashes will silently lose writes. Concurrent creations will wedge the hive sync queue due to out-of-order parent/child events.
- **Remediation:** Thread a `&mut sqlx::Transaction` through `Task::create` and `Task::update`, executing both the entity write and the `enqueue_task_upsert_op` sequentially inside the same SQLite transaction before committing.

### [BLOCKING] Fencing guard bypasses completed tasks, allowing double execution
- **Tag:** [BLOCKING]
- **Lens:** mechanics / fidelity
- **Claim:** The fencing guard in `handle_op_batch_apply` looks up the hive assignment using `WHERE task_id = $1 AND completed_at IS NULL`. For a task that has been completely finished by another reassigned node (or operator-cancelled), there is no active assignment, so the query returns `None`. The logic then incorrectly falls through to the normal apply path (assuming it is node-owned or unassigned work). This allows a partitioned node's late write to bypass the fence and overwrite a completed task's status.
- **Citation:** `crates/remote/src/nodes/ws/session.rs:1958` (`SELECT id, fencing_token FROM node_task_assignments WHERE task_id = $1 AND completed_at IS NULL`) and the unsafe fall-through at `crates/remote/src/nodes/ws/session.rs:1989` (`// No active assignment → fall through to 106's normal apply ...`).
- **Impact if shipped:** A partitioned node's late commit can resurrect and overwrite a completely finished task, violating the core partition-safety goal of SC3.
- **Remediation:** If `assignment` is `None` but `payload.shared_task_id` is present, explicitly verify whether the sending node is the task's original creator (`source_node_id == node_id`). If it is a hive-assigned task (not node-owned) and has no active assignment, the op must be rejected as stale rather than falling through to apply.

## Verdict
- SC2 met: PARTIAL — Single channel and ack cursor are implemented, but the non-transactional enqueue violates the no-loss and parent-before-child cross-entity ordering guarantees.
- SC3 met: PARTIAL — The fencing token bump and self-fence are structurally sound, but the hive-side fencing guard bypasses completely finished tasks, failing to prevent a partitioned node from overwriting completed work.
- Overall: FAIL