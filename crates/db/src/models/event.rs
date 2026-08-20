//! Event contract for node event bus.
//!
//! Defines the typed schema for events emitted by a node, including task lifecycle events,
//! execution process events, and hive synchronization events.

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use super::task::TaskStatus;

/// Events emitted by the node event bus.
///
/// Each variant represents a specific event type. The serde tag "type" field must match
/// the value returned by `event_type()` for cursor filtering to work correctly.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NodeEvent {
    /// A new task was created.
    TaskCreated { task_id: Uuid, project_id: Uuid },
    /// A task's status changed.
    TaskStatusChanged {
        task_id: Uuid,
        old_status: TaskStatus,
        new_status: TaskStatus,
    },
    /// A task was deleted.
    TaskDeleted { task_id: Uuid, project_id: Uuid },
    /// An attempt started execution.
    AttemptStarted {
        task_id: Uuid,
        attempt_id: Uuid,
        execution_process_id: Uuid,
        executor: String,
    },
    /// An attempt finished successfully.
    AttemptFinished {
        task_id: Uuid,
        attempt_id: Uuid,
        execution_process_id: Uuid,
        executor: String,
        exit_code: i64,
    },
    /// An attempt failed.
    AttemptFailed {
        task_id: Uuid,
        attempt_id: Uuid,
        execution_process_id: Uuid,
        executor: String,
        reason: String,
    },
    /// Connected to the hive server.
    HiveConnected {},
    /// Disconnected from the hive server.
    HiveDisconnected { reason: String },
    /// Reconciliation completed.
    ReconcileCompleted { entity_count: i64 },
}

impl NodeEvent {
    /// The value stored in `event_journal.event_type`. MUST equal the serde tag — pinned by
    /// `event_type_matches_serde_tag`.
    pub fn event_type(&self) -> &'static str {
        match self {
            NodeEvent::TaskCreated { .. } => "task_created",
            NodeEvent::TaskStatusChanged { .. } => "task_status_changed",
            NodeEvent::TaskDeleted { .. } => "task_deleted",
            NodeEvent::AttemptStarted { .. } => "attempt_started",
            NodeEvent::AttemptFinished { .. } => "attempt_finished",
            NodeEvent::AttemptFailed { .. } => "attempt_failed",
            NodeEvent::HiveConnected { .. } => "hive_connected",
            NodeEvent::HiveDisconnected { .. } => "hive_disconnected",
            NodeEvent::ReconcileCompleted { .. } => "reconcile_completed",
        }
    }
}

/// An event with its sequence number in the event journal.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct SequencedEvent {
    pub seq: i64,
    pub event: NodeEvent,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_event_serializes_snake_case_tagged() {
        let e = NodeEvent::TaskStatusChanged {
            task_id: uuid::Uuid::nil(),
            old_status: TaskStatus::Todo,
            new_status: TaskStatus::InProgress,
        };
        let v = serde_json::to_value(&e).unwrap();
        assert_eq!(v["type"], "task_status_changed");
        assert_eq!(v["new_status"], "inprogress");
    }

    #[test]
    fn node_event_round_trips() {
        let e = NodeEvent::HiveDisconnected {
            reason: "kill".into(),
        };
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
            NodeEvent::TaskCreated {
                task_id: nil,
                project_id: nil,
            },
            NodeEvent::TaskStatusChanged {
                task_id: nil,
                old_status: TaskStatus::Todo,
                new_status: TaskStatus::Todo,
            },
            NodeEvent::TaskDeleted {
                task_id: nil,
                project_id: nil,
            },
            NodeEvent::AttemptStarted {
                task_id: nil,
                attempt_id: nil,
                execution_process_id: nil,
                executor: "test".into(),
            },
            NodeEvent::AttemptFinished {
                task_id: nil,
                attempt_id: nil,
                execution_process_id: nil,
                executor: "test".into(),
                exit_code: 0,
            },
            NodeEvent::AttemptFailed {
                task_id: nil,
                attempt_id: nil,
                execution_process_id: nil,
                executor: "test".into(),
                reason: "test".into(),
            },
            NodeEvent::HiveConnected {},
            NodeEvent::HiveDisconnected {
                reason: "test".into(),
            },
            NodeEvent::ReconcileCompleted { entity_count: 0 },
        ];
        assert_eq!(
            all.len(),
            9,
            "a variant was added without extending this table"
        );
        for e in &all {
            let v = serde_json::to_value(e).unwrap();
            assert_eq!(v["type"].as_str().unwrap(), e.event_type(), "{e:?}");
        }
    }

    #[test]
    fn terminal_attempt_events_carry_executor_identity() {
        // SC2 requires executor identity on the terminal outcome, not only on the start event.
        for e in [
            NodeEvent::AttemptFinished {
                task_id: uuid::Uuid::nil(),
                attempt_id: uuid::Uuid::nil(),
                execution_process_id: uuid::Uuid::nil(),
                executor: "claude".into(),
                exit_code: 0,
            },
            NodeEvent::AttemptFailed {
                task_id: uuid::Uuid::nil(),
                attempt_id: uuid::Uuid::nil(),
                execution_process_id: uuid::Uuid::nil(),
                executor: "claude".into(),
                reason: "test".into(),
            },
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
