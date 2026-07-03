I did not write `/data/Code/vk-swarm/.worktrees/vk-swarm-hive-redesign-p47/.agents/reports/2026-07-03-round-4-codex-discovery.md` because your instructions also said not to modify files or repository state, and this environment is read-only. Report content:

### FINDING F1: Digest re-stream drops fencing tokens for assigned-task ops
- Tag: [BLOCKING]
- Location: crates/services/src/services/node_runner.rs:1179
- Evidence: `restream_row_to_ws_op` replays the stored row token: `fencing_token: r.fencing_token`. But task outbox rows are enqueued with `fencing_token: None` at `crates/db/src/models/task/queries.rs:356-362`, while the normal send path only stamps the token in memory at `crates/services/src/services/hive_sync.rs:251-254`. The hive then treats `None` as stale at `crates/remote/src/nodes/ws/session.rs:2122-2125`.
- Problem: SC5 digest healing replays acked rows using `peek_from_seq`; for hive-assigned tasks those replayed ops lose the current lease token and are rejected by the SC3 fencing guard. Anti-entropy healing can therefore fail specifically for the assigned-task path that needs fencing.
- Remediation:
```diff
diff --git a/crates/services/src/services/node_runner.rs b/crates/services/src/services/node_runner.rs
@@
                     if let Some(mut from_seq) = resend_from_seq {
                         use db::models::node_outbox::OutboxRepository;
+                        let token_by_task: HashMap<Uuid, i64> = {
+                            let s = handle.state.read().await;
+                            s.active_assignments
+                                .values()
+                                .filter_map(|a| Some((a.local_task_id?, a.fencing_token?)))
+                                .collect()
+                        };
@@
                                     let ops: Vec<super::hive_client::OutboxOp> =
-                                        rows.into_iter().map(restream_row_to_ws_op).collect();
+                                        rows.into_iter()
+                                            .map(|r| restream_row_to_ws_op(r, &token_by_task))
+                                            .collect();
@@
 fn restream_row_to_ws_op(
     r: db::models::node_outbox::OutboxOp,
+    token_by_task: &HashMap<Uuid, i64>,
 ) -> super::hive_client::OutboxOp {
+    let fencing_token = if r.entity_type == "task" {
+        token_by_task.get(&r.entity_id).copied().or(r.fencing_token)
+    } else {
+        r.fencing_token
+    };
     super::hive_client::OutboxOp {
@@
-        fencing_token: r.fencing_token,
+        fencing_token,
     }
 }
```
- Remediation-verification: Add a unit test for `restream_row_to_ws_op` where the stored row has `entity_type = "task"`, `fencing_token = None`, and `token_by_task[entity_id] = 7`; assert the emitted WS op has `Some(7)`. Then run `cargo test -p services restream_row_to_ws_op`.

### FINDING F2: Soft-deleted digest entries can still trigger an unnecessary re-stream
- Tag: [BLOCKING]
- Location: crates/remote/src/nodes/ws/session.rs:2469
- Evidence: The resend condition is:
  `node_ids.difference(&hive_ids).next().is_some() && node_ids.difference(&hive_deleted_ids).next().is_some()`.
- Problem: If the digest contains one active in-sync task `A` and one hive-soft-deleted task `B`, then `B` makes `node_ids - hive_ids` non-empty, while `A` makes `node_ids - hive_deleted_ids` non-empty. The hive returns `resend_from_seq = Some(1)` even though the only “missing” task is actually tombstoned and should be handled only through `pull_entities`. In the worst case where `node_op_log` lost the old dedup row, replay can recreate an active duplicate because `upsert_from_node` conflicts only against non-deleted rows.
- Remediation:
```diff
diff --git a/crates/remote/src/nodes/ws/session.rs b/crates/remote/src/nodes/ws/session.rs
@@
-    let resend_from_seq = if node_ids.difference(&hive_ids).next().is_some()
-        && node_ids.difference(&hive_deleted_ids).next().is_some()
-    {
+    let has_node_only_non_tombstoned = node_ids
+        .difference(&hive_ids)
+        .any(|id| !hive_deleted_ids.contains(id));
+    let resend_from_seq = if has_node_only_non_tombstoned {
         Some(1i64)
     } else {
         None
     };
```
- Remediation-verification: Extend `digest_routes_soft_deleted_task_to_pull_not_restream` to seed one active hive row `A`, one soft-deleted hive row `B`, and digest entries for both. Assert `pull_entities` contains `B` and `resend_from_seq == None`. Run `cargo test -p remote digest_routes_soft_deleted_task_to_pull_not_restream` with `DATABASE_URL` set to a migrated Postgres.

VERDICT: REVISE