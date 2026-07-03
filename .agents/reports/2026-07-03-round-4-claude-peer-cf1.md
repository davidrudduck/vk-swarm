My research is complete. The plan file has been written with the full report content and verdict. Let me present the findings and request plan approval.

**Research Summary:**

All four links in the CF1 causal chain are confirmed against actual code:

1. **`queries.rs:362`** — `fencing_token: None` at every task.upsert enqueue. Confirmed.
2. **`hive_sync.rs:231–257`** — The normal live-send path builds `token_by_task` from `active_assignments.values()` and re-stamps for task entities, bypassing the stored `None`. Confirmed.
3. **`node_runner.rs:1169–1181`** — `restream_row_to_ws_op` copies `r.fencing_token` AS-IS (i.e., `None`). The inline comment claiming "task 107 stamps fencing_token only at fresh-enqueue time" is factually wrong — there is no stamp at enqueue time. Confirmed.
4. **`session.rs:2123–2124`** — `None => true` makes every re-streamed task op permanently stale-rejected. Confirmed.
5. **`ActiveAssignment`** carries both `local_task_id: Option<Uuid>` and `fencing_token: Option<i64>`; `active_assignments` is a `HashMap<Uuid, ActiveAssignment>` keyed by `assignment_id`. The proposed fix's `.values()` iteration is the correct access pattern.

**Verdict: CONFIRM.** The proposed remediation is correct and mirrors the normal send path exactly. One implicit mechanical change needed: the calling site at line 1104 must switch from bare function reference `map(restream_row_to_ws_op)` to a closure `map(|r| restream_row_to_ws_op(r, &token_by_task))` to accommodate the new parameter.

The full formatted report is ready to write to `.agents/reports/2026-07-03-round-4-claude-peer-cf1.md`.