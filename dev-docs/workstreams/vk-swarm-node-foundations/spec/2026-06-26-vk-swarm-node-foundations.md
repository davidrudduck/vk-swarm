---
doc_type: spec
status: shipped
workstream: vk-swarm-node-foundations
change_kind: behaviour
---

# vk-swarm-node-foundations — Make the local node correct, durable, and crash-resumable (Phase 2a)

> **Child of** [`vk-swarm-refactor`](./2026-06-25-vk-swarm-refactor.md). Ships **first**; the
> [`vk-swarm-hive-redesign`](./2026-06-26-vk-swarm-hive-redesign.md) workstream depends on a node that
> works 100% standalone.
>
> **Analysis basis (do not duplicate):** evidence + file:line citations in
> [`docs/specs/2026-06-25-vk-swarm-phase1-analysis.md`](../../specs/2026-06-25-vk-swarm-phase1-analysis.md)
> (§3 resumability, §3.5 offline-first, §5 forward-ports). This PRD captures *intent*; `/wai:spec`
> adds design.

## Intent (what / why)

Make a single vk-swarm node **fully correct, durable, and crash-resumable on its own — with or without
a hive.** This is the foundation everything else builds on, and it is deliberately sequenced **before**
the hive redesign: a node that owns its work authoritatively is the clean source a rebuilt hive
re-ingests later, and it delivers standalone value immediately.

Two things define "done" for the node: (1) the program's #1 pain — **resumability is lost on any
system hiccup** — is eliminated by re-attaching/re-spawning in-context on restart (Phase 1 found the
durable materials already exist on disk; recovery just throws them away); and (2) the node is
**simplified** — its web UI is stripped back to manage only *local* work plus *read-only* visibility
into hive sync status, removing the multi-node display/management complexity that contributed to the
§2 sync fragility.

The node↔hive *sync contract* (ordered outbox, assignment leases, conflict policy) is **explicitly out
of scope here** — it is defined by the new hive and lives in `vk-swarm-hive-redesign`. This workstream
may keep, disable, or leave the current sync mechanism as-is; correctness of the node does not depend
on it.

## Users / who is affected

- **The operator running coding agents** (the `/wai`+`/dr` remote-control workflow) — relieved of
  manual crash recovery; gets a simpler, local-focused UI.
- **Future headless service-nodes** — which live or die on rock-solid node-local durability and
  resumability (this workstream is their prerequisite).

## Success criteria

- SC1: After `kill -9` of a node mid-run, on restart the agent is **fenced then re-spawned with its
  prior session context** (`--resume <session_id>`) in **the correct worktree** (`container_ref`), with
  **no manual intervention** and **no second concurrent writer** in the worktree. *(Clauses: SC1a fence
  the prior process — confirm the original agent PID is dead, using pid + fingerprint to defeat PID
  reuse, before relaunch; SC1b re-spawn-with-session; SC1c correct worktree; SC1d no manual step.)*
  See [ADR-0001](../../../dev-docs/adr/0001-crash-recovery-fence-then-resume.md).
- SC1-fallback: For executors that do **not** support session resume, recovery applies the defined
  fallback policy (cold re-spawn from `executor_action` where safe, else mark-failed-and-surface) — SC1
  is not silently Claude-only. *(The per-executor resume-capability audit is a decompose task.)*
- SC2: Queued follow-up prompts **survive a restart** — persisted to local SQLite and drained on
  resume (no longer an in-memory `HashMap`).
- SC3: A **queryable durable workstream-state surface** exists (worktree path, current phase via the
  ordered `execution_processes` chain, last-completed step, transcript/session pointer) that recovery
  resumes from and downstream phases can query — delivered by **extending** existing tables plus an
  explicit assembling **view**, not a new run entity. See
  [ADR-0003](../../../dev-docs/adr/0003-durable-local-state-schema.md). *(Clauses: SC3a resume-intent
  marker on `execution_processes`; SC3b read-only assembling view over attempts+processes+sessions.)*
- SC4: A **local-durability audit** confirms every run/management state element is durably persisted
  locally with no gaps (the message-queue fix in SC2 is the first known hole closed).
- SC5: The node web UI **manages only locally-created/locally-run** projects, tasks, attempts, and
  executions, **plus read-only visibility into hive sync status/configuration** — it no longer renders
  or manages remote (other-node) state. Visibility discriminator: **created on this node OR has a local
  `task_attempt`** (so hive-assigned work this node is *running* is NOT hidden — the naive
  `shared_task_id IS NULL` filter is rejected). Sync plumbing and `remote_*`/`shared_task_id` columns
  are left intact. See [ADR-0002](../../../dev-docs/adr/0002-node-ui-local-only.md). *(Clauses: SC5a
  local-or-run visibility discriminator; SC5b read-only hive-sync view; SC5c remote-state management
  removed; SC5d sync plumbing untouched.)*
