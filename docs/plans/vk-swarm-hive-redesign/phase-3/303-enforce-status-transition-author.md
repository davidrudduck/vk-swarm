---
id: "303"
phase: 3
title: Enforce single-author status transitions at handle_op_batch (node-reported gated on lease+token)
status: ready
depends_on: ["301", "302"]
parallel: false
conflicts_with: ["302", "304"]
files:
  - crates/remote/src/nodes/ws/session.rs
irreversible: false
scope_test: "crates/remote/src/nodes/ws/session.rs"
allowed_change: edit
covers_criteria: [SC4]
covers_tests: [TS3]
---
## Reconciled matrix (ADR-0010, ratified 2026-06-30)
This task enforces 301's reconciled (ratified) matrix over the real `TaskStatus` enum
(`Todo/InProgress/InReview/Done/Cancelled` — no `Failed`, no `Assigned`). The TS3 table below uses that
matrix: node authors `InProgress→Done` / `InProgress→InReview`; hive authors `Todo→InProgress`,
`InReview→Done`, `InReview→InProgress`, and `*→Cancelled`. `failed`/`assigned` are NOT `task.status`
values (execution / assignment layers). See `dev-docs/adr/0010-task-status-state-machine.md` (## Decision)
and 301's reconciled matrix.

## Failing test (write first)
**PRECONDITION (Trap 2b — NON-NEGOTIABLE):** REQUIRES a live, migrated Postgres (`shared_tasks`,
`node_task_assignments` **incl. P2's `fencing_token`/`lease_expires_at` columns**, `node_op_log`,
`swarm_projects`/`node_local_projects`). A run without `DATABASE_URL` returns early (skip) = HOLLOW pass.
Stand up Postgres, `sqlx::migrate!("./migrations")`, export `DATABASE_URL=postgres://…` before the gate.

**PRECONDITION (P2 — read CONTRACT §C):** this task GATES on P2's lease + fencing. P2 (a) adds
`fencing_token BIGINT NOT NULL DEFAULT 0` + `lease_expires_at TIMESTAMPTZ` to `node_task_assignments`
(CONTRACT §B) and (b) adds the fencing check to `handle_op_batch` that rejects an op whose
`op.fencing_token < assignment.fencing_token` (CONTRACT §C). **303 does NOT re-implement the fencing
check — it rides it** and adds the orthogonal *transition-legality* guard. If `node_task_assignments`
has no `fencing_token` column, or `handle_op_batch` has no P2 fencing check, **P2 has not landed → STOP**
(see STOP triggers; P2's fencing seam — tasks 201/203/205 — must land first. It is encoded as a prose+STOP
precondition rather than a `depends_on` edge per the ledger's phase-by-phase model; `depends_on` carries
the intra-P3 edges 301/302).

**The test MUST be a `#[cfg(test)] mod` INSIDE `session.rs`** (the `handle_op_batch` apply path is private;
an integration test in `crates/remote/tests/` sees only `pub` items). **Sibling read (rubric #9):** copy
the helpers from `crates/remote/tests/backfill_e2e.rs` verbatim — `fn database_url()`, the `skip_without_db!`
macro, `async fn create_pool()`, the `create_test_organization`/`create_test_node` fixtures — AND reuse
106's `op_batch_tests` seeding (org + node + swarm_project + `node_local_projects` link). There is NO shared
`common` module — inline the helpers (106 already inlines them; you may add a second `#[cfg(test)] mod`
beside `op_batch_tests`, or extend it).

This is **TS3 — table-driven over the transition matrix** (spec ## Test strategy). Each legal transition is
accepted from its SOLE author and rejected from the other party / without a valid lease+token:

```rust
#[cfg(test)]
mod status_guard_tests {
    use super::*;
    // inline: database_url(), skip_without_db!, create_pool(), seed_org_node_swarm_linked_project(),
    //         seed_shared_task_with_status(pool, source_node_id, local_task_id, TaskStatus) -> task_id,
    //         seed_active_assignment(pool, task_id, node_id, fencing_token) -> assignment_id
    //         (copied from backfill_e2e.rs + 106's op_batch_tests).

    // A node→hive op carries a status change. We drive it through the SAME apply path
    // `handle_op_batch` uses (extract the per-op apply if needed — see STOP note) and assert
    // accept vs reject.

    #[tokio::test]
    async fn node_reported_in_progress_to_done_accepted_with_valid_lease_and_token() {
        skip_without_db!();
        let pool = create_pool().await;
        // seed: shared_task status='in-progress'; active assignment with fencing_token = T.
        // op: task.upsert payload.status="done", op.fencing_token = Some(T).  → APPLIED:
        //   shared_tasks.status == 'done'; applied_through_seq advances.
    }

    #[tokio::test]
    async fn node_reported_done_rejected_without_lease_or_current_token() {
        skip_without_db!();
        let pool = create_pool().await;
        // (a) no active assignment at all → node-reported in-progress→done REJECTED (no lease):
        //     status stays 'in-progress', op NOT applied. (Rides P2: no lease ⇒ not a valid commit.)
        // (b) active assignment fencing_token = T_new, op.fencing_token = Some(T_old) (T_old < T_new) →
        //     REJECTED by P2's stale-token check; status unchanged. (303 asserts the seam, does not
        //     re-implement it.)
    }

    #[tokio::test]
    async fn hive_authored_transition_rejected_when_reported_by_node() {
        skip_without_db!();
        let pool = create_pool().await;
        // shared_task status='in-review'; op (from a node) payload.status="cancelled" (a *→cancelled,
        // HIVE-authored transition). Even WITH a valid lease+token, a NODE may not author it →
        // REJECTED: status stays 'in-review'. Same for an op trying in-review→done from the node (a
        // hive operator-review transition). This is the core SC4 single-author rejection
        // (node_may_author == false).
    }

    #[tokio::test]
    async fn illegal_transition_rejected_from_either_party() {
        skip_without_db!();
        let pool = create_pool().await;
        // shared_task status='done'; node op payload.status="in-progress" (done→in-progress is in NO
        // author's column → illegal) → REJECTED, status stays 'done'. Asserts illegal transitions are
        // rejected, not merged.
    }

    #[tokio::test]
    async fn noop_same_status_is_not_a_rejected_transition() {
        skip_without_db!();
        let pool = create_pool().await;
        // shared_task status='in-progress'; node op payload.status="inprogress" (from==to). A no-op MUST
        // NOT be treated as an illegal transition that wedges the op — it is applied as an idempotent
        // upsert (other fields may change) and the cursor advances. Assert: no error, applied_through_seq
        // advances, status still 'in-progress'.
    }
}
```
> The no-op case is load-bearing: `task.upsert` ops carry the WHOLE task (title/description too), so most
> ops do NOT change status. A from==to op must pass the guard (it is not a transition), or every metadata
> edit on an in-progress task would be rejected and wedge the cursor.

## Change
- **File:** `crates/remote/src/nodes/ws/session.rs`
- **Anchor:** inside `handle_op_batch` step (c)/(d) — AFTER 302's `canonical_status_from_node(status_str)`
  produces the incoming `status: TaskStatus`, and AT/AROUND the same site where P2 adds its fencing check,
  BEFORE the `upsert_from_node` call that 106 added. The guard reads the CURRENT shared_task status (the
  "from") and the incoming status (the "to").
- **Before:** (106's apply: it resolves context, maps status via 302, then calls
  `SharedTaskRepository::upsert_from_node(...)`. P2 has, by this point, added an `assignment` lookup +
  stale-token reject around this op. Copy the EXACT current text of the (c)/(d) block as it stands after
  106 + 302 + P2 land.)
- **After:** insert the transition-author guard between "incoming status known" and "upsert". EXACT contract:
  1. **Determine the current (`from`) status.** Look up the existing shared task for this op
     (`SharedTaskRepository::find_by_id` on the resolved shared_task id, OR the source-unique lookup 106
     already resolves; reuse whatever 106/P2 already fetched — do NOT add a second query if the row is in
     hand). If NO shared_task exists yet (first sight), there is no `from` → treat as the creation path:
     skip the transition guard (creation is not a transition; the matrix governs transitions of an
     existing row). Record this in a comment.
  2. **No-op short-circuit:** if `incoming == current`, it is NOT a transition — proceed to `upsert_from_node`
     unchanged (metadata-only update). Do NOT reject.
  3. **Author check** (the SC4 core): `let author = status_machine::author_of_transition(current, incoming);`
     - `author == None` → **illegal transition** → REJECT this op's status change: do NOT call
       `upsert_from_node` for it; log `warn!` and SKIP+advance the cursor for this op (an illegal
       node-reported status must not wedge the op-log — treat like 106's permanent-skip: record in
       `node_op_log`, advance `applied_through_seq`, `continue`). Surface the rejection (warn with
       from/to/node_id).
     - `author == Some(Hive)` → a HIVE-authored transition arriving FROM A NODE → **REJECT** (same
       SKIP+advance as illegal): a node may not author `todo→in-progress`, `*→cancelled`. Log `warn!`.
     - `author == Some(Node)` → node-reported and node-authorable. **It is accepted ONLY with a valid
       lease + current fencing token (CONTRACT §C / P2):** require that (i) an ACTIVE assignment exists
       for this task on this node (`TaskAssignmentRepository::find_active_for_task` returns `Some`, lease
       not expired) AND (ii) P2's fencing check passed for this op (op.fencing_token == assignment's
       current token). **Reuse P2's existing assignment+token decision** — if P2 already computed an
       `is_fenced_ok`/`assignment` binding above, gate on it; do NOT duplicate the stale-token comparison.
       If there is no valid lease+token, REJECT (SKIP+advance, warn). Only when lease+token are valid AND
       the transition is node-authored does the op proceed to `upsert_from_node`.
  > Net: `upsert_from_node` is reached for an existing-row op ONLY when the transition is a no-op, OR is
  > node-authored with a valid lease+token. Hive-authored-from-node and illegal transitions are rejected
  > (SKIP+advance), never merged. This removes the field-level status conflict at the source (SC4).
- Add `use super::status_machine;` (or `use crate::nodes::ws::status_machine;`) at the top of `session.rs`
  if not already present from 302.

## Allowed moves
ONLY: add the `use` for `status_machine`, insert the transition-author guard described above into the
existing `handle_op_batch` apply path, and add the `#[cfg(test)] mod status_guard_tests`. REUSE
`status_machine::author_of_transition`/`node_may_author` (301), `canonical_status_from_node` (302),
`SharedTaskRepository`, `TaskAssignmentRepository`, and P2's existing lease/fencing decision — do NOT
re-implement any of them. Do NOT touch the WS enum definitions, the node crate, `tasks.rs`, `status_machine.rs`,
any migration, or 106's park/skip context-resolution branches. Do NOT re-implement P2's stale-token check.

## STOP triggers
- **P2 not landed:** `node_task_assignments` has no `fencing_token`/`lease_expires_at` column, OR
  `handle_op_batch` has no assignment lookup / stale-token reject (CONTRACT §C). → STOP. 303 GATES on P2;
  it cannot be authored against a `handle_op_batch` that has no fencing seam. (P2's fencing seam is tasks
  201/203/205; it is a prose+STOP precondition, not a `depends_on` edge — the documented phase-by-phase
  dependency. Verify P2 has landed before executing.)
- **106 not landed:** `handle_op_batch` / its (c)/(d) apply block is absent → STOP (303 edits inside it).
- **Treating an illegal/wrong-author transition as PARK (break, no advance)** → BUG: that wedges the
  op-log on the first rejected status (like 106's permanent-skip wedge). Rejected status transitions
  SKIP + ADVANCE (record in `node_op_log`, advance the cursor, `continue`) — they do NOT park. PARK is
  106's TRANSIENT-only case. Verify against 106's park-vs-skip split.
- **Gating creation (no existing shared_task) as a transition** → BUG: the matrix governs transitions of
  an EXISTING row; a first-sight op has no `from`. Skip the guard on creation (let 106's upsert create it).
- **Double-counting the cursor / double-writing `node_op_log`:** the rejected-transition SKIP must use the
  SAME single `node_op_log` write 106 uses for its permanent-skip branch — an op is recorded at most once.
- `query!`/`query_as!` fail offline → export `DATABASE_URL=postgres://…` (Trap 2b). Do NOT `cargo sqlx prepare`.
- The unit-test cannot cheaply construct a `ws_sender` for `handle_op_batch`: if 106 already extracted a
  send-free apply fn (its STOP note offered `handle_op_batch_apply(pool, …) -> i64`), test THAT directly.
  If not, extract the per-op apply into a testable fn and record the split (mirrors 106's note). Do NOT
  change the public handler signature.

## Done when
`WAI_TYPECHECK_CMD="cargo check -p remote" WAI_TEST_CMD='test -n "$DATABASE_URL" && cargo test -p remote status_guard' bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-hive-redesign 303` exits 0
(run with `DATABASE_URL=postgres://…` pointed at a migrated Postgres — Trap 2b. **The `test -n "$DATABASE_URL" &&`
prefix makes the gate FAIL-CLOSED** (tournament R1/F2): without `DATABASE_URL` the gate FAILS instead of
`skip_without_db!` reporting a hollow green.)
