# vk-swarm — Phase 1 Deep Analysis & Remediation Scoping

**Date:** 2026-06-25
**Status:** Findings (authoritative source for the Phase 2 PRDs; PRDs link here, do not duplicate)
**Scope:** Phase 1 of the 8-phase vk-swarm orchestration-platform program — deep analysis of the
existing codebase to de-risk Phase 2 (foundational remediation).
**Method:** Read-only investigation by four parallel subagents; every claim cited to `file:line`.
Existing `docs/architecture/*.mdx` were used as a map and **verified against source**, not trusted.

> **Settled context (not relitigated here):** vk-swarm is a **standalone application** built on the
> vk-swarm codebase, not a WednesdayAI extension. The two non-negotiable design principles are
> (1) **offline-first durable local state** and (2) **durable resumability**. WednesdayAI integration
> is the last, optional phase. See the program kickoff brief.

---

## 0. Executive Summary

vk-swarm is **structurally sound where it matters most and broken precisely where the program needs
it to be strong.** The crate layering, the executor abstraction, and the local-SQLite storage model
are keepers. The two subsystems that Phase 2 must guarantee — **crash-resumable workstreams** and
**durable multi-node sync** — are exactly the two that are deficient, and for opposite reasons:

| Question | Verdict | One-line |
| --- | --- | --- |
| **Is Hive sync instability fixable schema debt, or architectural?** | **Architectural** (defects are real and in the design) | Migration churn is the *symptom*; the disease is multi-channel sync with no cross-entity ordering, no end-to-end ack, and two divergent inbound paths. The *fix's scope* — targeted ordering-guard vs full reconciliation-contract rebuild — is an **open Phase-2 design question** (§2.7). |
| **How close are we to crash-resumable workstreams?** | **Partial — and recovery actively destroys state** | The durable materials (worktree path, OS PID, `session_id` resume token, executor action JSON) are **already persisted**; boot recovery force-fails everything and never resumes. **The re-spawn-with-`--resume` fix is cheap** (token + relaunch already exist); live-PID re-attach is a separate, unconfirmed optimization. |
| **Is local state truly offline-first?** | **Yes** | Hive linkage is additive nullable metadata; losing the Hive loses no local run state. Keep this model. |
| **Should we rebase onto current upstream vibe-kanban?** | **No — stay put, forward-port selectively** | Diverged ~Dec 2025; upstream rewrote into 28 crates + a workspaces/sessions split. Rebase infeasible; cherry-pick 3 stability fixes. |

**The single most important Phase-2 insight:** the resumability fix and the sync fix are
**independent and asymmetric in cost.** Resumability (via re-spawn-with-`--resume`) is mostly "wire
up capabilities that already exist on disk" (days–weeks, very high leverage). The sync fix is the
genuine architectural lift — but **how big a lift is itself an open question**: the minimal fix may
be a targeted cross-entity ordering guard + ack'd delivery (days), or a full reconciliation-contract
rebuild (weeks), depending on a code-level check deferred to Phase 2 design (§2.7). Phase 2 should
stage them: **2a resumability + stability forward-ports first**, **2b sync fix second.**

---

## 1. Architecture Audit

### 1.1 Subsystem health scoreboard

| Subsystem | Health | Note |
| --- | --- | --- |
| Crate structure / layering | **Healthy** | Clean acyclic layered DAG; no cycles. |
| Executor abstraction | **Healthy** | `enum_dispatch` trait over 12 agents + per-agent log normalizers — the product's strongest design. (`claude.rs` is a 3,052-line god-file: refactor, don't rebuild.) |
| Server / routes / MCP | **Healthy** | ~35 route modules, clean MCP tool server, dedicated WS streaming util, consistent `ApiResponse<T>`. |
| Local SQLite storage model | **Healthy** | Offline-first, authoritative, WAL-checkpointed. |
| Data model (sync columns) | **Needs-work** | Core project/task/attempt schema stable; **remote-sync columns visibly unstable** (repeated `reset_*`/`remove_*_cache`). |
| Node registry | **Needs-work** | `CachedNode` + Hive relay coherent but recently re-architected (`drop_legacy_cached_tables`); unsettled. |
| Frontend | **Fragile** | **Two real-time stacks coexist** mid-migration (`react-use-websocket` + `@electric-sql`). |
| Sync / reconciliation layer | **Fragile** | See §2 — architectural defects, not cosmetic. |
| Crash recovery / resumability | **Fragile** | See §3 — recovery destroys resumable state. |

