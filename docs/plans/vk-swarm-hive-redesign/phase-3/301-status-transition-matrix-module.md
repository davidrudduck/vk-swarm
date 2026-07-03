---
id: "301"
phase: 3
title: Status transition matrix module — single-author guard table (ADR-0010 §D)
status: ready
depends_on: []
parallel: false
conflicts_with: ["302"]
files:
  - crates/remote/src/nodes/ws/status_machine.rs
  - crates/remote/src/nodes/ws/mod.rs
irreversible: false
scope_test: "crates/remote/src/nodes/ws/status_machine.rs"
allowed_change: mixed
covers_criteria: [SC4]
covers_tests: []
---
## Reconciled matrix (ADR-0010 §D, ratified 2026-06-30)
ADR-0010's status matrix was RECONCILED to the real `TaskStatus` enum and ratified. Both crates' enums
(node `task/mod.rs:27`, hive `tasks.rs:25`) are exactly `Todo/InProgress/InReview/Done/Cancelled` — there
is **no `Failed`, no `Assigned`** variant. SC4's parenthetical `assigned`/`failed` are authority labels
for lifecycle concepts that map to OTHER columns, not `task.status` values: `assigned` = an active
`node_task_assignments` row (hive, assignment layer, ADR-0009); `failed` = an `execution_status` outcome
(node, execution layer). The ratified `task.status` matrix encoded below:
- **Hive-authored:** `Todo→InProgress` (the on-assign-and-start transition), `InReview→Done` /
  `InReview→InProgress` (operator review), `*→Cancelled` (operator action).
- **Node-reported** (accepted only with a valid lease + current fencing token — 303 enforces):
  `InProgress→InReview`, `InProgress→Done`.

This matrix, 303's TS3 enforcement, and 304's legacy-path mapping are all encoded against these ratified
values. See `dev-docs/adr/0010-task-status-state-machine.md` (## Decision).

## Failing test (write first)
**HERMETIC — NO Postgres.** This task is a pure transition table (an enum + two fns over the hive
`TaskStatus`). It has NO DB, NO lease, NO fencing — those live in 303. Its test is a plain
`#[cfg(test)] mod tests` with no `DATABASE_URL` precondition and NO fail-closed gate prefix (contrast 303,
which IS Postgres-bound). Keeping the author-legality matrix out from behind a DB skip means it always runs.

The test lives INSIDE the new `status_machine.rs` (the fns are `pub(crate)`, the test is colocated):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::tasks::TaskStatus;

    // Every legal transition is accepted from its SOLE author; the other party is rejected.
    #[test]
    fn matrix_authors_each_transition_exactly_once() {
        // table: (from, to, expected_author)
        let cases: &[(TaskStatus, TaskStatus, TransitionAuthor)] = &[
            // hive-authored lifecycle (over the hive enum's actual variants — the reconciled matrix)
            (TaskStatus::Todo, TaskStatus::Cancelled, TransitionAuthor::Hive),
            (TaskStatus::InReview, TaskStatus::Done, TransitionAuthor::Hive), // operator approve
            (TaskStatus::InReview, TaskStatus::InProgress, TransitionAuthor::Hive), // operator reopen
            (TaskStatus::InProgress, TaskStatus::Cancelled, TransitionAuthor::Hive),
            (TaskStatus::InReview, TaskStatus::Cancelled, TransitionAuthor::Hive),
            (TaskStatus::Done, TaskStatus::Cancelled, TransitionAuthor::Hive),
            // node-reported (rideable only with a valid lease+token — 303 enforces that; the matrix
            // only declares the AUTHOR)
            (TaskStatus::InProgress, TaskStatus::Done, TransitionAuthor::Node),
            (TaskStatus::InProgress, TaskStatus::InReview, TransitionAuthor::Node),
        ];
        for (from, to, want) in cases {
            assert_eq!(
                author_of_transition(*from, *to),
                Some(*want),
                "author of {:?}->{:?}",
                from,
                to
            );
        }
    }

    #[test]
    fn illegal_transitions_have_no_author() {
        // A transition not in the matrix is rejected (no author) — illegal, never merged.
        assert_eq!(author_of_transition(TaskStatus::Done, TaskStatus::InProgress), None);
        assert_eq!(author_of_transition(TaskStatus::Cancelled, TaskStatus::InProgress), None);
        assert_eq!(author_of_transition(TaskStatus::Done, TaskStatus::InReview), None);
        // a no-op (from == to) is not an authored transition
        assert_eq!(author_of_transition(TaskStatus::Done, TaskStatus::Done), None);
    }

    #[test]
    fn node_may_author_only_node_transitions() {
        // The predicate 303 calls: may a NODE report author `from→to`?
        assert!(node_may_author(TaskStatus::InProgress, TaskStatus::Done));
        assert!(node_may_author(TaskStatus::InProgress, TaskStatus::InReview));
        // a node may NOT author a hive transition (the core SC4 rejection)
        assert!(!node_may_author(TaskStatus::InReview, TaskStatus::Done));
        assert!(!node_may_author(TaskStatus::InReview, TaskStatus::InProgress));
        assert!(!node_may_author(TaskStatus::InProgress, TaskStatus::Cancelled));
        // no-op / illegal are not node-authored
        assert!(!node_may_author(TaskStatus::Done, TaskStatus::Done));
        assert!(!node_may_author(TaskStatus::Done, TaskStatus::InProgress));
    }
}
```

## Change
- **File:** `crates/remote/src/nodes/ws/status_machine.rs` (NEW)
- **Anchor:** whole file (create).
- **Before:** (file does not exist)
- **After:** a self-contained module encoding ADR-0010 §D over the hive `TaskStatus`
  (`crate::db::tasks::TaskStatus` — `Todo/InProgress/InReview/Done/Cancelled`, `tasks.rs:25`). It declares
  WHO authors each transition; it does NOT touch the DB, lease, or fencing.
```rust
//! Explicit `task.status` transition matrix (ADR-0010 §D, CONTRACT §D).
//!
//! Single source of truth for WHICH party may author WHICH status transition. Every legal
//! transition has exactly ONE authoritative author, so there is no field-level status merge (SC4).
//! This module is pure: it encodes the matrix and answers author/legality questions. Enforcement at
//! the apply site (rejecting an illegal or wrong-author transition, and requiring a valid
//! lease + fencing token for node-reported transitions) lives in `session.rs::handle_op_batch`
//! (task 303), which rides P2's fencing check (CONTRACT §C).

