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
#[allow(dead_code)]
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
#[allow(dead_code)]
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
#[allow(dead_code)]
pub(crate) fn node_may_author(from: TaskStatus, to: TaskStatus) -> bool {
    matches!(author_of_transition(from, to), Some(TransitionAuthor::Node))
}

/// Map a node-reported `task.status` wire string to the canonical hive `TaskStatus`.
///
/// The node serializes its `TaskStatus` `#[serde(rename_all = "lowercase")]` →
/// `todo`/`inprogress`/`inreview`/`done`/`cancelled` (`crates/db/src/models/task/mod.rs:25`); the hive
/// enum is `kebab-case` → `in-progress`/`in-review`. This is the SINGLE boundary where the two
/// representations are reconciled (ADR-0010 "one canonical wire value", CONTRACT §D). Both forms are
/// accepted (so a re-canonicalized value is idempotent); an UNKNOWN value returns `Err` and is NEVER
/// coerced to `Todo` (tournament R1/F5 — the legacy default-to-`Todo` parse silently corrupts).
#[allow(dead_code)]
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
            (TaskStatus::Todo, TaskStatus::InProgress, TransitionAuthor::Hive), // assign + start
            (TaskStatus::InReview, TaskStatus::Done, TransitionAuthor::Hive), // operator approve
            (TaskStatus::InReview, TaskStatus::InProgress, TransitionAuthor::Hive), // operator reopen
            (TaskStatus::Todo, TaskStatus::Cancelled, TransitionAuthor::Hive),
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
}