### 1.2 Crates (8) — clean layered DAG

Verified via `path = "../"` edges in each `Cargo.toml` (outgoing → / incoming ←):

- `utils` → 0 / ← **7** — foundation (shared helpers).
- `executors` → 1 (`utils`) / ← 4 — AI-agent abstraction.
- `db` → 2 (`executors`, `utils`) / ← 4 — SQLx models + migrations.
- `remote` → 1 (`utils`) / ← 2 — standalone Hive server (`vks-hive-server`, Postgres).
- `services` → 4 (`db`, `executors`, `remote`, `utils`) / ← 3 — business logic (46 modules).
- `deployment` → 4 / ← 2 — deployment trait.
- `local-deployment` → 4 / ← 1 — concrete local impl.
- `server` → **7** (all) / ← 0 — aggregation root (`vks-node-server`, `vks-mcp-server`).

Two unambiguous "hubs": `utils` is the most-depended-on foundation; `server` is the aggregation
root. No cycles. **Keep this structure.**

### 1.3 Data model — core entities

`project` (`models/project/`) → has-many `task` (`models/task/`) → has-many `task_attempt`
(`task_attempt.rs`, carries `origin_node_id`) → has-many `execution_process/` →
`execution_process_logs` + `log_entry/`. `executor_session.rs` ties an attempt to an agent run;
`cached_node.rs` mirrors Hive node state locally. Supporting: `draft`, `label`, `template`,
`webhook`, `merge`, `activity_feed`, `dashboard`. **The sync-facing columns are the only unstable
part** (see §2.2).

### 1.4 Executor abstraction — the product core (keep)

- Core trait `StandardCodingAgentExecutor` at `crates/executors/src/executors/mod.rs:235`,
  `#[enum_dispatch]` over `enum CodingAgent` at `mod.rs:101`
  (`ClaudeCode, Amp, Gemini, Codex, Opencode, CursorAgent, QwenCode, Copilot, Droid` + ACP).
- Log normalization: `trait LogNormalizer` at `crates/executors/src/logs/normalizer.rs:20`
  emitting `Vec<Patch>` (JSON-patch); shared tool modeling in `logs/tool_states.rs`.
- Spawn/supervision: `command.rs`, `stdout_dup.rs`, `approvals.rs`, `mcp_config.rs`,
  `session_index.rs`.

This is genuine depth and the right abstraction. **Caveat:** `claude.rs` (3,052 lines) and
`codex.rs` (1,161 lines) are monoliths — split them (refactor, not rebuild).

### 1.5 Server / routes / MCP (keep)

~35 route modules under `crates/server/src/routes/` (dir modules for `projects/`, `tasks/`,
`task_attempts/`). MCP server at `crates/server/src/mcp/task_server.rs` via `rmcp` 0.5. Real-time
streaming: `routes/logs.rs`, `routes/processes.rs`, shared `crates/server/src/ws_util.rs`; plus
`terminal.rs` (xterm), `events.rs`, `proxy.rs`.

### 1.6 Frontend (fragile — consolidate)

React 18 + Vite + TS + Tailwind/shadcn. Backend comms in `frontend/src/lib/api/`; domain hooks in
`frontend/src/hooks/`; client state via `zustand`, server state via `@tanstack/react-query`.
**Two real-time paradigms coexist:** legacy `react-use-websocket@4.7` (consuming `logs.rs`/
`processes.rs` streams) **and** `@electric-sql/client` + `@electric-sql/react` +
`@tanstack/electric-db-collection` (under `frontend/src/lib/electric/`). Carrying both is the single
largest source of frontend complexity and a state-divergence risk.

---

## 2. Hive / Sync Root-Cause — **Verdict: Architectural**

### 2.1 The verdict and its single most decisive piece of evidence

The instability is a **deeper architectural problem in the sync/reconciliation design.** Schema
cleanup will **not** stabilize it.

**Decisive evidence:** `crates/db/migrations/20260128000000_reset_attempt_sync_for_shared_tasks.sql`
documents in its own header that *"attempts were synced before tasks were linked to hive, resulting
in wrong shared_task_id values in hive's node_task_attempts table."* This is a **cross-entity
ordering bug**: parent (`tasks`) and child (`task_attempts`) propagate through **independent sync
channels** with no ordering or atomicity guarantee, so a child can land before its parent. No
migration consolidation fixes this — it is intrinsic to the multi-path design. A second migration,
`20251230032410_fix_is_remote_for_local_projects.sql`, indicts the conflict logic directly:
*"the upsert_remote_task ON CONFLICT clause overwrote a local task."* These are SQL `UPDATE`
statements run against **live data** to repair corruption the sync design produced.

