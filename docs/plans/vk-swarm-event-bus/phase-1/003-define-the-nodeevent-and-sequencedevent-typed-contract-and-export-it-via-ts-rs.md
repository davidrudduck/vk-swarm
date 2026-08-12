---
id: "003"
phase: 1
title: "Define the NodeEvent and SequencedEvent typed contract and export it via ts-rs"
status: ready
depends_on: []
parallel: false
conflicts_with: []
files:
  - "crates/db/src/models/event.rs"
  - "crates/db/src/models/mod.rs"
  - "crates/server/src/bin/generate_types.rs"
  - "shared/types.ts"
irreversible: false
scope_test: "crates/db"
allowed_change: mixed
covers_criteria: []
covers_tests: []
---
## Failing test (write first)
**File:** `crates/db/src/models/event.rs` (colocated `#[cfg(test)] mod tests`)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_event_serializes_snake_case_tagged() {
        let e = NodeEvent::TaskStatusChanged {
            task_id: uuid::Uuid::nil(),
            old_status: "todo".into(),
            new_status: "inprogress".into(),
        };
        let v = serde_json::to_value(&e).unwrap();
        assert_eq!(v["type"], "task_status_changed");
        assert_eq!(v["new_status"], "inprogress");
    }

    #[test]
    fn node_event_round_trips() {
        let e = NodeEvent::HiveDisconnected { reason: "kill".into() };
        let s = serde_json::to_string(&e).unwrap();
        let back: NodeEvent = serde_json::from_str(&s).unwrap();
        assert_eq!(format!("{back:?}"), format!("{e:?}"));
    }

    #[test]
    fn event_type_matches_serde_tag_for_every_variant() {
        // event_type() is what lands in the event_journal.event_type column; it MUST equal the
        // serde tag or cursor filtering by type silently misses rows. Table-driven across ALL
        // NINE variants: checking one variant would let any of the other eight match arms drift
        // while this test stayed green.
        let nil = uuid::Uuid::nil();
        let all = vec![
            NodeEvent::TaskCreated { .. },
            NodeEvent::TaskStatusChanged { .. },
            NodeEvent::TaskDeleted { .. },
            NodeEvent::AttemptStarted { .. },
            NodeEvent::AttemptFinished { .. },
            NodeEvent::AttemptFailed { .. },
            NodeEvent::HiveConnected { .. },
            NodeEvent::HiveDisconnected { .. },
            NodeEvent::ReconcileCompleted { .. },
        ];
        assert_eq!(all.len(), 9, "a variant was added without extending this table");
        for e in &all {
            let v = serde_json::to_value(e).unwrap();
            assert_eq!(v["type"].as_str().unwrap(), e.event_type(), "{e:?}");
        }
    }

    #[test]
    fn terminal_attempt_events_carry_executor_identity() {
        // SC2 requires executor identity on the terminal outcome, not only on the start event.
        for e in [
            NodeEvent::AttemptFinished { /* .. */ executor: "claude".into(), .. },
            NodeEvent::AttemptFailed { /* .. */ executor: "claude".into(), .. },
        ] {
            let v = serde_json::to_value(&e).unwrap();
            assert_eq!(v["executor"], "claude");
        }
    }

    #[test]
    fn task_status_strings_use_serde_spelling() {
        // TaskStatus has TWO string forms: serde renames lowercase ("inprogress") while strum
        // Display is kebab-case ("in-progress") — see crates/db/src/models/task/mod.rs:21-33.
        // Emission sites must use the serde form or consumers and this contract silently disagree.
        let s = serde_json::to_value(TaskStatus::InProgress).unwrap();
        assert_eq!(s.as_str().unwrap(), "inprogress");
        assert_ne!(s.as_str().unwrap(), TaskStatus::InProgress.to_string());
    }
}
```

Fill in the elided fields — they are spelled out in the Change section. The first two tests are the
load-bearing ones: they pin that the stored `event_type` column cannot drift from the serde tag for
ANY variant, and that SC2's executor-identity clause is satisfied on terminal events.


## Change
**File:** `crates/db/src/models/event.rs`
**Anchor:** new file
**After:** define the typed contract. Derive `Debug, Clone, Serialize, Deserialize, TS`. Use
`#[serde(tag = "type", rename_all = "snake_case")]` on `NodeEvent` so the discriminant is a
`type` field, exactly as the spec's Design specifies.

