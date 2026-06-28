# ADR-0005 — Store fence_attempt_count in execution_processes

- **Status:** accepted
- **Date:** 2026-06-27
- **Workstream:** foundations-followup1

## Context

`cleanup_orphan_executions` returns `FenceOutcome::CouldNotKill` when a process survives SIGKILL
(D-state / uninterruptible sleep). The current CouldNotKill arm sets `resume_state='pending'`
and continues, but emits no observable escalation signal after repeated failed attempts.

An operator with a stuck process has no way to know how many restart cycles have been attempted
without reading raw log files. The HiveSyncStatusCard (and future telemetry) needs a DB-observable
counter that survives server restarts (a crash is precisely when D-state processes occur).

## Decision

Add `fence_attempt_count INTEGER NOT NULL DEFAULT 0` to `execution_processes`.

The counter is incremented in the `CouldNotKill` arm of `cleanup_orphan_executions` via a
dedicated scalar accessor (`increment_fence_attempt_count`). When the count reaches
`FENCE_ESCALATION_THRESHOLD` (default: 5), a structured `tracing::warn!` fires with
`process_id`, `fence_attempt_count`, and a human-readable escalation message.

## Alternatives rejected

**In-memory counter (e.g., `Arc<RwLock<HashMap<Uuid, u32>>>`)**
Resets on every server restart. A D-state process forces a restart — meaning the counter resets
exactly when it would otherwise be most useful. Rejected.

**New `fence_attempts` table**
A JOIN-based table avoids ALTER TABLE but adds schema complexity for a single integer.
`ALTER TABLE … ADD COLUMN` with `DEFAULT 0` is safe in SQLite: no backfill required, no locks
beyond schema write. Rejected in favour of the simpler column addition.

## Consequences

- **Irreversible in SQLite:** Column additions via `ALTER TABLE … ADD COLUMN` are supported
  in SQLite 3.1.3+ (2005), but column removal requires a full table rebuild. This column is
  additive and non-breaking; it defaults to 0 for all existing rows. If removal becomes
  necessary, a new migration using the `CREATE TABLE … INSERT … DROP … RENAME` pattern is required.
- **`.sqlx` cache delta:** `increment_fence_attempt_count` and `get_fence_attempt_count` use
  `query!` / `query_scalar!` macros. A `cargo sqlx prepare --workspace` pass is required at
  closeout (per Phase 2a Trap 2) to update the offline cache.
- **No query blast radius:** The `ExecutionProcess` struct (`FromRow`) does NOT include
  `fence_attempt_count`. It is accessed only through the two dedicated scalar accessors,
  following the same pattern as `resume_state` (Phase 2a decisions-ledger "resume-intent column
  accessed via dedicated scalar queries").