### 2.2 Migration churn — debt as *symptom*, not disease

112 migrations total (86 local-SQLite, 26 Hive-Postgres). The count alone is high-but-survivable;
the **nature** of the churn is the tell:

- ALTER churn concentrated on 4 core tables: `task_attempts` (28 ALTERs), `execution_processes`
  (23), `projects` (19), `tasks` (14).
- A whole sync subsystem built then demolished: `shared_tasks` created
  (`20251114000000_create_shared_tasks.sql`) then dropped
  (`20260102190717_drop_shared_tasks_table.sql`, header: *"ElectricSQL now syncs tasks directly"*) —
  a mid-flight pivot from cache-table to Electric-shape sync.
- `cached_nodes`/`cached_node_projects` created (`20251203`) then dropped
  (`20251205`, `20260126040426_remove_remote_project_cache.sql`); `node_cache.rs:1` self-labels
  *"legacy implementation."*
- Hive rebuilt its task-sync twice (`20260124000000_rebuild_swarm_task_sync.sql`).
- **Repeated live-data repair/reset migrations** — `clear_remote_project_id`,
  `reset_remote_project_links` (*"The hive linking system has had issues… clears all existing
  links"*), `reset_attempt_sync_for_shared_tasks`, `fix_is_remote_for_local_projects`. Maintainers
  periodically wipe and re-establish sync state in production. **This is the clearest signature of an
  unsettled sync model.**

### 2.3 Sync architecture as-built — the structural defect

**Two independent, overlapping delivery channels, plus separate per-entity push paths:**

- **Local → Hive:** tasks via `SharePublisher::share_task()` (`share/publisher.rs`, HTTP POST) +
  `HiveSyncService::sync_tasks()` (`hive_sync.rs:216`, WS batch); attempts/processes/logs via
  separate `find_unsynced` → send → `mark_hive_synced_batch` cycles (`hive_sync.rs:322,454`). Hive
  applies via `SharedTaskRepository::upsert_from_node()` (`remote/src/db/tasks.rs:558`).
- **Hive → other nodes (TWO channels):** (a) **Electric SQL shape poll** —
  `ElectricTaskSyncService::sync_project_tasks()` (`electric_task_sync.rs:164`) → `/v1/shape` →
  `Task::upsert_remote_task()` (`db/.../task/sync.rs:253`); (b) **WebSocket activity stream** —
  `project_watcher_task()` (`share.rs:459`) → `ActivityProcessor::process_event()`
  (`share/processor.rs:57`).
- **Schemas defined entirely separately** — 25 SQLite DDL files vs 19 Postgres DDL files, no shared
  source of truth. Local/Hive schema drift is structurally possible and unenforced.

### 2.4 Conflict / ordering model — fragile

- **Hive is central authority, field-level last-write-wins, no merge.** `upsert_from_node`
  (`tasks.rs:585-595`) uses `ON CONFLICT (source_node_id, source_task_id) DO UPDATE` and stamps
  `version = shared_tasks.version + 1`, **ignoring the version the node sends** (node hardcodes
  `version: 1`, `hive_sync.rs:289`). So Hive's version is a monotonic *delivery* counter, not a
  causal/conflict version. Concurrent edits from two nodes overwrite field-by-field; last POST wins.
  No vector clocks, no op-log, no three-way merge. *(This is a deliberate centralized-authority
  design for the round-trip — the real defect is the lack of merge/ordering, not the versioning
  itself.)*
- **Node accepts Hive state via a version gate** (`sync.rs:300`,
  `WHERE excluded.remote_version > tasks.remote_version`) — works for the normal round-trip but
  gives **zero protection for concurrent local edits**: an in-flight local edit can be silently
  clobbered by an arriving Hive update (exactly what `fix_is_remote_for_local_projects` repairs).
- **Delete semantics diverge by channel:** Electric delete actually deletes the local task; the
  activity-stream path only clears `shared_task_id` (`processor.rs`). Same logical event, two
  outcomes — orphaning risk.

### 2.5 Offline → reconnect behavior

- Catch-up is cursor/offset-based, per channel, best-effort (Electric `ShapeState (handle, offset)`;
  activity stream `shared_activity_cursors.last_seq`, `share.rs:507`).
- **Silent write-loss window:** outbound dirty flags (`hive_synced_at IS NULL`) are cleared **after
  send attempt, not after Hive durable ack** (`hive_sync.rs:303`). A node that writes locally then
  crashes before send, or drops after `mark_hive_synced` but before Hive commits, **loses the write
  silently** — there is no end-to-end ack handshake.
- `BackfillService` (`remote/src/nodes/backfill.rs`) has a 5-min-timeout pending-request tracker but
  is Hive-driven and attempt/log-focused; **there is no full anti-entropy reconciliation sweep** that
  compares node vs Hive state and heals divergence. The `reset_*` migrations *are* the recovery
  mechanism today.

### 2.6 Recommendation — fix the reconciliation contract (scope is a Phase-2 design decision)

> **Direction decided (2026-06-26):** rebuild the hive as a **hub-and-spoke central management layer**
> — nodes report up via a durable ordered ack'd outbox and manage only their own local work; the hive
> owns the global board and assigns work down with lease/atomic-checkout. This is the rebuild bracket
> below, made well-defined by a known-good topology, and is sequenced **after** the node is correct
> standalone. The §2.7 check is **redirected** from "targeted vs rebuild" to the **migration inventory**
> (what state lives only in the hive). See workstreams `vk-swarm-node-foundations` (first) and
> `vk-swarm-hive-redesign`.

**Keep:** local SQLite as node-of-record, Postgres as Hive store, WebSocket transport.
**The defects are architectural and must be fixed at the design level — but the *size* of the fix is
the open question (§2.7).** Two bracketing options:

- **Targeted (lighter):** make attempt-sync **wait on its parent task's `shared_task_id`** (close the
  cross-entity ordering hole at its source), add **end-to-end ack'd delivery** (Hive confirms durable
  commit before the node clears its dirty flag), and unify the **one delete semantic / one conflict
  policy** across the two inbound channels. Days–low-weeks if the per-entity push cycles can carry an
  ordering guard without restructuring.
