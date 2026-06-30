---
id: "205"
phase: 2
title: Hive fencing enforcement in handle_op_batch — reject stale-token ops, emit LeaseRevoked
status: ready
depends_on: ["106", "202", "203"]
parallel: false
conflicts_with: ["202", "204"]
files:
  - crates/remote/src/nodes/ws/session.rs
irreversible: false
scope_test: "crates/remote/src/nodes/ws/session.rs"
allowed_change: edit
covers_criteria: [SC3]
covers_tests: []
---
## Failing test (write first)
**PRECONDITION (Trap 2b — NON-NEGOTIABLE):** a `#[cfg(test)] mod` INSIDE `session.rs` (the handler and the
new fencing check are private to the module). REQUIRES a live, migrated Postgres (the `201` columns +
`node_fencing_token_seq`, `node_op_log` from 102, swarm-link tables). A run without `DATABASE_URL` returns
early (skip) = HOLLOW pass. Stand up Postgres, export `DATABASE_URL=postgres://…`, or RAISE.

**Sibling read (rubric #9):** task 106's `op_batch_tests` module is the direct sibling — read it and reuse
its fixtures (org/node/swarm_project/node_local_projects seeding) and the same ws-free `handle_op_batch_apply`
split IF 106 took it (STOP note in 106 records whether it did). `backfill_e2e.rs` is the helper source.

This task's tests prove the **stale-token rejection** (the SC3 commit-effect guarantee). Add to the
`op_batch_tests` module (or a new `fencing_tests` module beside it — keep one if reusing fixtures):
```rust
    #[tokio::test]
    async fn op_against_assigned_task_with_stale_token_is_rejected_not_applied() {
        skip_without_db!();
        let pool = create_pool().await;
        // FULL reassignment seed (so a WRONG-KEY lookup returns empty and this test FAILS — non-hollow):
        //   - org + node_a + node_b + swarm-linked project.
        //   - a shared_tasks row with (source_node_id = node_a, source_task_id = a_local_task_id) so the
        //     fence's find_by_source_task_id(node_a, a_local_task_id) resolves the shared id (tasks.rs:352).
        //   - node_task_assignments active row keyed on that shared id, reassigned to node_b with token T2
        //     (try_claim node_a@T1 with past TTL, then reclaim_expired_leases / try_claim node_b → T2 > T1).
        // Now node_a (partitioned-but-alive) sends an op: node_id = node_a, payload.id = a_local_task_id,
        //   fencing_token = T1 (stale vs the assignment's current T2). Apply it. ASSERT:
        //   - shared_tasks is NOT updated by the stale op (no apply),
        //   - node_op_log has NO dedup row for that op (no record),
        //   - applied_through_seq does NOT advance past the rejected op's seq (high-water unchanged),
        //   - a LeaseRevoked for node_a's (old) assignment was emitted (or the rejection surfaced as the
        //     contract specifies — see ws-free split note).
    }

    #[tokio::test]
    async fn op_with_current_token_against_assigned_task_applies_normally() {
        skip_without_db!();
        let pool = create_pool().await;
        // node_b holds the task (token T2). node_b sends an op stamped fencing_token = T2 → applies,
        //   records node_op_log, advances applied_through_seq. The fence does NOT block the rightful holder.
    }

    #[tokio::test]
    async fn op_with_null_token_node_owned_work_is_unaffected_by_the_fence() {
        skip_without_db!();
        let pool = create_pool().await;
        // an op whose task has NO active assignment AND fencing_token = None (node-owned work, CONTRACT
        //   §C / ADR-0009): the stale-token check does NOT apply; the op applies as in 106. Proves the
        //   fence is scoped to hive-assigned tasks only and never bounces node-owned ops.
    }
```

## Change
- **File:** `crates/remote/src/nodes/ws/session.rs`
- **Anchor:** INSIDE `handle_op_batch` (the fn task 106 added beside `handle_task_sync` @1547), in the
  per-op loop — specifically the apply path **(c)** of 106's contract (the `seen == false` →
  `upsert_from_node` branch). The fencing check is a NEW guard inserted BEFORE the apply, evaluated for
  every op whose task has an active assignment.
- **Sibling read (rubric #9):** 106's `handle_op_batch` already resolves the op's task context (project
  link, swarm link) and iterates ops in order with PARK / SKIP+ADVANCE / APPLY branches. The fencing guard
  slots into the APPLY branch (an op only reaches apply if its task is swarm-linked). Read 106's exact loop
  shape (PARK=break-no-advance, SKIP+ADVANCE=record+advance, APPLY=upsert) before inserting — the fence is
  a fourth terminal outcome: REJECT (no apply, no record, no advance, emit LeaseRevoked).
