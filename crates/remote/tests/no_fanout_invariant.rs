//! SC1 data-plane guard — no node↔node / node↔hive↔node fan-out.
//!
//! Asserts the structural invariant that the node-facing channel (`HiveMessage`,
//! the ONLY thing a node can receive) never carries one node's shared-*task* state
//! for relay to a different node. The exhaustive `match` below is a regression
//! fence: a new `HiveMessage` variant cannot be added without classifying it here,
//! and the test rejects any classification of `TaskStatePush`.
//!
//! Pairs with the send-site comment fence in
//! `crates/remote/src/nodes/ws/connection.rs` (task 602).

use remote::nodes::ws::message::{
    AuthResultMessage, BackfillRequestMessage, BackfillType, HiveMessage, LabelSyncBroadcastMessage,
    NodeRemovedMessage, ProjectSyncMessage, TaskAssignMessage, TaskCancelMessage, TaskDetails,
    TaskSyncResponseMessage,
};

/// How a `HiveMessage` reaches a node — the allowed delivery classes for SC1.
#[derive(Debug, PartialEq, Eq)]
enum Delivery {
    /// Per-node handshake / control / ack reply on the recipient's OWN socket
    /// (auth_result, heartbeat_ack, status_request, error, close, task_sync_response).
    PerNodeControl,
    /// The recipient's OWN assignment or cancel of its OWN assignment.
    OwnAssignment,
    /// A request for the recipient to push ITS OWN data up to the hive (carries no task state).
    OwnBackfillRequest,
    /// Project METADATA (link id, repo path, branch, owner-node display) — NOT shared-task state.
    ProjectMetadata,
    /// Organization-global label metadata broadcast (label name/icon/color/version) — NOT task state.
    LabelMetadata,
    /// FORBIDDEN: a push of one node's shared-TASK state to a different node. No variant may map here.
    /// Present only so the assertion below is meaningful; if a future variant is task-state fan-out,
    /// classify it here and watch this test fail (that is the point).
    #[allow(dead_code)]
    TaskStatePush,
}

/// Total, exhaustive classification of every `HiveMessage` variant. Adding a variant without
/// extending this `match` fails to compile under `-D warnings` — the regression fence.
fn classify(msg: &HiveMessage) -> Delivery {
    match msg {
        HiveMessage::AuthResult(_) => Delivery::PerNodeControl,
        HiveMessage::HeartbeatAck { .. } => Delivery::PerNodeControl,
        HiveMessage::StatusRequest { .. } => Delivery::PerNodeControl,
        HiveMessage::Error { .. } => Delivery::PerNodeControl,
        HiveMessage::Close { .. } => Delivery::PerNodeControl,
        HiveMessage::TaskSyncResponse(_) => Delivery::PerNodeControl,
        HiveMessage::TaskAssign(_) => Delivery::OwnAssignment,
        HiveMessage::TaskCancel(_) => Delivery::OwnAssignment,
        HiveMessage::BackfillRequest(_) => Delivery::OwnBackfillRequest,
        HiveMessage::ProjectSync(_) => Delivery::ProjectMetadata,
        HiveMessage::NodeRemoved(_) => Delivery::ProjectMetadata,
        HiveMessage::LabelSync(_) => Delivery::LabelMetadata,
        // Added by P1 task 103 (this task `depends_on: 103`, so OpAck is present at execution time):
        // durable ack on the recipient's OWN op-log cursor — control, never task-state fan-out.
        HiveMessage::OpAck { .. } => Delivery::PerNodeControl,
        // P2 lease variants (this task `depends_on: 202`, so they exist at execution time). A lease
        // grant/revoke targets the recipient's OWN assignment — control, never task-state fan-out.
        // Shapes per CONTRACT §A (struct-variants → match `{ .. }`).
        HiveMessage::LeaseGrant { .. } | HiveMessage::LeaseRevoked { .. } => Delivery::OwnAssignment,
        // P5 digest result (this task `depends_on: 501`) — directs the recipient's OWN heal; control.
        HiveMessage::DigestResult { .. } => Delivery::PerNodeControl,
    }
}

fn sample_uuid() -> uuid::Uuid {
    uuid::Uuid::nil()
}

