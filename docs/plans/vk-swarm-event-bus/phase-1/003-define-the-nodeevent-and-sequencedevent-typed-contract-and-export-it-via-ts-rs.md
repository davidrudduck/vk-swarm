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
    fn event_type_matches_serde_tag() {
        // event_type() is what lands in the event_journal.event_type column; it MUST equal the
        // serde tag or cursor filtering by type silently misses rows.
        let e = NodeEvent::TaskCreated { task_id: uuid::Uuid::nil(), project_id: uuid::Uuid::nil() };
        let v = serde_json::to_value(&e).unwrap();
        assert_eq!(v["type"].as_str().unwrap(), e.event_type());
    }
}
```

`event_type_matches_serde_tag` is the load-bearing one: it pins the invariant that the stored
`event_type` column and the serde tag cannot drift apart.


## Change
**File:** `crates/db/src/models/event.rs`
**Anchor:** new file
**After:** define the typed contract. Derive `Debug, Clone, Serialize, Deserialize, TS`. Use
`#[serde(tag = "type", rename_all = "snake_case")]` on `NodeEvent` so the discriminant is a
`type` field, exactly as the spec's Design specifies.

Variants (fields per the spec's Design "Event schema"): `TaskCreated { task_id, project_id }`,
`TaskStatusChanged { task_id, old_status, new_status }`, `TaskDeleted { task_id, project_id }`,
`AttemptStarted { task_id, attempt_id, execution_process_id, executor }`,
`AttemptFinished { task_id, attempt_id, execution_process_id, exit_code }`,
`AttemptFailed { task_id, attempt_id, execution_process_id, reason }`,
`HiveConnected {}`, `HiveDisconnected { reason }`, `ReconcileCompleted { entity_count }`.

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