- **Add the fencing guard (CONTRACT §C / ADR-0009).** For the op currently being applied, after its task
  context is resolved and BEFORE `upsert_from_node`:
  1. **Resolve the HIVE shared-task id (the load-bearing key — get this wrong and SC3 silently breaks).**
     The op arrives on the SENDER node's session, so `node_id` = the sender (node A, the partitioned writer)
     and `op.entity_id`/`payload.id` = A's LOCAL task id. The assignment row, however, is keyed on
     `node_task_assignments.task_id` which is `shared_tasks.id` (FK, verified
     `20251202000000_nodes_swarm.sql:75`). So you MUST first map A's local id → the shared id via the
     **existing** `SharedTaskRepository::find_by_source_task_id(source_node_id = node_id, source_task_id =
     payload.id)` (`tasks.rs:352`, the `(source_node_id, source_task_id)` unique-constraint lookup). Do NOT
     key the assignment lookup on `node_id`/`local_task_id` directly — after reassignment the active row is
     node B's, so a sender-keyed lookup finds NOTHING and the stale op would FALL THROUGH and apply = the
     exact double-execution SC3 forbids. (If 106's context step already produced this shared id pre-apply,
     reuse it; if it only resolves it INSIDE `upsert_from_node`'s `ON CONFLICT`, resolve it here explicitly
     with `find_by_source_task_id` BEFORE the apply.)
     - If `find_by_source_task_id` returns `None` → no shared task exists yet (first write) → no assignment
       can exist → fence does not apply; fall through to 106's normal apply.
  2. **Look up the active assignment by the shared id:**
     `SELECT id, fencing_token FROM node_task_assignments WHERE task_id = $shared_id AND completed_at IS NULL`
     (a narrow scalar/row read — do NOT use `NodeTaskAssignment` FromRow; the new column is not on that
     struct — see 203's judgment call).
  3. **No active assignment** → the fence does NOT apply (node-owned work or unassigned). Fall through to
     106's normal apply. (`op.fencing_token` may be `None` here; that is correct — CONTRACT §C.)
  4. **Active assignment present** → compare:
     - If `op.fencing_token` is `None` OR `op.fencing_token < assignment.fencing_token` → **REJECT**: do
       NOT call `upsert_from_node`, do NOT INSERT `node_op_log`, do NOT advance `applied_through_seq` past
       this op. Emit `send_message(ws_sender, &HiveMessage::LeaseRevoked { assignment_id: <the assignment's
       id from step 2>, reason: "stale fencing token".into() })` — the partitioned writer learns its lease
       is gone. Then STOP applying this op (the contract says do NOT advance high-water past a rejected op;
       mirror 106's PARK control-flow of NOT advancing, but this is a permanent reject, not a transient
       park — log at warn, and break/return so the cursor does not skip it). Record the precise control
       choice (break vs continue-without-advance) in the ledger; the test asserts the high-water does not
       pass the rejected seq.
     - If `op.fencing_token >= assignment.fencing_token` → the rightful current holder: fall through to
       106's normal apply.
  > **Scope (ADR-0009 §3):** the fence applies ONLY to ops against a hive-assigned task. Node-owned work
  > (no assignment) carries `fencing_token = None` and is committed under the node's ownership identity —
  > the stale-token check MUST NOT bounce it. The third test guards this.
- **ws-free split:** if 106 extracted a `handle_op_batch_apply` (no `ws_sender`) for testability, the
  fencing REJECT must still surface the `LeaseRevoked` emission. Either (a) thread an "ops to revoke"
  `Vec<(assignment_id, reason)>` out of the apply fn and have `handle_op_batch` send them after, or (b)
  keep the send in the wire fn. Pick one, mirror 106's split, and record it.

## Allowed moves
ONLY: add the active-assignment lookup + the stale-token guard inside `handle_op_batch`'s apply path, the
`LeaseRevoked` emission on reject, and the fencing test(s). Reuse the existing context resolution from 106,
`send_message`, and `HiveMessage::LeaseRevoked` (202). Do NOT re-implement 106's apply/park/skip logic, do
NOT touch `try_claim`/`renew_lease` (203), the WS enums (202), `handle_lease_heartbeat` (204), the node
side, or any migration.

## STOP triggers
- The fence bounces an op whose task has NO active assignment (node-owned work, `fencing_token = None`) →
  BUG: the fence is scoped to hive-assigned tasks (CONTRACT §C). Only ops whose task has an active
  assignment are subject to the token compare. The third test catches this.
- A rejected op ADVANCES `applied_through_seq` (or records a `node_op_log` dedup row) → BUG: a stale op must
  NOT be acked; advancing past it would let the node believe a bounced write was committed (silent loss in
  reverse). Do NOT advance, do NOT record. The first test asserts the high-water is unchanged.
- Keying the assignment lookup on the SENDER's `node_id`/`local_task_id` instead of the resolved
  `shared_tasks.id` → **THE SC3-BREAKING BUG**: after reassignment the active row is node B's, so a
  sender-keyed lookup finds nothing, the stale op falls through and APPLIES = double execution. The fence
  MUST resolve `find_by_source_task_id(node_id, payload.id) → shared id` first, then look up the assignment
  by that shared id. The first test seeds the full reassignment so a wrong key makes it fail (non-hollow).
- Comparing against the WRONG token (e.g. the node's claimed token instead of the assignment's CURRENT
  token) → BUG: the authority is `node_task_assignments.fencing_token` (bumped on every (re)claim by 201's
  sequence). Read it live per op.
- `handle_op_batch` (106) is absent or its loop shape differs from the read → STOP: 205 depends on 106
  having landed (depends_on: 106). If 106 drifted, re-locate the apply branch and record.
- Using `NodeTaskAssignment` FromRow to read `fencing_token` → STOP: the column is NOT on that struct
  (203's judgment call). Use a narrow `SELECT fencing_token, id` scalar/row read.

## Done when
`WAI_TYPECHECK_CMD="cargo check -p remote" WAI_TEST_CMD='test -n "$DATABASE_URL" && cargo test -p remote fencing' bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-hive-redesign 205` exits 0
(run with `DATABASE_URL=postgres://…` against a migrated Postgres — Trap 2b; `test -n` prefix FAIL-CLOSED.
If the tests live in the `op_batch_tests` module instead of a `fencing` module, set the test filter to the
module/test names you used and record it.)
