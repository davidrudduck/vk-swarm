# ADR-0003 — Durable local state schema (persist the message queue; mark resume intent)

- **Status:** accepted
- **Date:** 2026-06-26
- **Workstream:** vk-swarm-node-foundations

## Context

Two durability gaps block crash-resumability:

1. **Queued follow-up prompts are volatile.** `MessageQueueStore` is an in-memory
   `HashMap<Uuid, Vec<QueuedMessage>>` (`crates/local-deployment/src/message_queue.rs:49`), so any crash
   drops prompts the operator already submitted.
2. **Run state is fully on disk but lacks resume *intent*.** A run is already described by
   `task_attempts` + ordered `execution_processes` (full replayable `executor_action` JSON, status,
   pid, run_reason, before/after HEAD) + `executor_sessions` (`session_id`, `prompt`). Verification
   confirmed **no new run table is needed**. What is missing is a way to distinguish "was running and
   should be resumed" from "was running and is now abandoned/failed," which fence-then-resume (ADR-0001)
   needs at boot.

## Decision

**Extend the existing local schema; add exactly one new table.** Do not introduce a separate
workstream/run entity.

- **New table `queued_messages`** backing `MessageQueueStore`, preserving the in-memory shape:
  `id`, `task_attempt_id` (key), `content`, `variant`, `position` (ordering), `created_at`. The store's
  `add`/`remove`/`clear`/`peek_next` operations become DB-backed; on boot, attempts with rows re-trigger
  the existing drain (`try_consume_queued_message`, `container.rs:1179`, fired at `:738`).
- **Resume-intent marker on `execution_processes`** (a small column, e.g. `resume_state` /
  `resumable BOOLEAN`, or a status nuance) set/read by `cleanup_orphan_executions` so recovery can
  classify each `running` row as resume vs abandon.
- **Explicit read-only assembling view** (`v_workstream_state` or a query helper) joining
  `task_attempts` + latest/relevant `execution_processes` + `executor_sessions`, so the workstream-state
  object is **queryable** for recovery now and for downstream phases (P3/P6) later — satisfying SC3's
  "first-class" intent without a redundant table.

## Consequences

- Forward-only migrations (new table + new column + view) — hence this ADR (schema is hard to walk back).
- Queued prompts survive restart (SC2); recovery can act on intent (SC1/ADR-0001); a stable queryable
  surface exists for downstream consumers (SC3).
- Local schema stays the single source of truth; the node remains authoritative and hive-independent.

## Alternatives considered

- **New first-class `workstreams`/`sessions` tables** (cf. upstream's workspaces split) — rejected for
  this workstream: the existing triple already encodes the needed state; a new entity is migration risk
  and duplication. The *sessions concept* may be revisited later, separately.
- **Keep the queue in memory, accept loss** — rejected: directly violates SC2 and the offline-first
  principle.
