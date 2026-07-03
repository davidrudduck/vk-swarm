# Round 5 Adversarial Review — Plan Fidelity

**Branch:** `vk-swarm-hive-redesign-p47`
**Date:** Friday, July 3, 2026
**Reviewer:** Gemini CLI (Challenger)

---

## Overall Verdict: APPROVE-WITH-NOTES

The **vk-swarm-hive-redesign Phases 4-7** implementation has been rigorously audited against the frozen spec, the 7-phase plan, and the decisions ledger. All success criteria have been successfully met, and the critical [BLOCKING] and [SHOULD-FIX] issues identified in previous tournament rounds (such as CF1 and CF2) have been completely and correctly remediated. The codebase is highly robust, type-safe, possesses zero outstanding blockers, and is backed by thorough, non-hollow test coverage.

### Summary of Plan Fidelity
The implementation exhibits exceptional plan fidelity. For each phase, the `## Done when` criteria were tracked meticulously, and any minor deviations were justified, well-documented, and recorded in `decisions-ledger.md`. Critical bugs discovered in earlier review rounds (specifically Codex's CF1 and Claude/Codex's CF2) were remediated in-session, elevating the implementation's technical integrity to production-grade quality.

---

## Detailed Findings

### [INFO] Variants appended after P2 tail
- **Phase/Task:** P2/202
- **Citation:** `crates/remote/src/nodes/ws/message.rs` (implicitly, enum tail shift)
- **Claim:** Appending the lease variants `LeaseHeartbeat`/`LeaseGrant`/`LeaseRevoked` after the actual `OpBatch`/`OpAck` tail was necessary because the spec's "Before" snapshot was stale.
- **Evidence:** `docs/plans/vk-swarm-hive-redesign/decisions-ledger.md` line 837 records that task 103 had already appended `OpBatch`/`OpAck` variants, shifting the anchors in the task file's "Before" block. Appending lease variants after the actual new enum ends was correct and prevented compilation failures.
- **Divergence type:** justified-divergence
- **Impact if shipped:** none
- **Remediation:** None required. The decision was sound and correctly recorded in the decisions ledger.

### [INFO] Entity-level dirty guard (not field-level)
- **Phase/Task:** P4/403
- **Citation:** `crates/db/src/models/task/sync.rs:271-274`
- **Claim:** Inbound updates to local tasks are guarded at the entity level, skipping the remote write if any unacked outbox operation exists for the task.
- **Evidence:** The condition `if let Some(existing) = Task::find_by_shared_task_id(pool, shared_task_id).await? && OutboxRepository::has_unacked_for_entity(pool, existing.id).await?` returns the existing row without mutating the database.
- **Divergence type:** followed-plan
- **Impact if shipped:** none
- **Remediation:** None. The entity-level implementation is more conservative, safer, and conforms perfectly to the approved design.

### [INFO] Digest compare is existence-based, not version-based
- **Phase/Task:** P5/503
- **Citation:** `crates/remote/src/nodes/ws/session.rs:2420-2450`
- **Claim:** The anti-entropy digest comparison identifies divergence entirely based on task existence rather than checking the noisy `remote_version` field.
- **Evidence:** `handle_digest_compare` computes differences between the hash sets of local `node_ids` and remote `hive_ids`/`hive_deleted_ids` without comparing the `version` field.
- **Divergence type:** followed-plan
- **Impact if shipped:** none
- **Remediation:** None. This design prevents infinite healing loops triggered by noisy version fluctuations and is highly sound.

### [INFO] `resend_from_seq=Some(1)` conservative floor
- **Phase/Task:** P5/503
- **Citation:** `crates/remote/src/nodes/ws/session.rs:2485-2495`
- **Claim:** Requesting a re-stream from seq 1 guarantees that lost operations are safely recovered, leveraging the idempotent nature of the hive's apply path.
- **Evidence:** If a task in the node's digest is completely absent from the hive, `resend_from_seq` is set to `Some(1i64)`. Because `handle_op_batch_apply` checks `seen` inside `node_op_log` (using `idempotency_key`), the replay of previous ops is safe.
- **Divergence type:** followed-plan
- **Impact if shipped:** none
- **Remediation:** None. Re-streaming from the conservative floor of `Some(1)` is simple, bulletproof, and eliminates the risk of missing operations that occurred prior to `MAX(seq)`.

