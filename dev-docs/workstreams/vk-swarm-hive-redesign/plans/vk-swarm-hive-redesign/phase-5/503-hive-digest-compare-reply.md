---
id: "503"
phase: 5
title: Hive handle_digest — compare against shared_tasks/node_op_log, reply DigestResult (TS4 self-heal)
status: ready
depends_on: ["501"]
parallel: false
conflicts_with: ["501"]
files:
  - crates/remote/src/db/tasks.rs
  - crates/remote/src/nodes/ws/session.rs
irreversible: false
scope_test: "crates/remote/src/nodes/ws/session.rs"
allowed_change: edit
covers_criteria: [SC5]
covers_tests: [TS4]
---
## Failing test (write first)
**PRECONDITION (Trap 2b — NON-NEGOTIABLE):** REQUIRES a live, migrated Postgres (`shared_tasks` +
`node_op_log` from 102 + `swarm_projects`/`node_local_projects`). A run without `DATABASE_URL` returns
early (skip) = HOLLOW pass. Stand up Postgres, `sqlx::migrate!("./migrations")`, export
`DATABASE_URL=postgres://…` before the gate runs, or RAISE that CI Postgres is unavailable.

**The test MUST be a `#[cfg(test)] mod` INSIDE `session.rs`, NOT a `tests/` file** — `handle_digest` is
private to the module (same constraint as 106's `handle_op_batch`). **Sibling read (rubric #9):** copy
106's `op_batch_tests` helper block verbatim — `database_url()`, the `skip_without_db!` macro,
`create_pool()`, and 106's `create_test_organization`/`create_test_node`/swarm-project seed fixtures.
There is NO shared `common` module — inline them.

**This is the TS4 self-heal acceptance test.** It seeds a BIDIRECTIONAL divergence and asserts the
`DigestResult` reply directs convergence with NO manual `reset_*`-style step (the heal is the protocol
path only: 503's reply → 504's re-stream/pull → existing reconcile leg; no out-of-band SQL):

```rust
#[cfg(test)]
mod digest_tests {
    use super::*;
    // … 106's inlined database_url() / skip_without_db! / create_pool() / fixtures …

    #[tokio::test]
    async fn digest_detects_bidirectional_divergence_and_replies_resend_and_pull() {
        skip_without_db!();
        let pool = create_pool().await;
        // Seed: org, node, swarm_project, node_local_projects link.
        // HIVE-HAS / NODE-LACKS: insert a shared_tasks row for this node with source_task_id = HID,
        //   source_node_id = node, version = 1, that the node's digest will NOT mention.
        // NODE-HAS / HIVE-LACKS: the node's digest WILL include entity_id = NID (a local task id) for
        //   which NO shared_tasks row exists (source_node_id=node, source_task_id=NID absent).
        // Seed node_op_log with a HIGH applied high-water (e.g. MAX(seq)=10) but the lost entity NID's op
        // was an OLDER seq (e.g. 3) that the hive never durably applied — the tournament R2/F4 case: a
        // replay from the high-water would MISS seq 3.

        let entries = vec![ DigestEntry { entity_type: "task".into(), entity_id: NID, version: 1 } ];
        // Drive handle_digest (or the ws-free apply split — see STOP note). Capture the DigestResult.
        let result = handle_digest_compare(&pool, node_id, &entries).await.unwrap();

        // NODE-HAS/HIVE-LACKS → the hive asks the node to re-stream from AT/BELOW the lost op's seq, NOT
        // from the high-water. With the conservative floor, resend_from_seq == Some(1) (<= the lost seq 3),
        // so the missing op is guaranteed to replay (R2/F4).
        assert_eq!(result.resend_from_seq, Some(1),
            "node-has/hive-lacks → re-stream from the floor (<= any lost op's seq), NOT MAX(seq)");
        // HIVE-HAS/NODE-LACKS → the hive lists HID for the node to PULL via the reconcile leg.
        assert!(result.pull_entities.contains(&HID), "hive-has/node-lacks → node pulls this entity");
        assert!(!result.pull_entities.contains(&NID), "an entity the node already has is NOT pulled");
    }

    #[tokio::test]
    async fn digest_in_sync_replies_empty() {
        skip_without_db!();
        let pool = create_pool().await;
        // Seed one shared_tasks row (source_task_id = SID) AND a digest entry for SID at the same version.
        let entries = vec![ DigestEntry { entity_type: "task".into(), entity_id: SID, version: 1 } ];
        let result = handle_digest_compare(&pool, node_id, &entries).await.unwrap();
        assert!(result.resend_from_seq.is_none() && result.pull_entities.is_empty(),
            "no divergence → no heal directives (idempotent: a converged sweep is a no-op)");
    }
}
```
> The "no `reset_*`" assertion is STRUCTURAL: convergence is produced purely by the `DigestResult` the
> handler returns (consumed by 504 + the reconcile leg), with no test-side SQL repair. Record in the
> ledger that the heal path contains zero manual migration/`reset_*` step (the SC5/TS4 contract).

## Change

### 1. `crates/remote/src/db/tasks.rs` — list the hive's source_task_ids for a node
- **File:** `crates/remote/src/db/tasks.rs`
- **Anchor:** the `impl SharedTaskRepository` block; `find_by_source_task_id` (@352) is the existing
  single-row bridge lookup. Add a NEW batch method beside it.
- **Sibling read (rubric #9):** `find_by_source_task_id` (@352-401) is the exact pattern — `query!`,
  `source_node_id`/`source_task_id`/`version` casts, `WHERE source_node_id = $1`. Mirror its cast style.
- **Before:** (no `list_source_task_versions_for_node` method exists)
- **After:** add (returns the hive's view of what it holds for this node — the id-bridge keys + versions,
  excluding soft-deleted rows so a tombstoned task is not re-pulled):
```rust
/// One (source_task_id, version) the hive holds for a node — the hive side of the SC5 digest compare.
#[derive(Debug, Clone)]
pub struct NodeSourceTaskVersion {
    pub source_task_id: Uuid,
    pub version: i64,
}

impl SharedTaskRepository {
    /// All non-deleted shared_tasks the hive holds for `source_node_id`, keyed by the id bridge
    /// (`source_task_id` = the node's local task id). Used by `handle_digest` to compute the
    /// hive-has/node-lacks set (SC5).
    pub async fn list_source_task_versions_for_node(
        &self,
        source_node_id: Uuid,
    ) -> Result<Vec<NodeSourceTaskVersion>, SharedTaskError> {
        let rows = sqlx::query!(
            r#"SELECT source_task_id AS "source_task_id!: Uuid", version AS "version!"
               FROM shared_tasks
               WHERE source_node_id = $1
                 AND source_task_id IS NOT NULL
                 AND deleted_at IS NULL"#,
            source_node_id
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| NodeSourceTaskVersion { source_task_id: r.source_task_id, version: r.version })
            .collect())
    }
}
```
> `deleted_at IS NULL` keeps a hive soft-delete (the one-delete tombstone, ADR-0007) out of the
> pull set — a tombstoned task must NOT be resurrected onto the node by anti-entropy.

### 2. `crates/remote/src/nodes/ws/session.rs` — replace the 501 stub with the compare + reply
- **File:** `crates/remote/src/nodes/ws/session.rs`
- **Anchor:** the `NodeMessage::Digest { entries }` STUB arm added by 501 in `handle_node_message`, plus a
  NEW `handle_digest` fn beside `handle_op_batch` (106) / `handle_task_sync` (@1547).
- **Sibling read (rubric #9):** 106's `handle_op_batch` is the template for the arm-replace + ws-reply
  shape (`send_message(ws_sender, &HiveMessage::…)`), and `handle_task_sync`'s `find_by_node_and_project`
  context resolution. `entries` binds as `&Vec<DigestEntry>` (the match is on `&NodeMessage` — borrow,
  do NOT move; take `&[DigestEntry]`, exactly the R1/F3 lesson from 106).
- **Before (the 501 stub arm):**
```rust
        NodeMessage::Digest { entries } => {
            // STUB — filled by task 503 (compare against shared_tasks + node_op_log, reply DigestResult).
            // Logs so the exhaustive match compiles now; 503 replaces the body with handle_digest(...).
            tracing::debug!(node_id = %node_id, entry_count = entries.len(), "received digest (compare TODO: task 503)");
            Ok(())
        }
```
- **After:**
```rust
        NodeMessage::Digest { entries } => {
            handle_digest(node_id, entries, pool, ws_sender).await
        }
```
- **Add `handle_digest`** (new fn beside `handle_op_batch`). EXACT contract:
  - Signature: `async fn handle_digest(node_id: Uuid, entries: &[DigestEntry], pool: &PgPool, ws_sender:
    &mut SplitSink<WebSocket, Message>) -> Result<(), HandleError>` (`DigestEntry` =
    `crate::nodes::ws::message::DigestEntry` from 501). For testability, factor the pure compare into
    `async fn handle_digest_compare(pool: &PgPool, node_id: Uuid, entries: &[DigestEntry]) ->
    Result<DigestResultParts, HandleError>` that `handle_digest` calls then sends (the tests call the
    compare fn directly, ws-free — same split 106 used for `ws_sender`; `DigestResultParts` is a tiny
    local `{ resend_from_seq: Option<i64>, pull_entities: Vec<Uuid> }` struct mapped into the WS variant).
  - **Compute the compare (tracer scope: entries with `entity_type == "task"` only; skip others):**
    1. `node_ids: HashSet<Uuid> = entries.iter().filter(|e| e.entity_type == "task").map(|e| e.entity_id).collect()`.
    2. `hive = SharedTaskRepository::new(pool.clone()).list_source_task_versions_for_node(node_id).await?`
       (construct the repo as `handle_task_sync` does); `hive_ids: HashSet<Uuid> = hive.iter().map(|h| h.source_task_id).collect()`.
    3. **hive-has / node-lacks → `pull_entities`:** `hive_ids.difference(&node_ids)` → the `source_task_id`s
       the hive holds but the node's digest omitted. Collect into `pull_entities: Vec<Uuid>` (these are
       the node's LOCAL ids = bridge keys; 504 maps them via the reconcile leg).
    4. **node-has / hive-lacks → `resend_from_seq`:** if `node_ids.difference(&hive_ids)` is NON-EMPTY
       (the node lists a task the hive has no shared_tasks row for → its op was lost), the hive must ask
       the node to re-stream from a point **AT OR BELOW the lost op's seq**. **Do NOT use `MAX(seq)`**
       (tournament R2/F4): a lost op at seq 3 while the hive's high-water is 10 would be MISSED by a
       replay from 10 → SC5 self-heal silently fails. Post-R1/F1 (apply-first/record-second) a `node_op_log`
       row implies the apply succeeded, so a lost entity has NO `node_op_log` row and the hive cannot derive
       its seq — therefore replay from the **conservative floor**: `resend_from_seq = Some(1)` (re-stream all
       retained ops; the node's `peek_from_seq` (504) includes acked-but-lost rows, and 106's
       `ON CONFLICT DO NOTHING` makes over-resending idempotent — safe, never lossy). *(Optimization for a
       later increment: carry a per-entity seq in the digest so the hive can pinpoint the replay point
       instead of flooring to 1 — out of tracer scope; CONTRACT §A `DigestEntry` has no seq today.)* If that
       difference is EMPTY, `resend_from_seq = None`.
    5. Return `DigestResultParts { resend_from_seq, pull_entities }`.
  - In `handle_digest`: `send_message(ws_sender, &HiveMessage::DigestResult { resend_from_seq,
    pull_entities }).await.map_err(|_| HandleError::Send)?;` (always reply, even when both are
    empty/None — an in-sync digest gets an empty DigestResult; 504 treats it as a no-op). Return `Ok(())`.
  > Heal-on-EXISTENCE, not version (advisor #3): `Task::update` bumps no local version, so `remote_version`
  > is noisy (0 pre-ack vs hive `version:1`); the TS4 signal is set-difference on `entity_id`, not a
  > version compare. `version` is carried for future drift use but does NOT drive resend/pull here.

## Allowed moves
ONLY: add `list_source_task_versions_for_node` + the `NodeSourceTaskVersion` projection to `tasks.rs`;
replace the 501 `Digest` stub body with the `handle_digest(...)` call; add `handle_digest` +
`handle_digest_compare` + the `DigestResultParts` local struct + the `#[cfg(test)] mod digest_tests`.
Reuse `send_message`, `SharedTaskRepository`, and the `node_op_log` `MAX(seq)` query shape — do NOT
re-implement them. Do NOT touch the WS enum definitions (501 owns them), the node side, the `OutboxOp`
apply path (106), or any migration. Tracer scope: `entity_type == "task"` ONLY.

## STOP triggers
- Treating a version difference as divergence → BUG (advisor #3): the heal signal is entity EXISTENCE
  (set-difference on `entity_id`), NOT version equality. `remote_version` is noisy. Do NOT resend/pull on
  a version mismatch alone in this tracer.
- `handle_digest` cannot reach `ws_sender` → it is a parameter threaded from `handle_node_message`
  (`ws_sender` @508), exactly like 106's `handle_op_batch`. Do NOT change existing handler signatures.
- The hive-has/node-lacks pull would resurrect a soft-deleted task → the `list_*` query MUST filter
  `deleted_at IS NULL` (ADR-0007 one-delete tombstone). Verify the predicate is present.
- `query!`/`query_as!` fail offline → export `DATABASE_URL=postgres://…` against a migrated Postgres
  (Trap 2b). Do NOT `cargo sqlx prepare`.
- The unit-test `ws_sender` arg cannot be cheaply constructed → use the `handle_digest_compare` split
  (returns `DigestResultParts`, no send); the tests call it directly. Record the split if taken.
- `node_op_log` (102) is absent in the PG under test → 102 (P1) must be applied; `MAX(seq)` needs the
  table. P5 rides P1; stand up the migrated PG including 102.

## Done when
`WAI_TYPECHECK_CMD="cargo check -p remote" WAI_TEST_CMD='test -n "$DATABASE_URL" && cargo test -p remote digest' bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-hive-redesign 503` exits 0
(run with `DATABASE_URL=postgres://…` pointed at a migrated Postgres — Trap 2b. **The `test -n "$DATABASE_URL" &&`
prefix makes the gate FAIL-CLOSED** (tournament R1/F2): `task-gate.sh` runs `WAI_TEST_CMD` via `bash -c`, so
without `DATABASE_URL` the gate fails instead of `skip_without_db!` reporting a hollow green.)