- **Full rebuild (heavier):** collapse the ~5 independent per-entity push paths + 2 inbound channels
  into **one ordered, ack'd change-feed** — a per-node monotonic op-log with idempotency keys — so
  cross-entity ordering and exactly-once apply are guaranteed *by construction*. Weeks.

The migration evidence proves corruption *happened*; it does not prove the minimal fix. Resolving
§2.7 decides which bracket Phase 2b commits to. Migration consolidation has hygiene value but touches
**none** of these failure modes, so it must **not** gate Phase 2 as a "fix."

### 2.7 Open verification items — **these bound Phase 2b scope; resolve in Phase 2 design before committing to a rebuild**

The verdict (defects are real and architectural) is settled. The following **decide whether 2b is a
targeted ordering-guard fix or a full reconciliation-contract rebuild**, and must be answered in code
(not from migration comments) before 2b's scope hardens:

- **Cross-entity ordering dependency (scope-determining):** confirm in the sync code
  (`hive_sync.rs` attempt path, the `find_unsynced` cycles, `task/sync.rs`) whether attempt-sync has
  **zero** ordering dependency on its parent task's `shared_task_id` (→ guardable incrementally) vs
  whether the independence is structural (→ rebuild). The `reset_attempt_sync` migration shows the
  corruption but not the minimal fix.
- **Broadcast fan-out double-delivery (hardening):** trace the Hive→node fan-out
  (`remote/src/activity/broker.rs`, `electric_proxy.rs`) to confirm whether the two inbound channels
  can deliver the *same* logical change twice (informs §2.4).
- **Schema drift (hardening):** diff the `tasks` column set between the SQLite and Postgres DDL trees
  to quantify current drift before building on either channel.

---

## 3. Offline-First + Resumability Gap Analysis — **Verdict: Partial; recovery destroys state**

### 3.1 The core finding

vk-swarm persists an **impressively complete** picture of each run to local SQLite — worktree path,
branch, executor, OS PID, started/completed timestamps, before/after git HEAD, and the agent's
external `session_id` (the resume token). **The raw materials for crash-resumable workstreams are
already on disk.** But the recovery path does the *opposite* of resume: on every server start,
`cleanup_orphan_executions` (`crates/services/src/services/container.rs:239-337`) **batch-marks every
`running` execution_process not owned by the current instance as `failed`** (query at
`crates/db/src/models/execution_process/queries.rs:114-129`) — **no PID liveness check, no re-attach,
no re-spawn.** Because `server_instance_id` is freshly minted each boot (cargo-watch restarts mint new
ones), a crash converts every in-flight agent into a dead `failed` row and flips its task to
`InReview`. The `--resume`/`--fork-session` machinery that *would* make resumption trivial exists but
is wired **only to user-initiated follow-ups**, never to recovery.

