# ADR-0007 — Collapse hive→node delivery to one channel, one delete semantic, one conflict policy

- **Status:** accepted
- **Date:** 2026-06-30
- **Workstream:** vk-swarm-hive-redesign
- **Supersedes behaviour of:** the dual REST-bulk-snapshot + WS-activity inbound paths and the dead
  `ElectricTaskSyncService` task-shape path

## Context

Verified in code (read-only investigation, file:line):

- **Two live inbound channels deliver the same logical change.** A hive `update_task` writes
  `shared_tasks` **and** inserts an activity row in one transaction
  (`crates/remote/src/db/tasks.rs:892-893`). The `shared_tasks` write feeds the **REST bulk snapshot**
  reconcile (`node_runner.rs:986` → `RemoteClient::fetch_bulk_snapshot`, `remote_client.rs:752` →
  `Task::upsert_remote_task`, `node_runner.rs:1012`); the activity row feeds the **WS activity stream**
  (`project_watcher_task` → `ActivityProcessor::process_event`, `processor.rs:57`). Both run
  continuously, so one change double-delivers.
- **The `ElectricTaskSyncService` task-shape poll is dead** — `sync_project_tasks`
  (`electric_task_sync.rs:164`) has zero runtime callers; the frontend Electric collections subscribe
  only to `nodes`/`projects`/`node_projects` shapes, not `shared_tasks` (`collections.ts:82,99,116`).
- **Updates are idempotent** (all three apply sites funnel through `upsert_remote_task` with the gate
  `WHERE excluded.remote_version > tasks.remote_version OR tasks.remote_version IS NULL`,
  `task/sync.rs:300`), **but DELETE diverges**: bulk-snapshot reconcile **hard-deletes** the local row
  (`Task::delete_by_shared_task_id`, `node_runner.rs:1034`, `task/sync.rs:370`) while the WS
  `task.deleted` path only **soft-unlinks** (`Task::set_shared_task_id(.., None)`, `processor.rs:437`).
  For the **same** hive soft-delete (`shared_tasks.deleted_at = NOW()`, `remote/src/db/tasks.rs:1122`),
  the node's outcome depends on which channel wins the race.
- **`task.reassigned` is dropped** by the activity processor (`processor.rs:77` `_ =>` arm), so
  reassignment only lands via the bulk snapshot — a hard channel dependency.
- **Local edits are silently clobbered:** the inbound gate is `remote_version`-only and the local
  update path does **not** bump `remote_version` (`task/queries.rs:305-307`), so an arriving hive
  `version = N+1` overwrites a concurrent local edit with no dirty-flag or merge.

This is the §2.7 "broadcast fan-out double-delivery" item resolved in code (analysis §2.7).

## Decision

1. **One live inbound channel.** The **WS activity stream is the single live delivery path**. The REST
   bulk snapshot is demoted to a **cold-start / gap-fill reconcile** only — it never runs as a second
   continuous channel alongside the WS stream. The dead `ElectricTaskSyncService` task-shape path is
   removed.
2. **One delete semantic.** A hive soft-delete maps to **soft-unlink + tombstone on the node**
   (clear `shared_task_id`, retain local `task_attempt`/run artifacts), applied **identically** on the
   live channel and the reconcile. The hub owns the board; the node never loses local work it ran.
3. **One conflict policy.** The hive is authoritative for shared-task fields; node-local edits travel
   up the ordered outbox ([ADR-0008](./0008-node-hive-ordered-ack-outbox.md)). The node applies a
   **dirty-guard**: an inbound update never overwrites a field that has an unacked outbound op. No
   silent clobber; the `remote_version`-only gate is replaced.
4. **No dropped event types.** The single channel must handle `task.reassigned` (and every authored
   event), closing the `processor.rs:77` gap.

## Consequences

- Eliminates the non-deterministic delete and the double-delivery harm class (SC7); reassignment is no
  longer channel-dependent.
- The reconcile path is repurposed as the anti-entropy gap-fill ([ADR-0008](./0008-node-hive-ordered-ack-outbox.md), SC5),
  not a parallel writer.
- Removing the Electric task-shape code is a deletion (irreversible); the Electric proxy used by the
  hive UI for `nodes`/`projects` shapes is untouched.

## Alternatives considered

- **Keep both channels, dedup by version** — rejected: leaves the divergent DELETE and the dropped
  `task.reassigned`, and the version gate cannot protect local edits.
- **Make Electric the single channel** — rejected: it is dead on both the Rust node side and the
  frontend; finishing it would re-introduce a third apply path.
