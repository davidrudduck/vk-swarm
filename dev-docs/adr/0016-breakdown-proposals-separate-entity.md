# ADR-0016: Breakdown proposals are a separate entity, never a TaskStatus variant

- **Status:** accepted
- **Date:** 2026-08-07
- **Workstream:** vk-swarm-task-breakdown
- **Spec:** `docs/superpowers/specs/2026-08-07-vk-swarm-task-breakdown.md`

## Context

P3 (AI task breakdown) needs "proposed subtasks" that exist on the board for review but are
not yet actionable tasks. The obvious shortcut — a new `proposed` value on `TaskStatus` — is
disproportionately expensive and risky (2026-08-07 codebase survey):

- Node SQLite stores `tasks.status` as `TEXT` with a `CHECK` constraint
  (`crates/db/migrations/20251215000000_replace_parent_task_attempt_with_parent_task_id.sql:23`);
  SQLite cannot alter a CHECK, so a new value forces the full 12-step table-rebuild
  migration, plus `crates/db/src/validation.rs:24` and multiple hardcoded SQL literals
  (`activity_feed.rs`, `dashboard.rs`, `project/stats.rs`).
- The hive uses a real Postgres enum `task_status`
  (`crates/remote/migrations/20251001000000_shared_tasks_activity.sql:57`) requiring
  `ALTER TYPE ... ADD VALUE` plus a matching Rust variant in `crates/remote/src/db/tasks.rs`.
- The wire boundary `canonical_status_from_node`
  (`crates/remote/src/nodes/ws/status_machine.rs:58-66`) rejects unknown status strings; on
  the legacy `sync_tasks` path a rejected task is re-selected and re-sent **every poll
  indefinitely** (permanent stuck-retry), and `author_of_transition` has a `_ => None`
  wildcard that would silently un-route the new status.
- `Task::create` unconditionally enqueues a `task.upsert` outbox op
  (`crates/db/src/models/task/queries.rs:292`), so proposal rows created as tasks would sync
  to the hive before acceptance — violating the review gate.

## Decision

Proposals live in **new, node-local tables** and become real tasks only on acceptance:

- `task_breakdown_proposals` — one row per breakdown run (parent `task_id`, lifecycle
  `draft | accepted | discarded | failed`, executor/process linkage, timestamps).
- `task_breakdown_proposal_items` — proposed subtasks (title, description, ordering,
  intra-set dependency references).
- `task_dependencies` — first-class dependency edges between **real** tasks
  (`task_id`, `depends_on_task_id`), written at acceptance time; the queryable substrate P5
  consumes.

`TaskStatus` is untouched on node and hive. Acceptance creates ordinary child tasks through
the existing `Task::create` path (hierarchy via `parent_task_id`), which sync via the
existing outbox exactly like operator-created tasks. Proposals themselves never cross the
wire; hive-side visibility of dependency edges is explicitly deferred.

## Consequences

- Positive: zero changes to the status CHECK constraint, Postgres enum, wire status machine,
  or sync retry semantics; the review gate is structural (proposals are not tasks, so no
  attempt can be started and nothing syncs early); migrations are purely additive.
- Negative / accepted cost: a second entity to render on the board (proposal review UI reads
  proposals, not tasks), and `task_dependencies` is node-local until a later phase teaches
  the hive about edges.
- Irreversibility: the migrations are additive, but the **contract** — "proposals are not
  tasks; dependency edges are a separate table keyed on real task ids" — is what P5/P6 will
  build against, so reversing it after P5 starts means rewriting their substrate. Hence this
  ADR.
