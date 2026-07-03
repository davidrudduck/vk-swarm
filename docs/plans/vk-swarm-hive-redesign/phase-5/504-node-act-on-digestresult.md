---
id: "504"
phase: 5
title: Node acts on DigestResult — re-stream from resend_from_seq + pull via reconcile leg
status: ready
depends_on: ["501", "502"]
parallel: false
conflicts_with: ["501"]
files:
  - crates/db/src/models/node_outbox.rs
  - crates/services/src/services/hive_client.rs
  - crates/services/src/services/node_runner.rs
irreversible: false
scope_test: "crates/db/src/models/node_outbox.rs"
allowed_change: edit
covers_criteria: [SC5]
---
## Failing test (write first)
The load-bearing NEW behavior here is the re-stream read: `peek_from_seq` must return ops at/after a seq
**ignoring `acked_at`** (so an acked-but-lost op replays — the node-has/hive-lacks heal). `peek_unacked`
(104) filters `acked_at IS NULL` and therefore canNOT reach a lost-after-ack op; this is why a new read
is required. Hermetic (`create_test_pool()`):

In `crates/db/src/models/node_outbox.rs` `#[cfg(test)] mod tests`:
```rust
#[tokio::test]
async fn peek_from_seq_returns_acked_and_unacked_ops_at_or_after_seq() {
    let (pool, _tmp) = db::test_utils::create_test_pool().await;
    let mk = |k: &str| NewOutboxOp {
        op_type: "task.upsert".into(), entity_type: "task".into(),
        entity_id: uuid::Uuid::new_v4(), payload: serde_json::json!({}),
        idempotency_key: k.into(), fencing_token: None,
    };
    let a = OutboxRepository::enqueue_op(&pool, mk("task:a:1")).await.unwrap();
    let b = OutboxRepository::enqueue_op(&pool, mk("task:b:1")).await.unwrap();
    let c = OutboxRepository::enqueue_op(&pool, mk("task:c:1")).await.unwrap();

    // Ack through b → a and b are acked, c is unacked.
    OutboxRepository::mark_acked_through(&pool, b.seq).await.unwrap();
    assert_eq!(OutboxRepository::peek_unacked(&pool, 10).await.unwrap().len(), 1, "unacked sees only c");

    // Re-stream from a.seq IGNORES acked_at → replays a, b, c (the SC5 heal can reach acked-but-lost ops).
    let restream = OutboxRepository::peek_from_seq(&pool, a.seq, 10).await.unwrap();
    assert_eq!(restream.len(), 3, "peek_from_seq returns acked+unacked at/after seq");
    assert_eq!(restream[0].seq, a.seq);
    assert!(restream[2].seq == c.seq, "seq-ordered");
}
```
> 502's `find_digest_entries` test + 503's TS4 acceptance test cover the end-to-end self-heal; THIS task's
> own failing test is the `peek_from_seq` read contract (the one new persistence primitive). The
> HiveEvent/node_runner wiring is behavior-checked by the panel + compile (the `process_event` match is
> exhaustive — a missing arm fails to build, same as 108).

## Change

### 1. `crates/db/src/models/node_outbox.rs` — add `peek_from_seq` (re-stream, ignores acked_at)
- **File:** `crates/db/src/models/node_outbox.rs`
- **Anchor:** the `impl OutboxRepository` block (104) — `peek_unacked` is the sibling to mirror (same
  `OutboxOp` row mapping, `String`→`serde_json::Value` payload round-trip, BLOB-UUID casts). Add the new
  method beside it.
- **Before:** (no `peek_from_seq` method exists)
- **After:** add (NOTE: NO `acked_at IS NULL` filter — the whole point is to replay acked rows too):
```rust
    /// Re-stream ops for SC5 anti-entropy heal: all ops with `seq >= from_seq` in seq order, **including
    /// already-acked ones** (a node-has/hive-lacks divergence means an acked op was lost hive-side, so it
    /// MUST be replayable). Distinct from `peek_unacked`, which filters `acked_at IS NULL`. Safe to
    /// over-send: the hive apply is idempotent (`ON CONFLICT DO NOTHING`, 106) so re-streamed ops never
    /// double-apply or re-advance falsely.
    pub async fn peek_from_seq(
        pool: &SqlitePool,
        from_seq: i64,
        limit: i64,
    ) -> Result<Vec<OutboxOp>, sqlx::Error> {
        let rows = sqlx::query!(
            r#"SELECT id as "id!: Uuid", seq as "seq!: i64", op_type, entity_type,
                      entity_id as "entity_id!: Uuid", payload,
                      idempotency_key, fencing_token as "fencing_token: i64",
                      created_at as "created_at!: DateTime<Utc>",
                      acked_at as "acked_at: DateTime<Utc>"
               FROM node_outbox
               WHERE seq >= ?
               ORDER BY seq ASC
               LIMIT ?"#,
            from_seq,
            limit
        )
        .fetch_all(pool)
        .await?;
        rows.into_iter()
            .map(|r| {
                Ok(OutboxOp {
                    id: r.id,
                    seq: r.seq,
                    op_type: r.op_type,
                    entity_type: r.entity_type,
                    entity_id: r.entity_id,
                    payload: serde_json::from_str(&r.payload).unwrap_or(serde_json::Value::Null),
                    idempotency_key: r.idempotency_key,
                    fencing_token: r.fencing_token,
                    created_at: r.created_at,
                    acked_at: r.acked_at,
                })
            })
            .collect()
    }
```
> Match 104's EXACT select-cast + payload-parse style (read 104's `peek_unacked` and copy it, dropping
> only the `WHERE acked_at IS NULL` and changing it to `WHERE seq >= ?`). If 104 maps the payload via a
> fallible `serde_json::from_str(...)?` rather than `unwrap_or`, mirror THAT — do not diverge from the
> sibling's error handling.

