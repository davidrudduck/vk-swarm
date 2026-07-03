# ADR-0011 — Hive-only-state cutover inventory (migrate / regenerate / discard)

- **Status:** accepted
- **Date:** 2026-06-30
- **Workstream:** vk-swarm-hive-redesign
- **Realizes:** SC6 and the **redirected §2.7** (analysis §2.6 — "the §2.7 check is redirected to the
  migration inventory")

## Context

Rebuilding the hive risks destroying state that lives **only** in Postgres with no node-of-record copy.
A full read-only inventory of the hive schema (`crates/remote/migrations/*.sql`,
`crates/remote/src/db/*`) established what exists and whether each item survives a rebuild. Two facts
shape the cutover:

- **The node↔hive id spaces differ** — node `tasks.id` ≠ hive `shared_tasks.id`, bridged by
  `tasks.shared_task_id` ↔ `shared_tasks.source_task_id`/`source_node_id`. This bridge **must** be
  preserved or every existing link breaks.
- **There is no board-ordering/layout table** — kanban order is *derived* from `status` + `activity_at`
  (grep for board/column/position/rank returns only comments). So there is **no operator-entered layout
  to migrate** — a real scope reduction.

## Decision

Classify all hive-only state into three buckets and cut over accordingly:

**MUST-MIGRATE** (one-time data migration into the rebuilt hive, before cutover):

- `node_api_keys` — operator-issued M2M credentials; **irrecoverable** (loss forces re-keying every node).
- `nodes` — node registry/identity (machine_id, public_url, key bindings depend on it).
- `node_task_assignments` (active, `completed_at IS NULL`) — the only record of which node owns which
  in-flight task.
- `swarm_projects` + `swarm_project_nodes` — operator-defined project↔node linking topology (incl.
  `is_owner`).
- `swarm_templates` — operator-authored, org-global.
- `shared_tasks` — the task registry incl. hive-only attribution (`creator_user_id`, `assignee_user_id`,
  `deleted_by_user_id`/`deleted_at`, `owner_node_id`/`executing_node_id`) **and the node↔hive id
  bridge** (`source_task_id`/`source_node_id`). **Apply the status enum value mapping**
  ([ADR-0010](./0010-task-status-state-machine.md)) on re-import.
- `labels` + `shared_task_labels` — hive-created labels and all task↔label assignments.
- Identity/tenancy — `users`, `organizations`, `organization_member_metadata`,
  `organization_invitations`, `oauth_accounts`.

**REGENERABLE** (drop and rebuild from node re-ingest via the new outbox,
[ADR-0008](./0008-node-hive-ordered-ack-outbox.md)):

- `node_local_projects`, `node_execution_processes`, `node_task_output_logs`,
  `node_task_progress_events`, attempt `sync_state`/`backfill_request_id`/`last_full_sync_at`,
  `project_activity_counters`, Electric replication plumbing.

**DISCARDABLE** (not migrated):

- `activity` history feed, `auth_sessions`, `oauth_handoffs`, `revoked_refresh_tokens`, completed
  `node_task_assignments`.

**Cutover sequence.** Because `vk-swarm-node-foundations` already made nodes authoritative for their own
state, cutover is: (1) run the one-time MUST-MIGRATE migration into the rebuilt hive, preserving the id
bridge; (2) bring nodes online — they re-ingest REGENERABLE state via the outbox; (3) discard the
discardable tables.

## Consequences

- SC6 is satisfied with an evidence-backed inventory; the rebuild cannot silently lose operator state.
- Dropping REGENERABLE/DISCARDABLE tables and remapping status values are irreversible data operations —
  gated behind a pre-cutover backup.
- The "no board-ordering table" finding removes a feared migration entirely.

## Alternatives considered

- **Migrate everything wholesale** — rejected: drags transient sync bookkeeping and stale history into
  the rebuilt schema, defeating the point of the rebuild.
- **Rebuild empty and let nodes repopulate everything** — rejected: loses the irrecoverable
  operator-only state (API keys, swarm-project topology, templates, identity, attribution).
