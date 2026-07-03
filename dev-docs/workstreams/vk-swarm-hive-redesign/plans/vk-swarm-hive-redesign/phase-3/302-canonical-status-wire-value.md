---
id: "302"
phase: 3
title: One canonical status wire value — extract node→hive mapping into a boundary helper
status: done
depends_on: ["301"]
parallel: false
conflicts_with: ["301", "303", "304"]
files:
  - crates/remote/src/nodes/ws/status_machine.rs
  - crates/remote/src/nodes/ws/session.rs
irreversible: false
scope_test: "crates/remote/src/nodes/ws/status_machine.rs"
allowed_change: edit
covers_criteria: [SC4]
covers_tests: []
---
## Failing test (write first)
**HERMETIC — NO Postgres.** The canonicalization fn is a pure `&str → Result<TaskStatus>` map; its test
is colocated in `status_machine.rs` with no `DATABASE_URL` precondition and NO fail-closed gate prefix.
(The DB-bound assertion that a `status:"inprogress"` op produces a `'in-progress'` shared_tasks row already
lives in 106's `op_batch_maps_node_lowercase_status_explicitly`; this task does NOT duplicate that — it
unit-tests the extracted boundary helper directly.)

Add to the `#[cfg(test)] mod tests` in `status_machine.rs`:

```rust
    #[test]
    fn canonicalizes_node_lowercase_status_to_hive_enum() {
        use crate::db::tasks::TaskStatus;
        // node TaskStatus serializes #[serde(rename_all="lowercase")] (db/.../task/mod.rs:25)
        // all five node wire forms canonicalize to their hive enum (representative subset below).
        assert_eq!(canonical_status_from_node("inprogress").unwrap(), TaskStatus::InProgress);
        assert_eq!(canonical_status_from_node("inreview").unwrap(), TaskStatus::InReview);
        assert_eq!(canonical_status_from_node("done").unwrap(), TaskStatus::Done);
        assert_eq!(canonical_status_from_node("cancelled").unwrap(), TaskStatus::Cancelled);
    }

    #[test]
    fn also_accepts_the_hive_hyphenated_forms() {
        use crate::db::tasks::TaskStatus;
        // the one canonical wire value is the hive hyphenated form; accept it idempotently so a
        // re-canonicalized value round-trips (CONTRACT §D "node and hive serialize identically").
        assert_eq!(canonical_status_from_node("in-progress").unwrap(), TaskStatus::InProgress);
        assert_eq!(canonical_status_from_node("in-review").unwrap(), TaskStatus::InReview);
    }

    #[test]
    fn rejects_unknown_status_returns_err_no_silent_default() {
        // tournament R1/F5: the legacy parse defaults an unknown value to the initial status (silent
        // corruption). The boundary helper MUST return Err on unknown, never a silent fallback.
        assert!(canonical_status_from_node("bogus").is_err());
        assert!(canonical_status_from_node("").is_err());
        assert!(canonical_status_from_node("IN_PROGRESS").is_err()); // case-sensitive: only the wire forms
    }
```

## Change
- **File:** `crates/remote/src/nodes/ws/status_machine.rs`
- **Anchor:** below the `node_may_author` fn added by 301.
- **Before:** (end of the 301 fns; no canonicalization helper exists)
- **After:** add ONE boundary helper that maps the node's wire status string to the canonical hive
  `TaskStatus`, accepting BOTH the node lowercase forms (`inprogress`/`inreview`) and the canonical hive
  hyphenated forms (`in-progress`/`in-review`), and returning `Err` on anything else (never default-to-Todo).