### 2. `crates/services/src/services/hive_client.rs` — emit a HiveEvent on DigestResult
- **File:** `crates/services/src/services/hive_client.rs`
- **Anchor:** `enum HiveEvent` (@661; 108 added an `OpAck` variant after `Error { message }` @687), and
  the `HiveMessage::DigestResult { .. }` STUB arm that 501 added in `handle_hive_message` (before the
  `_ =>` wildcard @1062).
- **Before (HiveEvent tail — 108's `OpAck` variant + closing brace):**
```rust
    /// Durable op-log ack: all node_outbox ops with seq <= applied_through_seq are persisted (SC2c).
    OpAck { applied_through_seq: i64 },
}
```
- **After:**
```rust
    /// Durable op-log ack: all node_outbox ops with seq <= applied_through_seq are persisted (SC2c).
    OpAck { applied_through_seq: i64 },
    /// Anti-entropy heal directive (SC5): re-stream the op-log from `resend_from_seq` and/or pull the
    /// listed entities via the bulk-snapshot reconcile leg.
    DigestResult { resend_from_seq: Option<i64>, pull_entities: Vec<Uuid> },
}
```
- **Before (the 501 stub arm in `handle_hive_message`):**
```rust
            HiveMessage::DigestResult { resend_from_seq, pull_entities } => {
                // STUB — filled by task 504 (re-stream from resend_from_seq + pull listed entities).
                // For now log only so the arm is EXPLICIT (not swallowed by the `_ =>` wildcard below).
                tracing::debug!(
                    ?resend_from_seq,
                    pull_count = pull_entities.len(),
                    "received digest_result (heal TODO: task 504)"
                );
            }
```
- **After:**
```rust
            HiveMessage::DigestResult { resend_from_seq, pull_entities } => {
                tracing::trace!(?resend_from_seq, pull_count = pull_entities.len(), "received digest_result");
                // Handle the send outcome (do NOT discard via `let _ =`): a send error means the
                // consumer loop is gone (closed/dropped) — log at warn so the caller can reconnect /
                // retry the digest heal on the next cycle. Mirrors 108's `OpAck` emission handling.
                if let Err(e) = self
                    .event_tx
                    .send(HiveEvent::DigestResult { resend_from_seq, pull_entities })
                    .await
                {
                    tracing::warn!(
                        error = ?e,
                        "failed to forward DigestResult to node_runner loop — consumer gone; \
                         reconnect/retry will re-emit the digest next cycle"
                    );
                }
            }
```
  > `handle_hive_message` is `&self` and `HiveClient` holds NO pool/remote_client — so the heal CANNOT run
  > here. It emits a `HiveEvent::DigestResult` (mirroring 108's `OpAck`), and the pool+client-bearing
  > consumer (`run_node_runner`) performs the re-stream + reconcile. Same node-side seam as 108.

### 3. `crates/services/src/services/node_runner.rs` — handle the heal where pool+client live
- **File:** `crates/services/src/services/node_runner.rs`
- **Anchor:** the EXHAUSTIVE `process_event` match (@341-484; 108 added an `OpAck` arm), and the
  `run_node_runner` loop (108 added a `Some(HiveEvent::OpAck { .. })` arm before the `Some(_) => {}`
  catch-all @921). The loop owns `handle`, `db.pool`, `remote_client`, `command_tx`;
  `handle.state.node_id()`/`handle.state.organization_id()` (@293,298) yield the ids the reconcile leg needs.
- **Add to `process_event`** (state-only ack; the work is in the loop) — beside 108's `OpAck` arm:
```rust
            HiveEvent::DigestResult { resend_from_seq, pull_entities } => {
                tracing::trace!(?resend_from_seq, pull_count = pull_entities.len(), "digest_result received");
                // Heal (re-stream + reconcile) happens in run_node_runner where pool+remote_client live.
            }
```
- **Add a pool+client-bearing arm in the `run_node_runner` loop** — BEFORE the `Some(_) => {}` catch-all,
  beside 108's `OpAck` arm:
```rust
                Some(HiveEvent::DigestResult { resend_from_seq, pull_entities }) => {
                    // (a) node-has/hive-lacks: re-stream the op-log from the hive's conservative cursor.
                    if let Some(from_seq) = resend_from_seq {
                        match OutboxRepository::peek_from_seq(&db.pool, from_seq, RESTREAM_LIMIT).await {
                            Ok(rows) if !rows.is_empty() => {
                                let ops = rows.into_iter().map(restream_row_to_ws_op).collect();
                                if let Err(e) = command_tx
                                    .send(super::hive_client::NodeMessage::OpBatch { ops })
                                    .await
                                {
                                    tracing::warn!(error = ?e, "Failed to re-stream op-log for digest heal");
                                }
                            }
                            Ok(_) => {}
                            Err(e) => tracing::warn!(error = ?e, "Failed to read op-log for digest re-stream"),
                        }
                    }
                    // (b) hive-has/node-lacks: pull via the bulk-snapshot reconcile leg (ADR-0008 gap-fill).
                    if !pull_entities.is_empty()
                        && let (Some(ref client), Some(org_id), Some(nid)) = (
                            remote_client.as_ref(),
                            handle.state.organization_id().await,
                            handle.state.node_id().await,
                        )
                        && let Err(e) = sync_remote_projects(&db.pool, client, org_id, nid).await
                    {
                        tracing::warn!(error = ?e, "Failed to pull entities for digest heal");
                    }
                }
```
  - **Add `use db::models::node_outbox::OutboxRepository;`** at the call site if not already imported by
    108 (108 imports it inside `apply_op_ack`; if it is function-local there, add a use here or qualify
    the path `db::models::node_outbox::OutboxRepository`).
  - **Add a `const RESTREAM_LIMIT: i64 = 500;`** near the module constants (bounds one heal batch; a
    larger divergence heals over successive digests).
  - **Add a small `fn restream_row_to_ws_op(r: db::models::node_outbox::OutboxOp) ->
    super::hive_client::OutboxOp`** mapping the db row → the WS op (the SAME map 107 does inline in
    `sync_outbox`; factor it or inline a closure — do not change 107).
  > Re-streaming is safe to over-send: 106's apply is idempotent (`ON CONFLICT DO NOTHING`) and advances
  > the ack only on a successful apply, so replayed ops converge without double-effect. The pull leg
  > reuses the EXISTING `sync_remote_projects` (the reconcile gap-fill leg, node_runner.rs:941) verbatim —
  > do NOT write a new per-entity pull; `pull_entities` being non-empty is the trigger to reconcile.

## Allowed moves
ONLY: add `peek_from_seq` to `OutboxRepository`; add the `HiveEvent::DigestResult` variant; replace 501's
`DigestResult` stub body with the event emission; add the `process_event` arm; add the `run_node_runner`
heal arm + `RESTREAM_LIMIT` const + `restream_row_to_ws_op` helper. Reuse `sync_remote_projects` and
`command_tx` — do NOT re-implement the reconcile leg or the op-stream. Do NOT advance/clear the ack
cursor here (108 owns that; the re-streamed ops are re-acked by the hive normally). Do NOT touch the WS
enum (501), the hive side (503), or any migration.

## STOP triggers
- `peek_from_seq` accidentally filters `acked_at IS NULL` → BUG: it MUST replay acked rows (that is the
  node-has/hive-lacks heal; `peek_unacked` already exists for the unacked case). Verify the SQL has
  `WHERE seq >= ?` and NO `acked_at` predicate.
- Advancing/clearing the ack cursor in this task → BUG: re-stream does NOT touch `acked_at`; the cursor
  advances ONLY via 108's `apply_op_ack` on a fresh `OpAck`. This task only READS the outbox.
- `process_event`'s match has gained a `_` wildcard → still add the explicit `DigestResult` arm (keep it
  named; the loop arm does the work).
- `handle.state.node_id()`/`organization_id()` accessors are absent/renamed → they are at @293/@298; use
  them. If the loop cannot reach `handle` (moved), capture the ids from the `Connected` event into the
  loop scope instead — record the approach. Do NOT block the heal on missing ids; skip the pull leg and
  log if unavailable (the next digest retries).
- 501's `DigestResult` stub arm / 502's `find_digest_entries` are not present → 501 and 502 must be
  `passed` (depends_on: 501, 502). Without 501 the WS variant/`HiveEvent` won't compile.
- `sync_remote_projects` signature differs from `(pool, &RemoteClient, org_id, node_id)` → re-anchor
  (node_runner.rs:941); do NOT change its signature.

## Done when
`WAI_TYPECHECK_CMD="cargo check -p services" WAI_TEST_CMD="cargo test -p db peek_from_seq" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-hive-redesign 504` exits 0
(export `DATABASE_URL=sqlite://<repo>/dev_assets/db.sqlite` migrated through 101 before running — Trap 2;
the new `peek_from_seq` `query!` validates against the live SQLite schema. `cargo check -p services`
proves the exhaustive `process_event` match + the `run_node_runner` heal arm compile.)