### 3.2 EXISTS-vs-NEEDED

| State element | Persisted today? (`file:line`) | Re-attached / re-spawned on restart? | Gap |
| --- | --- | --- | --- |
| Worktree path | **Y** — `task_attempts.container_ref` (`task_attempt.rs:41`), `resolve_container_ref` (`:507`) | **N** — read only to capture after-HEAD then abandon (`container.rs:291-301`) | Path survives; nothing relaunches into it. |
| Branch / target branch | **Y** — `task_attempt.rs:42-43` | n/a (static) | None. |
| Executor identity / launch spec | **Y** — `task_attempts.executor` (`:44`) + full `executor_action` JSON per process (`execution_process/mod.rs:68`) | **N** — read during normal spawn only | The "how to launch" blob is on disk but never replayed on boot. |
| **Transcript / session pointer** | **Y** — `executor_sessions.session_id` (`executor_session.rs:12`), written live from `LogMsg::SessionId` (`container.rs:809-820`); `find_latest_session_id_by_task_attempt` | **N** — consumed only by user follow-ups (`drafts.rs:249-255` → `--resume`, `claude.rs:266-268`) | **The resume token is persisted and usable — recovery just never calls it. Highest-leverage gap.** |
| OS process PID | **Y** — `execution_processes.pid`, `update_pid` (`lifecycle.rs:91`), `find_running_with_pids` (`queries.rs:192`) | **N** — PID used only for process-tree **kill**/inspect, never `kill(pid,0)` liveness | No "is my agent still alive?" check; survivable agents orphaned then killed. |
| Current phase / run status | **Partial** — `execution_process.status` + `run_reason` (`mod.rs:45-60`); `task_attempt_status` enum (`task_attempt.rs:28-35`) | **N** — running→**failed** unconditionally (`queries.rs:114`) | Coarse per-process flag, force-failed. No "resume from phase X." |
| Task graph / last-completed | **Partial** — hierarchy + per-process ordering; restore-boundary logic (`lifecycle.rs:104-144`) | **N** | No first-class workstream/phase-graph object. |
| Queued follow-up prompts | **N — in-memory `HashMap` only** (`local-deployment/src/message_queue.rs:47-62`) | **N** — lost on crash | Needs a DB table. |
| before/after git HEAD | **Y** (`mod.rs:70-72`) + startup backfill (`container.rs:339`) | read-only | Fine for diffing; not a resume lever. |

### 3.3 Restart behavior as-built

`server/src/main.rs:130-138` spawns `cleanup_orphan_executions()` at boot. It (1) runs
`mark_orphaned_as_failed` — a single `UPDATE … SET status='failed' WHERE status='running' AND
(server_instance_id IS NULL OR != ?)` (`queries.rs:118-126`), so after a crash **every in-flight run
fails at once** with no `pid` liveness check; (2) iterates remaining `running` rows, sets
`completion_reason='eof'`, captures after-HEAD best-effort, flips the parent task to `InReview`
(`container.rs:271-334`) **and propagates that to the Hive.** Worktree cleanup
(`cleanup_orphaned_worktrees`, `find_expired_for_cleanup` 72h) is keyed to attempt rows, so it does
**not** immediately nuke a just-crashed run's worktree — the path survives long enough to resume *if*
recovery code existed. A real OS-level orphan agent that outlives the server keeps running detached
and untracked until a later kill sweep. The "re-attach or re-spawn-with-context" requirement is
**entirely absent.**

### 3.4 The 3 biggest resumability gaps Phase 2 must close

1. **Recovery force-fails instead of resuming (headline).** `mark_orphaned_as_failed`
   (`queries.rs:114`) blindly fails all non-current-instance running rows. *Primary fix (sound,
   evidence-backed):* before failing, for each running CodingAgent process with a persisted
   `session_id`, **re-spawn** via the existing `spawn_follow_up(--resume <session_id>)` path into
   `container_ref` (`claude.rs:266`). This is the universal mechanism — it does not depend on the
   original OS process surviving. *Optional optimization (unconfirmed):* if the original PID is still
   alive (`kill(pid,0)`/sysinfo), re-attach log streaming instead of re-spawning — but note a child
   whose stdout was piped to the dead server typically dies on EPIPE, and even if it survives its
   stdin control channel is gone, so re-attach can observe but not necessarily drive it. **Confirm
   the executor I/O survival model before relying on re-attach;** do not let it gate the headline fix.
