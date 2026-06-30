---
id: "602"
phase: 6
title: No-fanout send-site comment fence — document the SC1 invariant at connection.rs
status: ready
depends_on: ["601"]
parallel: false
conflicts_with: []
files:
  - crates/remote/src/nodes/ws/connection.rs
irreversible: false
scope_test: "N/A"
allowed_change: edit
covers_criteria: []
covers_tests: []
---
## Failing test (write first)
N/A — this task adds ONLY a documentation comment block (no behavior, no new symbol). It is the prose
half of the SC1 no-fanout fence; the executable assertion lives in task 601
(`crates/remote/tests/no_fanout_invariant.rs`). A unit test cannot assert "a future author read a
comment", so verification is manual (below) plus the build staying green. The 601 test is the
mechanical regression guard this comment points reviewers at.

## Manual verification (record in decisions-ledger)
- `cargo check -p vks-hive-server` exits 0 (the added lines are comments only — must not change
  compilation).
- `git diff crates/remote/src/nodes/ws/connection.rs` shows ONLY added comment (`//!` / `//`) lines —
  zero changes to any `fn`, signature, or expression.
- `grep -n "SC1 no-fanout invariant" crates/remote/src/nodes/ws/connection.rs` returns the inserted
  module-level fence (so a future author editing the send primitives sees it).

## Change
- **File:** `crates/remote/src/nodes/ws/connection.rs`
- **Anchor:** the module doc-comment block at the very top of the file (lines 1-4), immediately above
  `use std::{collections::HashMap, sync::Arc};`. Append the invariant note to the existing `//!` block.
- **Before:**
```rust
//! Connection manager for tracking connected nodes.
//!
//! This module provides a centralized registry of connected nodes and their
//! WebSocket channels for sending messages.

use std::{collections::HashMap, sync::Arc};
```
- **After:**
```rust
//! Connection manager for tracking connected nodes.
//!
//! This module provides a centralized registry of connected nodes and their
//! WebSocket channels for sending messages.
//!
//! # SC1 no-fanout invariant (vk-swarm-hive-redesign, phase-6 — data plane)
//!
//! The four send primitives below — [`ConnectionManager::send_to_node`],
//! [`ConnectionManager::broadcast_to_org`], [`ConnectionManager::broadcast_to_org_except`],
//! and [`ConnectionManager::send_to_nodes`] — are the ONLY paths by which the hive pushes a
//! [`HiveMessage`] to a node (the per-node handshake/control replies in `session.rs` use the
//! socket sink directly). The data-plane contract is: **a shared task owned by node X is never
//! pushed to a different node Y.** Every `HiveMessage` variant that flows through here is one of:
//! a per-node control/ack reply, the recipient's OWN assignment/cancel, the recipient's OWN
//! backfill request, or project/label METADATA — NEVER another node's shared-*task* state for
//! relay. Cross-node task/attempt/execution state is served by the hive's own web UI reading
//! Postgres directly (and the browser-facing `electric_proxy`/`ActivityBroker` fan-out), NOT by
//! pushing it to nodes.
//!
//! Do NOT add a `broadcast`/`send_to_nodes` call that relays one node's shared-task state to other
//! nodes. The exhaustive `HiveMessage` classification in
//! `crates/remote/tests/no_fanout_invariant.rs` (test `no_hive_message_variant_is_task_state_fanout`)
//! is the regression fence: a new task-state-push variant breaks that test's exhaustive match or its
//! assertion. If you genuinely need cross-node task fan-out, that is a spec change (escalate), not a
//! new send call here.

use std::{collections::HashMap, sync::Arc};
```

## Allowed moves
ONLY append the `//!` invariant comment block to the existing module doc-comment at the top of
`connection.rs`, exactly as shown. Do NOT modify any function, signature, expression, `use`, or
struct. Do NOT touch `message.rs`, `session.rs`, or the 601 test file. The diff must be comment-only.

## STOP triggers
- The top-of-file lines 1-4 do not match the `Before` block verbatim (the module doc-comment changed
  since authoring) → re-read `crates/remote/src/nodes/ws/connection.rs:1-6` and re-anchor the insertion
  immediately after the existing `//!` block and before the first `use`; keep it comment-only.
- The `[`ConnectionManager::send_to_node`]` intra-doc links trigger a rustdoc/clippy warning under
  `-D warnings` (e.g. broken-intra-doc-links) → downgrade those four links to plain backtick code spans
  (`` `send_to_node` ``, etc.); the prose content is what matters, not the link form. Do NOT suppress
  with `#[allow]`.
- You find yourself changing any non-comment line → STOP; this task is a documentation fence only. Any
  behavioral change here is out of scope (601 already proves the invariant mechanically).
- `cargo check -p vks-hive-server` fails after the edit → the change is not comment-only (a real edit
  slipped in) or an intra-doc link broke; revert to comment-only per above.

## Done when
`WAI_TYPECHECK_CMD="cargo check -p vks-hive-server" WAI_TEST_CMD="cargo check -p vks-hive-server" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-hive-redesign 602` exits 0
(comment-only change; both gate commands are `cargo check` since there is no new test — `scope_test:
N/A` is satisfied by the `## Manual verification` section above, and the build staying green proves the
comment did not perturb compilation.)
