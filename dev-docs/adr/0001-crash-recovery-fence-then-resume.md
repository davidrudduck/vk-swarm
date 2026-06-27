# ADR-0001 — Crash recovery via fence-then-resume

- **Status:** accepted
- **Date:** 2026-06-26
- **Workstream:** vk-swarm-node-foundations
- **Supersedes behaviour of:** `cleanup_orphan_executions` blanket-fail boot sweep

## Context

On startup, `cleanup_orphan_executions` (`crates/services/src/services/container.rs:239-337`) calls
`ExecutionProcess::mark_orphaned_as_failed` (`crates/db/src/models/execution_process/queries.rs:114`),
which flips **every** `status='running'` row whose `server_instance_id != current` to `failed`.
Because `server_instance_id` is regenerated (`Uuid::new_v4()`) on every boot
(`crates/local-deployment/src/container.rs:137`), after a real crash this fails *all* in-flight runs and
the subsequent per-process loop finds nothing — the `InReview` flip and Hive push at `:315-318` are
**dead code on a crash**. The durable materials to resume are already on disk: worktree
(`task_attempts.container_ref`), the full replayable `executor_action` JSON (carries executor profile
+ prompt), and the agent `session_id` (`executor_sessions`). A working relaunch path exists
(`ClaudeCode::spawn_follow_up(--resume)`, `crates/executors/src/executors/claude.rs:252`, dispatched via
`coding_agent_follow_up.rs:53`). Recovery simply never uses them.

**Safety hazard:** runtime child handles live only in memory (`child_store`/`msg_stores` HashMaps,
`crates/local-deployment/src/container.rs:84-85`) and are lost on crash, but the **OS agent process can
outlive the server** (analysis §3.3). Naively re-spawning `--resume` into a worktree whose original
agent is still running produces **two writers racing the same git index** → corruption.

## Decision

Replace the blanket-fail sweep with a **fence-then-resume** recovery routine that runs **before** any
failure-marking, per `running` CodingAgent process:

1. **Fence.** Determine whether the original OS process is still alive using `pid` **plus a
   fingerprint** (process start-time / pgid) to defeat PID reuse. If alive, terminate it (and its
   process group) and confirm exit. Never proceed to step 2 until the worktree has no live writer.
2. **Resume.** If the executor supports session resume and a `session_id` is persisted, reconstruct the
   `ExecutorAction` from the DB and re-enter `start_execution` (`container.rs:1617`) to relaunch with
   `--resume` into `container_ref`. Mark the row with the resume-intent state (see ADR-0003).
3. **Fail only as last resort.** Mark `failed` (and propagate outward) **only** when the run is
   genuinely unrecoverable — no `session_id`, or a non-resumable executor with no fallback (per the
   executor-capability fallback policy in the spec's Decisions).

Recovery is ordered before `mark_orphaned_as_failed`; the blanket query is narrowed/retained only for
truly-abandoned rows.

## Consequences

- Crash → restart re-spawns in-context with no manual archaeology (SC1); a transient `failed`/`InReview`
  is no longer broadcast for runs that are actually resumable (SC8).
- New dependency on reliable liveness+fingerprint detection (sysinfo/procfs). PID-reuse false-positives
  are the main risk; the fingerprint mitigates.
- **Single-node assumption made explicit:** `mark_orphaned_as_failed` also matches *other live nodes'*
  rows (it keys on `!= current_instance`). Acceptable for a single node; **`vk-swarm-hive-redesign` must
  revisit** crash-vs-foreign disambiguation (lease/ownership) before multi-node.
- Re-attach to a *still-live* process (observe without re-spawn) is explicitly **not** adopted here —
  the in-memory log-forwarder handles are gone, so fence-then-resume is the universal mechanism.

## Alternatives considered

- **Re-attach to the live PID** — rejected as the primary path: control/stream handles are in-memory and
  unrecoverable after crash; an observed-but-undrivable process is worse than a clean resume.
- **Keep blanket-fail, rely on user follow-up** — status quo; loses context and is the pain this
  workstream exists to kill.