```rust
/// Map a node-reported `task.status` wire string to the canonical hive `TaskStatus`.
///
/// The node serializes its `TaskStatus` `#[serde(rename_all = "lowercase")]` →
/// `todo`/`inprogress`/`inreview`/`done`/`cancelled` (`crates/db/src/models/task/mod.rs:25`); the hive
/// enum is `kebab-case` → `in-progress`/`in-review`. This is the SINGLE boundary where the two
/// representations are reconciled (ADR-0010 "one canonical wire value", CONTRACT §D). Both forms are
/// accepted (so a re-canonicalized value is idempotent); an UNKNOWN value returns `Err` and is NEVER
/// coerced to `Todo` (tournament R1/F5 — the legacy default-to-`Todo` parse silently corrupts).
pub(crate) fn canonical_status_from_node(raw: &str) -> Result<TaskStatus, String> {
    match raw {
        "todo" => Ok(TaskStatus::Todo),
        "inprogress" | "in-progress" => Ok(TaskStatus::InProgress),
        "inreview" | "in-review" => Ok(TaskStatus::InReview),
        "done" => Ok(TaskStatus::Done),
        "cancelled" => Ok(TaskStatus::Cancelled),
        other => Err(format!("unknown node task.status wire value: {other:?}")),
    }
}
```
- **File:** `crates/remote/src/nodes/ws/session.rs`
- **Anchor:** the explicit node-lowercase status mapping inside `handle_op_batch` step (d) (added by 106 —
  the `match status_str { "todo"=>InProgress... }`-style inline map that 106's
  `op_batch_maps_node_lowercase_status_explicitly` test exercises). **106 inlined this map; this task
  REPLACES the inline map with a single call to the boundary helper** so there is exactly one mapping.
- **Before:** (106's inline status `match`/parse block in `handle_op_batch` step (d) — copy it EXACTLY as
  106 wrote it; e.g. a local `let status = match payload_status_str { "inprogress" => TaskStatus::InProgress, … };`)
- **After:** the same binding produced via the helper, propagating its `Err` exactly as 106's "unknown →
  Err/skip" already does:
```rust
        let status = crate::nodes::ws::status_machine::canonical_status_from_node(status_str)
            .map_err(HandleError::Validation)?; // or 106's existing unknown-status error/skip path
```
  Keep the surrounding control flow 106 established (the `?`/skip on unknown). If 106 used a different
  `HandleError` variant or a `continue`-skip for unknown, mirror THAT — do not change 106's error
  semantics, only the source of the mapping.

## Allowed moves
ONLY: add `canonical_status_from_node` to `status_machine.rs` (+ its colocated tests), and replace 106's
inline node-status `match` in `handle_op_batch` with a call to it. Do NOT change 106's surrounding apply
order, its park/skip branches, the WS enums, the node crate, `tasks.rs`, or any migration. Do NOT add the
transition-author guard here — that is 303.

## STOP triggers
- 106's inline status map is NOT where stated (handler refactored, or 106 not yet landed): STOP. This task
  depends on 106 having added the inline map in `handle_op_batch` step (d). If `handle_op_batch` or the
  inline `match` is absent, P1/106 has not landed — do not author the call site against thin air.
- The hive `TaskStatus` is not `kebab-case` (`in-progress`) as assumed (`tasks.rs:24`
  `#[sqlx(type_name="task_status", rename_all="kebab-case")]`): if the hive enum's serde/sqlx renaming
  changed, STOP and re-derive the accepted wire forms — the canonical value is whatever the hive enum
  actually serializes to.
- You consider making `canonical_status_from_node` accept `IN_PROGRESS`/uppercase/whitespace-trimmed
  forms "to be safe" → STOP. The contract is ONE canonical wire value; widening the accepted set
  re-introduces ambiguity. Accept only the node lowercase forms + the hive hyphenated forms; everything
  else is `Err`.
- You reach for `message.rs` or the node crate → STOP (P3 adds no WS variant; hive-only).

## Done when
`WAI_TYPECHECK_CMD="cargo check -p remote" WAI_TEST_CMD="cargo test -p remote status_machine" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-hive-redesign 302` exits 0
(the `status_machine` unit tests are hermetic; the `session.rs` call-site change is covered by 106's
existing Postgres-bound `op_batch_maps_node_lowercase_status_explicitly` — do NOT re-run that here).