- SC6: A single node build runs **fully standalone (no hive)** with its local UI **always available**,
  and also as a hive-connected node — the UI is never disabled.
- SC7: The upstream stability/testability forward-ports are merged and verified — **ACP bounded
  channels** (`harness.rs:259`/`client.rs:10`, drop-on-full per the Decision), **WAL-monitor panic
  supervision** (`wal_monitor.rs:138`), an **npm runtime-vuln CI gate**, and the **`qa_mock` executor**
  (absent today; required to test crash-resume — see Test strategy). *(Clauses: SC7a ACP; SC7b WAL;
  SC7c npm gate; SC7d qa_mock.)*
- SC8: Crash recovery (fence-then-resume) runs **before** any failure-marking; a run that is resumable
  is **never** marked `failed` or broadcast as `failed`/`InReview`, and only genuinely-unrecoverable
  runs are marked failed (and only then propagated outward). *(This corrects the current blanket-fail
  sweep, which is also dead-on-crash today — see ADR-0001.)*

## Constraints

- **Keep the storage core:** local SQLite as node-of-record; do not replace storage. The node's local
  schema may *shrink* as remote-state display is removed.
- **Primary recovery mechanism is re-spawn-with-`--resume`** (evidence-backed; independent of OS-process
  survival). Re-attach is optional/unconfirmed.
- **The node must remain fully functional standalone** — losing or disabling the hive loses no local
  capability (§3.5 confirms local state is already authoritative).
- **Do not design the node↔hive sync contract here** — it belongs to `vk-swarm-hive-redesign`.
- **Forward-port, do not rebase** the 3 upstream stability fixes.
- **GitHub targeting:** PRs only against `davidrudduck/vk-swarm`.

## Out of scope

- **The node↔hive sync redesign** (ordered outbox/op-log, end-to-end ack, assignment leases, status
  state machine, anti-entropy) — `vk-swarm-hive-redesign`.
- **The central hive UI / cross-node management** — `vk-swarm-hive-redesign`.
- **Schema/migration consolidation as a gating fix** — hygiene only, deferred.
- **Frontend real-time stack consolidation** beyond what stripping remote-state display naturally
  removes — a separate concern.

## Approach

Make the node correct standalone by changing four areas, none of which touch the node↔hive sync
*protocol* (that is `vk-swarm-hive-redesign`): (1) **recovery** — replace the blanket-fail boot sweep
with fence-then-resume; (2) **durability** — persist the message queue and add a resume-intent marker;
(3) **UI scope** — filter the read/API layer to local work and delete remote-display frontend; (4)
**forward-ports** — three upstream stability fixes plus the `qa_mock` executor for testability. All
change-points were verified against current code (file:line below). The work is independently
shippable and delivers standalone value before any hive work.

## Design / architecture

**1. Fence-then-resume recovery (SC1, SC8 · ADR-0001).** Rewrite
`cleanup_orphan_executions` (`crates/services/src/services/container.rs:239-337`) so that, *before*
`mark_orphaned_as_failed` (`crates/db/src/models/execution_process/queries.rs:114`), it iterates
`running` CodingAgent rows and: **fences** — checks the stored `pid` (`find_running_with_pids`,
`queries.rs:192`) for liveness via sysinfo/procfs **plus a start-time/pgid fingerprint** (PID-reuse
guard) and terminates the process group if alive; **resumes** — reconstructs the `ExecutorAction` from
`execution_processes.executor_action` (carries executor profile + prompt) + `session_id`
(`executor_sessions`, via `find_latest_session_id_by_task_attempt`) and re-enters
`start_execution_inner` (`crates/local-deployment/src/container.rs:1617`, the same path
`coding_agent_follow_up.rs:53` uses for `--resume`); **fails last** — only rows with no session/
non-resumable executor are marked failed and propagated (the outward push is
`share_publisher().update_shared_task_by_id`, `container.rs:318`). The blanket query is narrowed to
truly-abandoned rows.

**2. Durable message queue (SC2 · ADR-0003).** Add a `queued_messages` table (`id`, `task_attempt_id`,
`content`, `variant`, `position`, `created_at`) and back `MessageQueueStore`
(`crates/local-deployment/src/message_queue.rs:49`) with it — `add`/`remove`/`clear`/`peek_next`
become DB-backed. On boot, attempts with queued rows re-trigger the existing drain
(`try_consume_queued_message`, `container.rs:1179`, fired at `:738`).

**3. Durable workstream-state surface (SC3 · ADR-0003).** Add a resume-intent column to
`execution_processes` (migration) consumed by recovery, and an explicit read-only view joining
`task_attempts` + ordered `execution_processes` + `executor_sessions`. No new run entity — the triple
already encodes worktree/branch/executor/action/status/pid/HEAD/session/ordering.

