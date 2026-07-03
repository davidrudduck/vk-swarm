# Post-Phase 5 Integrated Adversarial Review — Anti-Entropy (SC5/ADR-0008)

**Date:** 2026-07-03
**Panelist:** opencode (orchestrator inline)
**Diff range:** `2ed6cacc..e7c88fb7` (6 commits, tasks 501-504)
**Report:** inline (this file)

## VERDICT: DEVIATES → REMEDIATED IN-SESSION

### F1 — Soft-deleted task stuck-loop (CROSS-TASK: 503 × 106)

**Root cause:** `list_source_task_versions_for_node` filtered `deleted_at IS NULL` → tombstoned task absent from `hive_ids` → compare sees "node-has/hive-lacks" → `resend_from_seq=Some(1)` → 504 re-streams → `handle_op_batch_apply` (106) skips (`seen=true` in `node_op_log`) → no change → infinite loop.

**Fix:** Added `list_soft_deleted_source_task_ids_for_node` (tasks.rs). `handle_digest_compare` now:
- routes soft-deleted tasks the node still has into `pull_entities` (reconcile leg → `deleted_task_ids` → `unlink_by_shared_task_id` → convergence);
- only sets `resend_from_seq` if node has something the hive has NEVER seen (not active AND not deleted).

**Test:** `digest_routes_soft_deleted_task_to_pull_not_restream` — seeds soft-deleted task, asserts `pull_entities.contains(&sid)` + `resend_from_seq == None`.

### F2 — >500 ops stuck-loop (CROSS-TASK: 503 × 504)

**Root cause:** `resend_from_seq=Some(1)` + `RESTREAM_LIMIT=500` + no cursor advancement = ops beyond seq 500 never heal (each cycle sends [1,500], all `seen=true`, lost op at seq 501+ never reached).

**Fix:** 504's re-stream leg now paginates — loop advancing `from_seq = last_seq + 1` until a batch returns `< RESTREAM_LIMIT` rows. Hive apply is idempotent (`ON CONFLICT DO NOTHING`), so the burst is safe.

### Conforms (8 items)

- #2 entity_type: 502 sends "task", 503 filters "task" — consistent.
- #3 entity_id: 502 maps local task id, 503 compares against `source_task_id` (id-bridge) — consistent. 504 pull leg over-pulls (full reconcile) but converges.
- #5 acked ops: `mark_acked_through` only touches `acked_at IS NULL` rows — re-acking is no-op.
- #6 WS enum: `DigestEntry` byte-identical across crates, same serde renames.
- #7 placement: `sync_digest` after `sync_outbox` — correct (digest reads tasks table, not outbox).
- #8 state access: `handle.state.read().await` block releases before `sync_remote_projects`; `org_id`/`node_id` immutable per session — no race.
- #9 no reset: no `reset_*`/`TRUNCATE`/`DELETE` in heal path (only test cleanup).
- #10 tombstone: addressed by F1 fix.