---
id: "601"
phase: 6
title: No-fanout invariant guard — exhaustive HiveMessage classification + topology assertion
status: ready
depends_on: ["103", "202", "501"]
parallel: false
conflicts_with: []
files:
  - crates/remote/tests/no_fanout_invariant.rs
irreversible: false
scope_test: "crates/remote/tests/no_fanout_invariant.rs"
allowed_change: create
covers_criteria: [SC1]
covers_tests: [TS7]
---
## Failing test (write first)
**HERMETIC — NO DATABASE (Trap 2b does NOT apply).** This task asserts a *static* protocol invariant
over the `HiveMessage` enum (the only thing a node can ever receive). It does **not** touch Postgres,
does **not** call a `query!`/`query_as!`, and does **not** stand up a node↔hive socket. There is NO
`skip_without_db!` guard because there is no DB precondition — the test runs and passes (or fails to
compile) on any checkout. Do NOT add a `DATABASE_URL` precondition; doing so would make a hermetic
guard look like the skip-guarded hive tests and invite a hollow pass.

**Why this is the right shape (the invariant restated).** SC1's data-plane half is "no node↔node /
node↔hive↔node fan-out": a shared task owned by node X is never *pushed* to a different node Y. The hive
delivers `HiveMessage` to nodes by exactly four send primitives in
`crates/remote/src/nodes/ws/connection.rs` — `send_to_node` (123), `broadcast_to_org` (138),
`broadcast_to_org_except` (159), `send_to_nodes` (189) — plus per-node direct replies via
`send_message` in `session.rs`. The invariant is structural: **every variant of `HiveMessage` is either
(a) a per-node control/handshake/ack reply, (b) the *recipient's own* assignment/cancel, (c) the
*recipient's own* backfill request, or (d) project/label *metadata* — and NONE carries another node's
shared-*task* state for relay to a third party.** A future variant that pushed shared task state would
have to be added to `HiveMessage`, and this test forces that addition to be classified (via an
exhaustive `match`) and then asserts the classification contains no `TaskStatePush`. That is the
regression fence: you cannot add a fan-out variant without either (i) editing this test's `match` (which
makes the intent reviewable) or (ii) breaking the build (non-exhaustive match under `-D warnings`).