**4. UI strip-back (SC5, SC6 · ADR-0002).** Apply the visibility discriminator (created-here OR
has-local-`task_attempt`) in `Task::find_by_project_id_with_attempt_status`
(`crates/db/src/models/task/queries.rs:15`); **remove** the request-time remote merge in `get_tasks`
(`crates/server/src/routes/tasks/handlers/core.rs:97-173`); remove node-surface proxies (`/api/nodes*`,
`/api/swarm/*`, `/api/merged-projects`, available-nodes/stream-connection-info); **delete** frontend
remote surfaces (`pages/Nodes.tsx`, `components/nodes/*`, `NodesContext`, `lib/api/nodes.ts`, navbar
entry, remote badges, `useMergedProjects`, remote stream hooks). Build the read-only hive status view on
`GET /api/database/sync-status` (`crates/server/src/routes/database.rs:295`) extended with
`hive_url`/`node_name`/last-synced; repurpose `SwarmSettings` read-only. **SC6 needs no code** — the UI
is already served unconditionally (`crates/server/src/routes/mod.rs:82-86`); introduce no headless flag.
**Touch no `upsert_remote_task` writer, publisher, WS runner, or `remote_*`/`shared_task_id` column.**

**5. Forward-ports (SC7).** ACP `mpsc::unbounded_channel::<AcpEvent>()` (`acp/harness.rs:259`,
`acp/client.rs:10`) → bounded `channel(N)` with drop-on-full for transcript events (per Decision D6);
wrap `WalMonitor::spawn`'s `tokio::spawn(monitor.run(rx))` (`crates/db/src/wal_monitor.rs:138-153`) in a
panic-supervising/restart wrapper; add an npm runtime-vuln CI gate (port upstream
`scripts/check-npm-runtime-vulns.mjs` + `pnpm.overrides`); forward-port `qa_mock.rs` into
`crates/executors/src/executors/` (present upstream, absent here).

## Decisions

- **D1 — Recovery is fence-then-resume**, ordered before failure-marking; re-attach-to-live-PID rejected
  as primary. *(Irreversible: boot-contract + migration.)* → [ADR-0001](../../../dev-docs/adr/0001-crash-recovery-fence-then-resume.md)
- **D2 — Extend existing tables + a view; add only `queued_messages`.** No new run entity. *(Irreversible:
  schema.)* → [ADR-0003](../../../dev-docs/adr/0003-durable-local-state-schema.md)
- **D3 — Strip the node UI via the read/API layer + frontend deletes only** → [ADR-0002](../../../dev-docs/adr/0002-node-ui-local-only.md), leaving sync plumbing and
  `remote_*`/`shared_task_id` columns intact. *(Irreversible: deletes code, changes `get_tasks` shape.)*
- **D4 — Visibility discriminator = created-locally OR has-local-`task_attempt`.** Exact SQL predicate
  confirmed at decompose against how assignment populates rows.
- **D5 — Executor resume-capability fallback policy:** resumable executors use `--resume`; non-resumable
  ones get cold re-spawn-from-`executor_action` where safe, else mark-failed-and-surface. Per-executor
  audit is a decompose task (SC1-fallback).
- **D6 — ACP bounded channels with drop-on-full** for transcript events → [ADR-0004](../../../dev-docs/adr/0004-acp-bounded-channels.md) (backpressure for control) — a
  faithful forward-port of upstream's already-vetted change; lossy-delivery semantic documented in the
  ADR. Does not apply to control-flow channels.
- **D7 — Single-node assumption named:** `mark_orphaned_as_failed` matches other nodes' rows too;
  acceptable here, **flagged for `vk-swarm-hive-redesign`** to revisit with ownership/lease.

## Test strategy

- **`qa_mock` executor is the keystone (SC7d):** crash-resume cannot be tested against a live agent CLI.
  Forward-port `qa_mock` so a deterministic agent can be killed mid-run; the canonical SC1 test is
  *spawn → `kill -9` the mock + node → restart → assert the prior PID is fenced and a single re-spawn
  with `--resume` (or the fallback) occurs in the same worktree, with no second writer.*
- **Recovery unit/integration:** classification (resume vs abandon) over seeded `execution_processes`
  rows; liveness+fingerprint logic incl. a PID-reuse case; assert resumable runs are never marked
  `failed` (SC8). Use `db::test_utils::create_test_pool()`.
- **Durability:** `queued_messages` persists across a simulated restart and drains in `position` order
  (SC2); resume-intent marker round-trips; assembling view returns the expected joined shape (SC3).
- **UI scope:** API tests assert a locally-created task and a hive-assigned task *with a local attempt*
  are both visible, while a remote task with no local attempt is not (SC5/D4); frontend build passes
  with remote surfaces removed.
- **Forward-ports:** ACP bounded-channel drop-on-full under flood (no unbounded growth); WAL monitor
  survives an injected panic (SC7b); npm gate fails CI on a seeded advisory (SC7c).
- **Manual smoke:** `kill -9` the dev node mid-run, restart, observe in-context resume (the #1-pain
  acceptance demo).