use crate::db::tasks::TaskStatus;

/// The party authorized to author a given status transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TransitionAuthor {
    /// Hive-authored (operator / assignment lifecycle): `todo→in-progress` (the on-assign-and-start
    /// analog), `in-review→done` / `in-review→in-progress` (operator review), `*→cancelled`. (ADR-0010's
    /// reconciled matrix collapses the `assigned` concept into the `node_task_assignments` row, not a
    /// hive `TaskStatus` value — see the module/STOP note.)
    Hive,
    /// Node-reported up the outbox, accepted only with a valid lease + current fencing token (303):
    /// `in-progress→done`, `in-progress→in-review`.
    Node,
}

/// Return the sole authoritative author of `from → to`, or `None` if the transition is illegal
/// (not in the matrix). A no-op (`from == to`) returns `None` — it is not an authored transition.
pub(crate) fn author_of_transition(from: TaskStatus, to: TaskStatus) -> Option<TransitionAuthor> {
    use TaskStatus::*;
    match (from, to) {
        // hive-authored lifecycle
        (Todo, InProgress) => Some(TransitionAuthor::Hive), // assign + start
        (InReview, InProgress) => Some(TransitionAuthor::Hive), // operator reopen
        (InReview, Done) => Some(TransitionAuthor::Hive), // operator approve
        (_, Cancelled) if from != Cancelled => Some(TransitionAuthor::Hive),
        // node-reported work/terminal transitions
        (InProgress, Done) => Some(TransitionAuthor::Node),
        (InProgress, InReview) => Some(TransitionAuthor::Node),
        // everything else (incl. no-ops) is illegal
        _ => None,
    }
}

/// True iff a NODE report may author `from → to` (the predicate 303's enforcement calls before
/// applying a node-reported status). False for hive-authored transitions, illegal transitions, and
/// no-ops.
pub(crate) fn node_may_author(from: TaskStatus, to: TaskStatus) -> bool {
    matches!(author_of_transition(from, to), Some(TransitionAuthor::Node))
}
```
- **File:** `crates/remote/src/nodes/ws/mod.rs`
- **Anchor:** the module-declaration block (`ws/mod.rs:16-19`).
- **Before:**
```rust
mod connection;
mod dispatcher;
pub mod message;
mod session;
```
- **After:**
```rust
mod connection;
mod dispatcher;
pub mod message;
mod session;
mod status_machine;
```

## Allowed moves
ONLY: create `status_machine.rs` with the enum + the two fns + the colocated `#[cfg(test)] mod tests`,
and add the single `mod status_machine;` line to `ws/mod.rs`. Do NOT touch `session.rs`, the WS enum
definitions, the node crate, any migration, or `tasks.rs`. Do NOT add the enforcement/DB/lease logic
here — that is 303. The matrix is data only.

## STOP triggers
- The hive `TaskStatus` enum (`crate::db::tasks::TaskStatus`, `tasks.rs:25`) has **no `Assigned`
  variant** — `Todo/InProgress/InReview/Done/Cancelled` only. Per ADR-0010's reconciled (ratified) matrix,
  `assigned` is a NODE-side assignment concept (`node_task_assignments`), not a hive `task.status` value,
  and `failed` is an `execution_status` outcome. Encode the matrix over the hive enum's ACTUAL variants:
  hive authors `Todo→InProgress` (assign+start), `InReview→Done` / `InReview→InProgress` (operator
  review), and `*→Cancelled`; node authors `InProgress→Done` and `InProgress→InReview`. If you find an
  `Assigned` hive `TaskStatus` variant after all, STOP — the enum changed under this plan and the matrix
  must be re-derived against it.
- `cargo check -p remote` fails on an unused-fn warning (`-D warnings`): `author_of_transition` /
  `node_may_author` / `TransitionAuthor` are consumed by 303, not by this task's non-test build. The
  colocated tests reference all three, so they are NOT dead in the test build. If the non-test build warns
  "never used", record it and add `#[cfg_attr(not(test), allow(dead_code))]` (or `#[allow(dead_code)]`)
  until 303 wires the call site. Do NOT delete them to silence the warning.
- You reach for `message.rs`, the node `hive_client.rs`, or any WS variant → STOP. P3 adds NO WS variant
  (CONTRACT §A: status rides the P1 op-log; Trap 3 does NOT apply to P3). This task is hive-only.

## Done when
`WAI_TYPECHECK_CMD="cargo check -p remote" WAI_TEST_CMD="cargo test -p remote status_machine" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-hive-redesign 301` exits 0
(no Postgres needed — the matrix tests are hermetic; do NOT add a `DATABASE_URL` precondition here).