/// One representative value per `HiveMessage` variant. This forces every variant to be CONSTRUCTED
/// here, so renaming/removing a variant breaks this list (a second fence alongside `classify`).
fn one_of_each() -> Vec<HiveMessage> {
    let now = chrono::Utc::now();
    vec![
        HiveMessage::AuthResult(AuthResultMessage {
            success: true,
            node_id: Some(sample_uuid()),
            organization_id: Some(sample_uuid()),
            error: None,
            protocol_version: 1,
            linked_projects: vec![],
            swarm_labels: vec![],
        }),
        HiveMessage::HeartbeatAck { server_time: now },
        HiveMessage::StatusRequest { message_id: sample_uuid() },
        HiveMessage::Error { message_id: None, error: "x".into() },
        HiveMessage::Close { reason: "x".into() },
        HiveMessage::TaskSyncResponse(TaskSyncResponseMessage {
            local_task_id: sample_uuid(),
            shared_task_id: sample_uuid(),
            success: true,
            error: None,
        }),
        HiveMessage::TaskAssign(TaskAssignMessage {
            message_id: sample_uuid(),
            assignment_id: sample_uuid(),
            task_id: sample_uuid(),
            node_project_id: sample_uuid(),
            local_project_id: sample_uuid(),
            task: TaskDetails {
                title: "t".into(),
                description: None,
                executor: "CLAUDE_CODE".into(),
                executor_variant: None,
                base_branch: "main".into(),
            },
        }),
        HiveMessage::TaskCancel(TaskCancelMessage {
            message_id: sample_uuid(),
            assignment_id: sample_uuid(),
            reason: None,
        }),
        HiveMessage::BackfillRequest(BackfillRequestMessage {
            message_id: sample_uuid(),
            backfill_type: BackfillType::FullAttempt,
            entity_ids: vec![],
            logs_after: None,
        }),
        HiveMessage::ProjectSync(ProjectSyncMessage {
            message_id: sample_uuid(),
            link_id: sample_uuid(),
            project_id: sample_uuid(),
            project_name: "p".into(),
            local_project_id: sample_uuid(),
            git_repo_path: "/r".into(),
            default_branch: "main".into(),
            source_node_id: sample_uuid(),
            source_node_name: "n".into(),
            source_node_public_url: None,
            is_new: true,
        }),
        HiveMessage::NodeRemoved(NodeRemovedMessage {
            node_id: sample_uuid(),
            reason: "x".into(),
        }),
        // P1 task 103 variant (present because this task depends_on 103). If 103's payload shape
        // differs from CONTRACT §A `{ applied_through_seq: i64 }`, build from the actual variant.
        HiveMessage::OpAck { applied_through_seq: 0 },
        HiveMessage::LabelSync(LabelSyncBroadcastMessage {
            message_id: sample_uuid(),
            shared_label_id: sample_uuid(),
            project_id: None,
            origin_node_id: sample_uuid(),
            name: "l".into(),
            icon: "tag".into(),
            color: "#fff".into(),
            version: 1,
            is_deleted: false,
        }),
        // P2 lease variants (depends_on 202) — shapes per CONTRACT §A.
        HiveMessage::LeaseGrant {
            assignment_id: sample_uuid(),
            fencing_token: 1,
            lease_expires_at: now,
        },
        HiveMessage::LeaseRevoked { assignment_id: sample_uuid(), reason: "x".into() },
        // P5 digest result (depends_on 501) — shape per CONTRACT §A.
        HiveMessage::DigestResult { resend_from_seq: None, pull_entities: vec![] },
    ]
}

/// TS7 — no fan-out: NO `HiveMessage` variant the hive can deliver to a node is a push of another
/// node's shared-TASK state. A task owned by node X is never relayed to node Y as task state; the
/// only task-shaped variants (`TaskAssign`/`TaskSyncResponse`) are the recipient's OWN assignment /
/// its OWN sync ack, classified as `OwnAssignment` / `PerNodeControl`, never `TaskStatePush`.
#[test]
fn no_hive_message_variant_is_task_state_fanout() {
    for msg in one_of_each() {
        let class = classify(&msg);
        assert_ne!(
            class,
            Delivery::TaskStatePush,
            "HiveMessage variant {msg:?} classified as forbidden TaskStatePush fan-out — \
             SC1 no-fanout invariant violated. If this is a legitimate new control/assignment/\
             metadata variant, classify it accordingly; a task-state push to nodes is OUT of \
             scope (the hive UI reads Postgres directly)."
        );
    }
}