2. **No crash-recovery use of the persisted resume token.** `session_id` is durably stored
   (`container.rs:809`) and a working `--resume/--fork-session` relaunch exists (`claude.rs:266`),
   but reachable only via user follow-ups. *Fix:* add a `resume_attempt(attempt_id)` orchestration
   entry reusing `find_latest_session_id_by_task_attempt` + the follow-up spawn, callable from boot
   recovery.
3. **Queued prompts are volatile.** `message_queue.rs` is a pure in-memory `HashMap`. *Fix:* back it
   with a `queued_messages` SQLite table keyed by `task_attempt_id`, drained on resume.

### 3.5 Offline-first assessment — healthy

Local state is genuinely authoritative and Hive-independent for the run/workstream core: Hive linkage
is additive, nullable metadata (`hive_synced_at`, `hive_assignment_id`, `origin_node_id` are all
`Option`, `task_attempt.rs:51-60`; `execution_process/mod.rs:85-91`); the node runner only starts
when `VK_HIVE_URL`+`VK_NODE_API_KEY` are present (`lib.rs:258-321`), "standalone mode" otherwise.
**Losing the Hive loses no local run state.** One wrinkle: the orphan sweep pushes a `failed`/
`InReview` shared-task update to the Hive *before* any resume logic could correct it
(`container.rs:317`) — a crash broadcasts a wrong "failed" state outward. This is a Phase-2 ordering
concern, not a durability loss.

---

## 4. Keep / Rebuild / Replace Decisions

| Subsystem | Decision | Rationale |
| --- | --- | --- |
| Crate structure / layering | **KEEP** | Clean acyclic DAG; no change needed. |
| Executor abstraction | **KEEP** (refactor `claude.rs`/`codex.rs` god-files) | Strongest design; product core. |
| Server / routes / MCP | **KEEP** | Well-partitioned; consistent envelope. |
| Local SQLite storage model | **KEEP** | Offline-first foundation works and is authoritative. |
| Crash recovery / resumability | **BUILD (new)** | Materials exist; recovery logic must be replaced to re-attach/re-spawn instead of force-fail (§3.4). |
| Message queue (follow-up prompts) | **REBUILD** (persist to SQLite) | In-memory only; lost on crash. |
| Sync / reconciliation contract | **REBUILD — hub-and-spoke** (decided 2026-06-26) | Architectural defects (§2.6); rebuild as central-hive op-log + assignment leases. Workstream `vk-swarm-hive-redesign`. |
| Node registry | **REBUILD-WITH-HIVE** | Subsumed by the hub-and-spoke hive redesign; nodes report up, hive owns the registry/board. |
| Data model — sync columns / migrations | **CONSOLIDATE** (hygiene; **non-gating**) | Churn is a symptom; stabilize *after* the rebuild settles the model. |
| Frontend real-time stack | **CONSOLIDATE** (pick one: Electric SQL vs WS streams) | Mid-migration fragility; retire one paradigm. |
| Upstream stability fixes | **FORWARD-PORT** | ACP bounded channels, WAL panic supervision, npm vuln gate (§5). |

---

## 5. Old vibe-kanban Base Assessment

**Fork point:** Shared git roots back to `563994934` ("Init", 2025-06-14); last shared SHA
`eca26240`. Code history forked **June 2025**; the two trees **architecturally diverged ~late
Nov/early Dec 2025** — both share migrations through `20251129155145_drop_drafts_table.sql`, then
upstream went `20251202000000_migrate_to_electric.sql` while vk-swarm went
`20251203000000_create_cached_nodes.sql`. Upstream is ~2,277 commits ahead of the fork point.

**Divergence summary:**

| Area | Upstream | vk-swarm | Divergence |
| --- | --- | --- | --- |
| Core entity model | `workspaces` + `sessions` (task_attempts split, `20251216142123`) | `task_attempts` | HIGH |
| Crate count | 28 (incl. `relay-*` ×8, `workspace-manager`, `git`, `tauri-app`, …) | 8 | HIGH |
| Multi-node / sync | `relay-*` (SPAKE2/Ed25519 cloud relay) | `crates/remote` Hive + Electric-style sync | HIGH (parallel approaches) |
| Frontend | pnpm monorepo (`local-web`, `remote-web`, `web-core`, `ui`) | single `frontend/` | HIGH |
| Executor types | adds `claude_terminal` (tmux), `opencode`, `qa_mock`, `executor_discovery`, `model_selector` | adds `session_index.rs` | MED |
| ACP channels | bounded + backpressure + WAL panic supervision | **unbounded** (`UnboundedSender`) | MED (stability) |
| Git ops | dedicated `crates/git` (git2 + CLI hybrid, multi-repo) | `services/git.rs` single file | MED |

