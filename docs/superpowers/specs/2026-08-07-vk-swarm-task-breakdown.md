---
doc_type: spec
status: draft
workstream: vk-swarm-task-breakdown
change_kind: behaviour
---

# vk-swarm-task-breakdown — AI task breakdown harness (P3 / SC3)

Phase 3 of the `vk-swarm-refactor` program
(`docs/superpowers/specs/2026-06-25-vk-swarm-refactor.md`). Owns umbrella success criterion
**SC3**: "A high-level goal can be turned by an AI harness into correctly-scoped,
independently-executable subtasks."

## Intent (what / why)

Give the operator a kanban-driven way to turn a high-level goal card into a reviewed set of
correctly-scoped, independently-executable child tasks — produced by an AI harness running
**node-local through the existing executor abstraction** (the same `enum_dispatch` CLI-agent
layer that runs task attempts today).

Why now: P1/P2 delivered a durable, offline-first, crash-resumable node + hive substrate. The
next bottleneck to actually *managing development work cycles in vk-swarm* is that decomposing
work is still manual and lives outside the board (chat sessions, WAI docs). P3 closes that gap
and is — together with P4 (event bus) — the gate for P5 (conflict/priority/dependency
automation), P6 (AI management agent), and P7 (MCP/ACP connectivity).

Decisions settled at intent time (interview 2026-08-07):

- **Placement: node-local, executor-driven.** Breakdown runs on the node that owns the
  project, delegated to a configured executor (e.g. Claude Code CLI). No hive-side model
  access, no new infra; the hive observes resulting tasks through normal sync.
- **Triggers: all three, phased.** (1) A "Break down" action on a task card in the node UI —
  the flagship path; (2) an MCP tool on the node's existing MCP server so external agents can
  invoke the same harness (paves P7); (3) an automatic trigger on task creation behind a
  per-project opt-in heuristic (size/label based — heuristic defined at `/wai:spec` time).
- **Output shape: native child tasks + dependency metadata.** Proposals materialize as real
  vk-swarm tasks under the parent (existing task hierarchy), each carrying description, scope
  notes, and inter-task dependency edges persisted queryably — the substrate P5/P6 consume.
- **Human-in-the-loop: review-then-accept.** Every trigger (card action, MCP, auto) produces a
  *proposal set* the operator reviews, edits, prunes, and accepts in the UI before subtasks
  land as actionable board tasks. Nothing becomes ready work without acceptance.

## Users / who is affected

- **Primary — the operator** driving multiple coding agents across nodes (today `/wai` + `/dr`
  with Claude Code): gains in-board decomposition instead of out-of-band chat/docs planning.
- **External agents** (MCP clients, later the P7 MCP/ACP fabric): gain a programmatic
  breakdown entry point with identical semantics to the UI path.
- **Downstream phases as consumers:** P5 needs the dependency edges this produces; P6 needs
  correctly-scoped ready tasks to select from; P4's bus will carry this feature's lifecycle
  events (proposal created / accepted) once both exist.

## Success criteria

Runtime-observable on a running node (not "test X passes"):

- **SC-A (card trigger → proposal).** From the node kanban UI, invoking "Break down" on a task
  whose description states a multi-part goal yields, on that same board, a proposal set of ≥2
  subtasks each with a title, a scope description, and any dependency edges — visibly marked
  as *proposed*, not yet actionable.
- **SC-B (review gate).** Proposed subtasks can be edited, deleted, and re-ordered; no
  proposed subtask is startable (no attempt can be created) and none syncs to the hive as
  ready work before acceptance.
- **SC-C (accept → native tasks).** Accepting the (possibly edited) set creates real child
  tasks under the parent in the existing hierarchy; each can immediately host its own task
  attempt; dependency edges are persisted and queryable via the node API.
- **SC-D (MCP parity).** Calling the breakdown MCP tool against the same task produces the
  same proposal artifact observable via the node API/UI, subject to the same review gate.
- **SC-E (auto trigger, opt-in).** With the per-project auto-breakdown setting enabled,
  creating a task matching the heuristic yields a proposal set (never directly-landed tasks)
  flagged for review; with the setting off (default), task creation behaviour is unchanged.
- **SC-F (offline-first).** SC-A through SC-C succeed with the hive unreachable; after
  reconnect, accepted child tasks reconcile to the hive with zero silent loss (rides SC1
  guarantees).
- **SC-G (executor failure is survivable).** If the executor run fails or emits an unusable
  breakdown, the operator sees a localized error state on the card and can retry; no partial
  child tasks land.

## Constraints

- **Reuse the proven core:** the executor abstraction (`enum_dispatch` over CLI agents),
  local-SQLite-as-node-of-record, existing task parent/child hierarchy, `ApiResponse<T>`
  route patterns, ts-rs typegen (`npm run generate-types`), i18n for all new UI strings.
- **Offline-first is non-negotiable** (umbrella principle 1): no hive round-trip on the
  breakdown path.
- **Node-scoped:** a breakdown operates on one project on one node. Cross-node/global goals
  are a hive concern deferred to later phases.
- **Structured output contract:** the executor must return a machine-parseable breakdown
  (schema defined at `/wai:spec`); free-text parsing heuristics are not acceptable as the
  contract.
- **Reference systems read-only:** paperclip (`/data/Code/reference/agents/paperclip`) for
  governance/decomposition prior art; upstream vibe-kanban for hierarchy patterns.
- **GitHub targeting:** PRs only against `davidrudduck/vk-swarm`.

## Out of scope

- **Hive-side breakdown** or any hive-initiated assignment of breakdown work to nodes.
- **Conflict/priority/dependency *computation and visualisation*** (P5) — this workstream
  only *persists* dependency edges; it does not schedule, rank, or graph them.
- **Autonomous execution of accepted subtasks** (P6) — acceptance creates ready tasks; no
  agent auto-starts them.
- **Event-bus emission** (P4) — proposal/acceptance lifecycle events are designed so P4 can
  carry them later, but no bus integration ships here.
- **Recursive breakdown** (auto-decomposing generated subtasks) — single level only.
- **WednesdayAI integration** (P8).