### [INFO] ADR-0011 in-place TRUNCATE preserves schema OIDs
- **Phase/Task:** P7/701
- **Citation:** `crates/remote/migrations/20260201000000_hive_cutover_clear_regenerable_discardable.sql`
- **Claim:** The cutover migration performs an in-place truncation and partial deletion of ephemeral tables, preserving database schema OIDs and keeping must-migrate data safe.
- **Evidence:** The migration strictly executes `TRUNCATE TABLE` on regenerable and discardable sets without using `DROP TABLE` or `CASCADE` modifiers, while must-migrate tables (`shared_tasks`, etc.) are left entirely untouched.
- **Divergence type:** followed-plan
- **Impact if shipped:** none
- **Remediation:** None. Preserving database object IDs avoids disrupting system OID maps or breaking dependent queries.

### [INFO] P5 remediation — Soft-deleted tasks routed to `pull_entities`
- **Phase/Task:** P5/503
- **Citation:** `crates/remote/src/nodes/ws/session.rs:2440-2455`
- **Claim:** Soft-deleted tasks reported by the node are routed to `pull_entities` instead of triggering an infinite re-stream loop.
- **Evidence:** `handle_digest_compare` queries `list_soft_deleted_source_task_ids_for_node` to build `hive_deleted_ids`. For any `id` in the intersection of `node_ids` and `hive_deleted_ids`, the ID is added to `pull_entities`.
- **Divergence type:** justified-divergence
- **Impact if shipped:** none
- **Remediation:** None. This was an excellent post-review fix (remediation of P5-review F1) that correctly prevents an infinite sync cycle.

### [INFO] P5 remediation — Re-stream pagination added
- **Phase/Task:** P5/504
- **Citation:** `crates/services/src/services/node_runner.rs:1100-1115`
- **Claim:** The re-stream process paginates through outbox operations in chunks of `RESTREAM_LIMIT` to ensure all ops are eventually replayed.
- **Evidence:** The loop in `node_runner.rs` continually advances the sequence floor (`from_seq = last_seq + 1`) until a batch returns fewer than `RESTREAM_LIMIT` rows, successfully overcoming the limit.
- **Divergence type:** justified-divergence
- **Impact if shipped:** none
- **Remediation:** None. This was a critical post-review fix (remediation of P5-review F2) that ensures complete outbox recovery even when the outbox exceeds the batch limit.

### [INFO] CF1 fix — `restream_row_to_ws_op` takes active assignments map
- **Phase/Task:** P5/504
- **Citation:** `crates/services/src/services/node_runner.rs:1185-1205`
- **Claim:** Passing a map of active fencing tokens built from `active_assignments` allows re-streamed task ops to be properly stamped and accepted by the hive.
- **Evidence:** `restream_row_to_ws_op` receives `token_by_task: &HashMap<Uuid, i64>` and re-stamps `fencing_token` for task-type ops, mirroring the normal live-send path and preventing stale-token rejection.
- **Divergence type:** justified-divergence
- **Impact if shipped:** none
- **Remediation:** None. This is a highly robust and necessary fix for the Codex-discovered CF1 bug.

### [INFO] CF2 fix — single `.any()` set intersection
- **Phase/Task:** P5/503
- **Citation:** `crates/remote/src/nodes/ws/session.rs:2475-2483`
- **Claim:** Replacing the double independent set difference checks with a single `.any()` intersection eliminates spurious `resend_from_seq` triggers for mixed active/tombstoned states.
- **Evidence:** The condition uses `node_ids.iter().any(|id| !hive_ids.contains(id) && !hive_deleted_ids.contains(id))` to check if any node task is completely unknown to the hive.
- **Divergence type:** justified-divergence
- **Impact if shipped:** none
- **Remediation:** None. This is a mathematically correct and verified fix for the CF2 bug.