**Forward-port (Priority 1 — stability/security):**
- **ACP unbounded→bounded channels** — `crates/executors/src/executors/acp/client.rs` uses
  `mpsc::UnboundedSender<AcpEvent>`; upstream fixed (`45fdc0d78`, 2026-05-11) with bounded channels +
  drop-on-full for transcript events. **Memory-growth/OOM risk under sustained load.**
- **WAL monitor panic supervision** — wrap the `crates/db/src/wal_monitor.rs` background task in
  `supervised_run()` so panics are logged, not silently dropped.
- **npm runtime-vuln gate** — upstream `scripts/check-npm-runtime-vulns.mjs` + `pnpm.overrides`
  pinning `preact >=10.27.3`, `devalue >=5.6.4`, `fast-uri >=3.1.2`; vk-swarm has no equivalent CI
  check.

**The "workspaces" feature specifically:** upstream split `task_attempts` into a persistent
*workspace* (branch/worktree, multi-repo) + per-run *sessions*. The **rename is not needed** —
vk-swarm's `task_attempts` + Hive model is coherent. But two *concepts* are worth forward-porting
independently: (a) **multi-repo per workspace** (`workspace_repo` table / `WorktreeContainer`) — a
genuine capability gap *if* cross-repo tasks are ever needed; (b) the **sessions split** (multiple
executor runs per context) — decouples workspace lifespan from individual executor run and aligns
naturally with the durable-workstream object Phase 2 introduces (§3.4 #2, §6).

**Rebase verdict: stay-put + forward-port selectively. Full rebase is infeasible** — migration
sequences diverged at ~#49; upstream's `task_attempts→workspaces/sessions` rename collides with
vk-swarm's Hive tables (`shared_tasks`, `cached_nodes`, `hive_sync_tracking`); upstream's frontend
monorepo vs our flat `frontend/` is a near-total rewrite; relay vs Hive conflict at every
network-boundary touchpoint. Cherry-pick the 3 stability items (2–4 files each, clean patches).

**Top 3 frozen-base risks:** (1) ACP memory growth under sustained load; (2) silent WAL-monitor
crashes losing the DB-integrity signal; (3) npm runtime vulnerabilities shipping unchecked.

---

## 6. Phase 2 Remediation Spec (scoping)

**Goal:** make the server/node foundations **durable, offline-first, and crash-resumable** — the
prerequisite all later phases (P3–P8) build on. Phase 2 is **staged by cost/leverage asymmetry**: the
resumability win is cheap and high-leverage; the sync rebuild is the heavy architectural lift.

> **Decision (2026-06-26) — coordination topology + sequencing.** Phase 2 is split into two sequenced
> workstreams that supersede the 2a/2b sketch below: **(1) `vk-swarm-node-foundations`** (ships first)
> — make the node correct/durable/crash-resumable standalone, and **strip its web UI back to local-only
> CRUD + read-only hive-sync visibility** (removing remote-state display); **(2) `vk-swarm-hive-redesign`**
> (after the node is 100%) — rebuild the hive as a **hub-and-spoke central management layer**. Beyond
> the sync items below, the hive design must add three things topology does *not* give for free:
> a **single ordered per-node outbox/op-log** (ordering), an explicit **`task.status` state machine**
> (hive-authored vs node-reported transitions), and **lease/atomic-checkout assignment** (no
> double-execution on partition). The §2.7 check is redirected to a **hive-only-state migration
> inventory**. Items 1–5 below ≈ workstream 1; items 6–10 ≈ workstream 2.

### Phase 2a — Durable resumability + stability (fast, high-leverage; mostly wiring existing capabilities)

1. **Crash-resumable recovery (the #1 pain).** Replace the force-fail boot sweep
   (`container.rs:239`, `queries.rs:114`) with a recovery routine that, per running CodingAgent
   process with a persisted `session_id`, **re-spawns** via the existing
   `spawn_follow_up(--resume <session_id>)` into `container_ref` (the primary, evidence-backed
   mechanism — independent of OS-process survival); only mark `failed` when no `session_id` exists.
   Add a `resume_attempt(attempt_id)` orchestration entry (reuses
   `find_latest_session_id_by_task_attempt`). *Stretch optimization, pending I/O-survival
   confirmation:* re-attach to a still-live PID instead of re-spawning (§3.4 #1).
2. **Durable workstream-state object.** Promote the run from a coarse per-process `status` to a
   first-class persisted workstream record (worktree path, current phase, task graph, last-completed
   task, transcript/session pointer) — the unit recovery resumes and the consulting/management layer
   (P3, P6) consumes. This is where the upstream **sessions** concept (§5) is the right model to
   forward-port.
3. **Persist the message queue.** Replace the in-memory `HashMap` (`message_queue.rs`) with a
   `queued_messages` SQLite table keyed by `task_attempt_id`, drained on resume.
4. **Forward-port stability fixes.** ACP bounded channels; WAL monitor panic supervision; npm
   runtime-vuln gate.
5. **Fix the crash→Hive ordering wrinkle.** Don't broadcast `failed`/`InReview` to the Hive before
   recovery runs (§3.5).

### Phase 2b — Sync fix (architectural lift; gates multi-node correctness; **scope decided by §2.7**)

> **First action of 2b: resolve the §2.7 ordering check** to choose between the targeted and rebuild
> brackets below. Items 7–9 apply to **both** brackets; item 6 is the rebuild-only step.

6. **(Rebuild bracket only) Single ordered, ack'd change-feed.** Collapse the ~5 per-entity push
   paths + 2 inbound channels into one per-node monotonic op-log with idempotency keys; cross-entity
   ordering (parent-before-child) and exactly-once apply guaranteed by construction.
   **(Targeted bracket alternative:** make attempt-sync wait on parent `shared_task_id` and add an
   ordering guard to the existing per-entity cycles.)
7. **End-to-end acknowledgment.** Hive confirms durable commit before the node clears its dirty flag
   — closes the silent write-loss window (`hive_synced_at`).
8. **One conflict policy, one delete semantic**, applied uniformly across channels.
9. **Anti-entropy reconciliation sweep** to replace the manual `reset_*` migrations as the divergence
   recovery mechanism.
10. **Schema/migration consolidation** (hygiene, after the model settles — explicitly **not** a
    prerequisite for 6–9).

### Sequencing & dependencies

- **2a is independent of 2b** and should ship first (delivers the #1 motivating pain fix quickly,
  de-risks every demo). 2b is required before scaling multi-node usage but does not block single-node
  resumable workstreams.
- Resolve the §2.7 items at the start of 2b — the **cross-entity ordering check first** (it picks
  the targeted-vs-rebuild bracket), then the fan-out double-delivery and SQLite↔Postgres column-drift
  checks to finalize the chosen design.
- Frontend real-time consolidation (§4) can proceed in parallel with either stage.

### Exit criteria for Phase 2

- Kill `-9` the node mid-run → on restart the agent is **re-attached or re-spawned with its prior
  session context** in the correct worktree (no manual archaeology). Queued prompts survive.
- A node that writes offline and reconnects has **zero silent write loss** (verified by ack'd
  delivery), and divergence self-heals via the reconciliation sweep (no `reset_*` migration needed).
- The 3 upstream stability fixes are merged; CI enforces the npm vuln gate.

---

## Appendix — Key files to start any Phase 2 verification

- Sync: `crates/services/src/services/hive_sync.rs`, `electric_task_sync.rs`, `share/publisher.rs`,
  `share/processor.rs`; `crates/remote/src/db/tasks.rs:558`; `crates/db/src/models/task/sync.rs:253`.
- Recovery/resumability: `crates/services/src/services/container.rs:239-337,809-820`;
  `crates/db/src/models/execution_process/queries.rs:80-202`, `lifecycle.rs`;
  `crates/db/src/models/task_attempt.rs`; `crates/db/src/models/executor_session.rs`;
  `crates/executors/src/executors/claude.rs:252-282`;
  `crates/local-deployment/src/message_queue.rs`; `crates/server/src/main.rs:126-149`.
- Executor core: `crates/executors/src/executors/mod.rs:101,235`; `logs/normalizer.rs:20`.
- Migrations (churn evidence): `crates/db/migrations/` — `grep -E 'reset|cache|sync|drop'`.
- Upstream forward-ports: `crates/executors/src/executors/acp/client.rs`;
  `crates/db/src/wal_monitor.rs`.