**The test MUST be a NEW integration test file** `crates/remote/tests/no_fanout_invariant.rs`.
`HiveMessage` is `pub` and reachable as `remote::nodes::ws::message::HiveMessage`
(crate name is `remote`; `nodes`, `ws`, `message` are all `pub mod`). An external integration
test sees it. **Sibling read (rubric #9):** `crates/remote/tests/pool_config.rs` is the existing
*hermetic* (no-DB) integration test in this dir — copy its bare `use remote::…;` + `#[test]`
shape (NOT `backfill_e2e.rs`, which is the DB-bound template and the wrong model here).

Create `crates/remote/tests/no_fanout_invariant.rs` with EXACTLY this content:
```rust
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
```
> Run before the change exists: `cargo test -p remote --test no_fanout_invariant` — RED
> (the file does not compile/exist). After creating the file it is GREEN. The fence value is in the
> RED-on-future-regression: adding a task-state-push `HiveMessage` variant either breaks `classify`'s
> exhaustive match (compile error) or, if classified `TaskStatePush`, fails this assertion.

### DECISION-LOCKED forward variants (this phase executes AFTER P1/P2/P5 — read before authoring)
Phase-6 is sequenced **after** P1, P2, P5 (plan: "Depends P1, P2"; P5 adds a hive→node variant too).
Those phases ADD `HiveMessage` variants per CONTRACT §A, so by the time 601 runs, `HiveMessage` may
carry variants NOT present on `main` today. The exhaustive `match` in `classify` would then fail to
compile. **This is anticipated and pre-decided — do NOT use executor judgment.** When a CONTRACT §A
hive→node variant exists at execution time, add EXACTLY these arms to `classify` (and a matching
constructor to `one_of_each()`), then keep the assertion as-is:

| HiveMessage variant (when present) | Phase | `Delivery` arm | Why it is NOT fan-out |
|------------------------------------|-------|----------------|------------------------|
| `OpAck { applied_through_seq }` | P1 (task 103) | `PerNodeControl` | durable ack on the recipient's OWN op-log cursor |
| `LeaseGrant { .. }` | P2 | `OwnAssignment` | lease for the recipient's OWN assignment |
| `LeaseRevoked { .. }` | P2 | `OwnAssignment` | revokes the recipient's OWN assignment lease |
| `DigestResult { .. }` | P5 (task 501) | `PerNodeControl` | reply to the recipient's OWN digest exchange |

(All four are struct-variants per CONTRACT §A — match with `{ .. }`. `OpAck` is already authored into the
test below because this task `depends_on: 103`; the P2/P5 arms are added only if those phases have landed.)

None of these is another node's shared-task state for relay → none is `TaskStatePush`; the assertion
still passes. If a §A variant's payload differs from the contract shape, build the constructor from the
ACTUAL struct in `message.rs` (read it). If a variant NOT in this table or CONTRACT §A appears and it
genuinely pushes another node's shared-TASK state to a third node — that is a real SC1 regression: STOP
and escalate (Trap 6); do NOT classify it benign to make the test green.

## Change
- **File:** `crates/remote/tests/no_fanout_invariant.rs`
- **Anchor:** new file (does not exist on `main`).
- **Before:** (none — file is created)
- **After:** EXACTLY the file contents in the "Failing test" block above.

## Allowed moves
ONLY create `crates/remote/tests/no_fanout_invariant.rs` with the content above, PLUS — if and only if
a CONTRACT §A hive→node variant from the "DECISION-LOCKED forward variants" table exists at execution
time — the pre-decided `classify` arm + matching `one_of_each()` constructor for that variant (exactly
the `Delivery` arm the table assigns). No other source file may change: do NOT modify `message.rs`,
`connection.rs`, `session.rs`, or anything else (602 owns the `connection.rs` comment fence). Do NOT add
a Postgres/`DATABASE_URL` precondition. Do NOT add new dependencies — `uuid` (crate dep
`crates/remote/Cargo.toml:33`) and `chrono` (`:17`) are already normal deps of `remote` and
available to integration tests. If a struct field name in `one_of_each()` does not match `message.rs`,
align the literal to the ACTUAL field (read `message.rs`) — do NOT change `message.rs`.

## STOP triggers
- The exhaustive `match` in `classify` does NOT compile because the variant list differs from
  `message.rs` (a variant was added/removed/renamed since this task was authored) → FIRST consult the
  "DECISION-LOCKED forward variants" table: if it is one of `OpAck`/`LeaseGrant`/`LeaseRevoked`/
  `DigestResult` (P1/P2/P5 added them between authoring and execution), apply the pre-decided arm +
  constructor from that table — this is NOT executor judgment, it is decision-locked. Only if the new/
  renamed variant is NOT in that table do you read the current `HiveMessage`
  (`crates/remote/src/nodes/ws/message.rs:91`) and reason about it — and if it genuinely pushes another
  node's shared-TASK state to a third node, that is a real SC1 finding: STOP and escalate (Trap 6), do
  NOT silently classify it benign to make the test pass.
- A field literal in `one_of_each()` is rejected by the compiler (field name/type drift) → align the
  literal to `message.rs`; do NOT edit `message.rs`.
- The import path `remote::nodes::ws::message::…` fails → confirm the crate name
  (`crates/remote/Cargo.toml` `name = "remote"`) and that `nodes`/`ws`/`message` are still
  `pub mod`. Adjust the `use` path only; do NOT add `pub` to anything (the modules are already `pub`).
- You feel the need to touch `connection.rs` → that is 602's file. STOP; this task is `create`-only.

## Done when
`WAI_TYPECHECK_CMD="cargo check -p remote --tests" WAI_TEST_CMD="cargo test -p remote --test no_fanout_invariant" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-hive-redesign 601` exits 0