Variants (fields per the spec's Design "Event schema"): `TaskCreated { task_id, project_id }`,
`TaskStatusChanged { task_id, old_status, new_status }`, `TaskDeleted { task_id, project_id }`,
`AttemptStarted { task_id, attempt_id, execution_process_id, executor }`,
`AttemptFinished { task_id, attempt_id, execution_process_id, executor, exit_code }`,
`AttemptFailed { task_id, attempt_id, execution_process_id, executor, reason }`,
`HiveConnected {}`, `HiveDisconnected { reason }`, `ReconcileCompleted { entity_count }`.

**`executor` on the terminal variants is required, not optional.** SC2 reads "Starting a task attempt
and its terminal outcome (finished or failed) each emit events carrying task id, attempt id, and
executor identity." The original schema carried `executor` only on `AttemptStarted`, which made SC2
unsatisfiable no matter what task 007 did — a field absent from the contract cannot be serialized.
Task 007 loads it from the owning `TaskAttempt` inside the same transaction as the completion write.

**Status string form:** `old_status` / `new_status` are the serde spelling of `TaskStatus`, NOT its
strum `Display` form. `crates/db/src/models/task/mod.rs:21-33` gives `TaskStatus` two different
string representations — serde `rename_all = "lowercase"` yields `inprogress`, strum yields
`in-progress`. Prefer typing the fields as `TaskStatus` directly so the compiler removes the choice;
if they must stay `String`, produce them by serializing `TaskStatus` through serde, never via
`to_string()`. This is pinned by `task_status_strings_use_serde_spelling`.

Also add:
```rust
impl NodeEvent {
    /// The value stored in `event_journal.event_type`. MUST equal the serde tag — pinned by
    /// `event_type_matches_serde_tag`.
    pub fn event_type(&self) -> &'static str { /* match self { ... } */ }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct SequencedEvent {
    pub seq: i64,
    pub event: NodeEvent,
}
```

**File:** `crates/db/src/models/mod.rs`
**Anchor:** the module declaration list
**Change:** add `pub mod event;` in alphabetical position.

**File:** `crates/server/src/bin/generate_types.rs`
**Anchor:** the `decls` vector inside `fn generate_types_content()` (starts L11) — follow the
existing `db::models::…::Decl::decl(),` pattern exactly.
**Change:** add two entries:
```rust
        db::models::event::NodeEvent::decl(),
        db::models::event::SequencedEvent::decl(),
```

**File:** `shared/types.ts`
**Change:** regenerate, do not hand-edit — `npm run generate-types` from the repo root.


## Allowed moves
ONLY the type definition, its module registration, the two typegen lines, and the
REGENERATED shared/types.ts. Do NOT hand-edit shared/types.ts (it carries a do-not-edit banner). Do
NOT add journal persistence or bus wiring — tasks 004/005.


## STOP triggers
- `NodeEvent` or `SequencedEvent` already exists anywhere (`git grep -n "enum NodeEvent"`) — they
  did not at decomposition time.
- `npm run generate-types` produces a diff in shared/types.ts beyond the two new types.
- The variant fields the spec names cannot be sourced at an emission site (e.g. no executor identity
  is reachable at attempt start) — STOP and escalate rather than inventing a field.


## Manual verification (record in decisions-ledger)
Gate invocation (the Done-when placeholders): this is a Rust crate, so the runner MUST be overridden — the auto-detected runner would try vitest. Use WAI_TYPECHECK_CMD="cargo check --workspace" with the WAI_TEST_CMD given below.
WAI_TEST_CMD="cargo test -p db event"

1. `cargo test -p db event` — the three tests above pass.
2. `npm run generate-types:check` — exits 0 (types current).
3. `grep -n "NodeEvent" shared/types.ts` — the generated union is present with snake_case tags.


## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-event-bus 003` exits 0
