---
id: "106"
phase: 1
title: Hive handle_op_batch — idempotent apply + durable OpAck
status: ready
depends_on: ["102", "103"]
parallel: false
conflicts_with: ["103"]
files:
  - crates/remote/src/nodes/ws/session.rs
irreversible: false
scope_test: "crates/remote/src/nodes/ws/session.rs"
allowed_change: edit
covers_criteria: [SC2]
covers_tests: [TS1]
---
## Failing test (write first)
**PRECONDITION (Trap 2b — NON-NEGOTIABLE):** REQUIRES a live, migrated Postgres (`node_op_log` from
102 + `swarm_projects`/`node_local_projects`/`shared_tasks`). A run without `DATABASE_URL` returns early
(skip) = HOLLOW pass. Stand up Postgres, `sqlx::migrate!("./migrations")`, export `DATABASE_URL=postgres://…`
before the gate runs, or RAISE that CI Postgres is unavailable.

**The test MUST be a `#[cfg(test)] mod` INSIDE `session.rs`, NOT a `tests/` file.** `handle_op_batch` is
private to the module; an integration test in `crates/remote/tests/` is a separate crate and sees only
`pub` items, so it could not call the handler (and `pub`-exposing an internal WS apply fn is the wrong
contract). A unit `mod tests` in `session.rs` sees the private fn and builds a `PgPool` from
`DATABASE_URL` itself. **Sibling read (rubric #9):** `crates/remote/tests/backfill_e2e.rs` — copy its
exact helpers verbatim into the test module: `fn database_url() -> Option<String>` (reads `DATABASE_URL`),
the `skip_without_db!` macro, `async fn create_pool() -> PgPool` (`PgPool::connect(&url)`), and its
`create_test_organization`/`create_test_node` fixture style. There is NO shared `common` module (the
`remote/tests/` dir holds only `backfill_e2e.rs` + `pool_config.rs`) — inline the helpers.

Add to `session.rs`:
```rust
#[cfg(test)]
mod op_batch_tests {
    use super::*;

    fn database_url() -> Option<String> { std::env::var("DATABASE_URL").ok() }
    macro_rules! skip_without_db { () => {
        if database_url().is_none() { eprintln!("Skipping: DATABASE_URL not set"); return; }
    }; }
    async fn create_pool() -> sqlx::PgPool {
        sqlx::PgPool::connect(&database_url().unwrap()).await.expect("connect")
    }
    // … inline create_test_organization / create_test_node / seed a swarm_project + node_local_projects
    //    link exactly as backfill_e2e.rs seeds them.

    #[tokio::test]
    async fn op_batch_applies_swarm_linked_task_idempotently_and_acks() {
        skip_without_db!();
        let pool = create_pool().await;
        // seed org, node, swarm_project, node_local_projects(node_id, local_project_id, swarm_project_id).
        // op = OutboxOp { seq:1, op_type:"task.upsert", entity_type:"task", entity_id: local_task_id,
        //   payload: serde_json::json!({"id": local_task_id, "project_id": local_project_id,
        //   "title":"t", "description": null, "status":"done"}), idempotency_key:"task:..:..", fencing_token:None }
        // Apply twice (build a no-op ack sink — see STOP note on ws_sender in test):
        //   1st: node_op_log gets exactly one row; shared_tasks has the task; applied_through_seq == 1.
        //   2nd: node_op_log still ONE row (ON CONFLICT DO NOTHING); no error; applied_through_seq == 1.
    }

    #[tokio::test]
    async fn op_batch_PARKS_when_local_project_link_absent() {
        skip_without_db!();
        let pool = create_pool().await;
        // seed org+node but NO node_local_projects row → TRANSIENT (ProjectsSync race).
        // op seq:1 for that missing project. Apply: applied_through_seq does NOT advance to 1
        // (stays at high-water 0), and node_op_log has NO row for the key → the node re-sends it.
    }

    #[tokio::test]
    async fn op_batch_SKIPS_AND_ADVANCES_when_project_present_but_not_swarm_linked() {
        skip_without_db!();
        let pool = create_pool().await;
        // seed node_local_projects row WITHOUT swarm_project_id (or no swarm_project_nodes link) →
        // PERMANENT (project intentionally not swarm-linked). Apply: applied_through_seq DOES advance
        // to 1 (the op is acked/skipped, NOT parked) and node_op_log records it — so a non-swarm task
        // at the outbox head does NOT wedge the cursor forever.
    }

    #[tokio::test]
    async fn op_batch_maps_node_lowercase_status_explicitly() {
        skip_without_db!();
        let pool = create_pool().await;
        // seed org+node+swarm-linked project. Apply an op whose payload was produced by the node:
        //   payload.status = "inprogress"  (node TaskStatus #[serde(rename_all="lowercase")])
        // Assert the resulting shared_tasks row status == 'in-progress' (the hive's InProgress), NOT the
        // wrong fallback status. Repeat with "inreview" -> 'in-review'. This is the tournament R1/F5
        // guard: the default handle_task_sync parse would silently coerce the node's lowercase forms to
        // the wrong fallback (the InProgress/InReview hyphenation mismatch).
    }

    #[tokio::test]
    async fn op_batch_does_not_lose_apply_when_upsert_fails_then_retried() {
        skip_without_db!();
        let pool = create_pool().await;
        // tournament R1/F1: prove apply-then-record. Drive an op whose first upsert_from_node fails
        // (e.g. a transiently-violating fixture), assert NO node_op_log row was written and NO ack
        // advanced; then on the clean retry assert the shared_task IS applied and the dedup row appears.
        // (If injecting an upsert failure is impractical at this seam, assert the weaker invariant: a
        // node_op_log row exists ONLY for ops whose shared_tasks apply is present — never a dedup row
        // without its task.)
    }
}
```
> The third test is the wedge guard. Without the park-vs-skip split (below), a single non-swarm task at
> the outbox head would park permanently and the op-log would never advance for that node.

## Change
- **File:** `crates/remote/src/nodes/ws/session.rs`
- **Anchor:** the `NodeMessage::OpBatch { ops }` STUB arm added by 103 in `handle_node_message`
  (@~580-585 after 103 lands), plus a NEW `handle_op_batch` fn placed beside `handle_task_sync` (@1547).
- **Sibling read (rubric #9):** `handle_task_sync` (@1547-1727) is the apply+reply template AND the
  authority on park-vs-skip. It resolves context in three steps that ALREADY distinguish transient from
  permanent — copy that exactly:
  1. `NodeLocalProjectRepository::find_by_node_and_project` returns `None` (@1577-1599) → **TRANSIENT**
     (ProjectsSync race): `handle_task_sync` replies "RETRY". For the op-log → **PARK** (break, no advance).
  2. row present but `local_project.swarm_project_id` is `None` (@1603-1626) → **PERMANENT** (not linked):
     `handle_task_sync` replies "link via UI". For the op-log → **SKIP + ADVANCE** (treat like the
     `op_type != "task.upsert"` guard — ack it; do NOT wedge the cursor).
  3. `swarm_project_nodes`/`swarm_projects` org lookup returns `None` (@1629-1667) → **PERMANENT** (bad
     link): also **SKIP + ADVANCE**.
  Ack reuses the `HeartbeatAck` send shape (@604-612).
- **Before (the 103 stub arm):**
```rust
        NodeMessage::OpBatch { ops } => {
            // STUB — filled by task 106 (idempotent apply + durable OpAck). Logs so the exhaustive
            // match compiles now; 106 replaces the body with handle_op_batch(...).
            tracing::debug!(node_id = %node_id, op_count = ops.len(), "received op_batch (apply TODO: task 106)");
            Ok(())
        }
```
- **After:**
```rust
        NodeMessage::OpBatch { ops } => {
            handle_op_batch(node_id, organization_id, node_name, ops, pool, ws_sender).await
        }
```
- **Add `handle_op_batch`** (new fn beside `handle_task_sync`). EXACT contract:
  - Signature mirrors `handle_task_sync` PLUS the `ops` slice. **`ops` is borrowed, NOT owned**
    (tournament R1/F3): `handle_node_message(msg: &NodeMessage, …)` (`session.rs:501`) matches on a
    `&NodeMessage`, so the `NodeMessage::OpBatch { ops }` arm binds `ops: &Vec<OutboxOp>` — it CANNOT be
    moved into an owned `Vec` param. Take a slice:
    `async fn handle_op_batch(node_id: Uuid, organization_id: Uuid, node_name: &str, ops: &[OutboxOp],
    pool: &PgPool, ws_sender: &mut SplitSink<WebSocket, Message>) -> Result<(), HandleError>`
    (`OutboxOp` = `crate::nodes::ws::message::OutboxOp` from 103). The call arm is
    `handle_op_batch(node_id, organization_id, node_name, ops, pool, ws_sender).await` (the `&Vec` coerces
    to `&[OutboxOp]`). Iterate `for op in ops` (yields `&OutboxOp`; `op.seq` is `Copy`, use `&op.idempotency_key`/`&op.op_type` for binds).
  - `applied_through_seq` starts at the node's current high-water:
    `SELECT COALESCE(MAX(seq),0) FROM node_op_log WHERE node_id = $1`.
  - Iterate `ops` IN ORDER. For each op:
    - **(a) Tracer scope guard:** `op.op_type != "task.upsert"` → SKIP + `applied_through_seq = op.seq`
      (later increments add other op types). Continue.
    - **(b) Resolve context (copy `handle_task_sync` @1569-1667):**
      - `find_by_node_and_project(pool, node_id, payload.project_id)` is `None` → **PARK**: `break` (do
        NOT advance, do NOT insert node_op_log). TRANSIENT — re-sent after ProjectsSync. Log debug.
      - row present but `swarm_project_id` is `None`, OR the swarm-link/org query is `None` → **SKIP +
        ADVANCE**: `applied_through_seq = op.seq`; record it in node_op_log (so the high-water reflects
        it); do NOT call `upsert_from_node`. PERMANENT (not swarm-linked) — must NOT wedge the cursor.
        Continue.
    - **(c) Idempotent apply — APPLY FIRST, RECORD SECOND (tournament R1/F1):** the dedup row must NOT be
      persisted independently of a successful apply. If `INSERT node_op_log` commits but `upsert_from_node`
      then fails, a retry sees the dedup row (`rows_affected==0`), SKIPS the apply, advances the ack →
      **silent loss**. So order it:
      1. `let seen: bool = SELECT EXISTS(SELECT 1 FROM node_op_log WHERE node_id=$1 AND idempotency_key=$2)`.
      2. `seen == true` → already applied in a prior committed pass: SKIP the upsert; `applied_through_seq =
         op.seq`; continue.
      3. `seen == false` → `SharedTaskRepository::upsert_from_node(UpsertTaskFromNodeData { swarm_project_id,
         project_id: swarm_project_id, organization_id: org_id, origin_node_id: node_id, local_task_id:
         payload.id, title, description, status: <mapped per (d)>, version: 1, owner_node_id: Some(node_id),
         owner_name: Some(node_name.to_string()), assignee_user_id: None })` (construction as
         `handle_task_sync` @1675). `upsert_from_node` is itself idempotent (`ON CONFLICT (source_node_id,
         source_task_id) DO UPDATE`, `tasks.rs:585`). **ONLY AFTER it returns `Ok`** →
         `INSERT INTO node_op_log (..) ON CONFLICT (node_id, idempotency_key) DO NOTHING`; then
         `applied_through_seq = op.seq`.
      If `upsert_from_node` returns `Err`, propagate (`?`) → **no dedup row, no ack** → the node re-sends and
      re-applies; no silent loss. (Apply-then-record avoids threading a transaction through
      `upsert_from_node`, which lives in `tasks.rs` outside this task's `files:`.)
    - **(d) Status value mapping (tournament R1/F5):** the op payload is `serde_json::to_value(&Task)` (105)
      and the node `TaskStatus` serializes `#[serde(rename_all="lowercase")]` → `todo`/`inprogress`/
      `inreview`/`done`/`cancelled` (`crates/db/src/models/task/mod.rs:24`). The hive's `handle_task_sync`
      status parse accepts `in_progress`/`in-progress` and DEFAULTS unknown → `Todo` (`session.rs:1559`),
      which would silently corrupt `inprogress`/`inreview` → `Todo`. `handle_op_batch` MUST map the node's
      lowercase forms EXPLICITLY (`todo`→Todo, `inprogress`→InProgress, `inreview`→InReview, `done`→Done,
      `cancelled`→Cancelled) and on an UNKNOWN value return `Err`/log+skip — do NOT default-to-`Todo`.
  - After the loop: `send_message(ws_sender, &HiveMessage::OpAck { applied_through_seq }).await
    .map_err(|_| HandleError::Send)?;` (always send). Return `Ok(())`.
  > For the SKIP+ADVANCE permanent cases (b), recording the op in node_op_log keeps the cursor and the
  > dedup table consistent (a re-sent permanent-skip op stays deduped). The PARK case (transient) is the
  > ONLY one that does not advance.

## Allowed moves
ONLY: replace the 103 `OpBatch` stub body with the `handle_op_batch(...)` call, add the `handle_op_batch`
fn (apply + ack with the park-vs-skip split above), and add the `#[cfg(test)] mod op_batch_tests`. Reuse
`send_message`, `SharedTaskRepository`, `NodeLocalProjectRepository`, `UpsertTaskFromNodeData`, and the
swarm-link queries — do NOT re-implement them. Do NOT touch the WS enum definitions (103 owns them), the
node side, or any migration. Tracer scope: `task.upsert` ONLY.

## STOP triggers
- Treating "project present but not swarm-linked" as PARK (break) → BUG: that wedges the op-log on the
  first non-swarm task (105 enqueues for ALL projects, most non-swarm). PARK is for the TRANSIENT
  `node_local_projects`-row-absent case ONLY; the permanent not-linked cases SKIP + ADVANCE. This split
  is the core correctness of the task — verify against `handle_task_sync`'s three-branch resolution.
- `handle_op_batch` cannot reach `ws_sender` → it is a parameter threaded from `handle_node_message`
  (which has `ws_sender` @508). This NEW handler takes `ws_sender` from the start; do NOT change the
  signatures of existing `handle_attempt_sync`/`handle_execution_sync`/`handle_logs_batch`.
- The op payload field names differ from the node `Task` (105 serializes `serde_json::to_value(&Task)`):
  the node `Task` has `id: Uuid` and `project_id: Uuid`. Parse `payload.id` (local task id) and
  `payload.project_id` (local project id). If a name differs, align the parse — do NOT change 105.
- `query!`/`query_as!` fail offline → export `DATABASE_URL=postgres://…` against a migrated Postgres
  (Trap 2b). Do NOT `cargo sqlx prepare`.
- The unit-test `ws_sender` arg: `send_message` needs a `SplitSink<WebSocket, Message>`. If one cannot
  be cheaply constructed in the test, extract the apply loop into a `handle_op_batch_apply(pool, …) ->
  i64` (returns `applied_through_seq`, no send) that `handle_op_batch` calls then sends; test the
  apply fn directly and assert the returned seq. Record this split if taken (it keeps the test
  ws-free while preserving the ack path).

## Done when
`WAI_TYPECHECK_CMD="cargo check -p remote" WAI_TEST_CMD='test -n "$DATABASE_URL" && cargo test -p remote op_batch' bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-hive-redesign 106` exits 0
(run with `DATABASE_URL=postgres://…` pointed at a migrated Postgres — Trap 2b. **The `test -n "$DATABASE_URL" &&`
prefix makes the gate FAIL-CLOSED** (tournament R1/F2): `task-gate.sh` runs `WAI_TEST_CMD` via `bash -c`, so
without `DATABASE_URL` the gate fails instead of `skip_without_db!` reporting a hollow green.)
