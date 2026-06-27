---
doc_type: spec
status: draft
workstream: vk-swarm-refactor
change_kind: behaviour
---

# vk-swarm-refactor — Program Umbrella (north-star intent)

> **Thin umbrella.** This doc captures the *program* intent and non-negotiables only. Each phase is
> its own workstream with its own settled spec and testable success criteria. Phase 2 is split into
> two sequenced children: [`vk-swarm-node-foundations`](./2026-06-26-vk-swarm-node-foundations.md)
> (the node, to 100% — ships first) then
> [`vk-swarm-hive-redesign`](./2026-06-26-vk-swarm-hive-redesign.md) (hub-and-spoke central hive —
> after the node is solid).
>
> **Analysis basis (do not duplicate):** all architecture/root-cause/gap evidence lives in
> [`docs/specs/2026-06-25-vk-swarm-phase1-analysis.md`](../../specs/2026-06-25-vk-swarm-phase1-analysis.md).
> This umbrella references it; it does not restate findings.

## Intent (what / why)

Evolve vk-swarm from a swarm-aware Vibe-Kanban fork into a full **coding-agent orchestration
platform**: kanban-driven, multi-host, offline-first, that drives multiple CLI agents (Claude Code,
Codex, opencode, Gemini, Copilot) and — eventually — runs entire workstreams end-to-end with an AI
consulting/management layer on top.

This is a **standalone application built on the vk-swarm codebase** — *not* a WednesdayAI extension
and *not* WednesdayAI core. Rationale (settled): a durable kanban/CI orchestrator (persistent board
state, git worktrees, multi-CLI process supervision, offline sync) is a fundamentally different
product shape than WednesdayAI's lean chat/channel assistant, and vk-swarm already implements ~85% of
it. WednesdayAI integration is the **last, optional** phase — a thin adapter, never the host.

The program runs in 8 phases: (1) deep analysis ✅ → (2) durable/offline-first/resumable foundations →
{(3) AI task breakdown ⟂ (4) event bus} → (5) conflict/priority/dependency automation → (6) AI
management agent → (7) MCP/ACP connectivity (rides P3–P5) → (8) WednesdayAI adapter (optional).
Dependency graph: P2 blocks everything; P3 ⟂ P4; P5 needs P3 (+rides P4); P6 consumes P4+P5; P8
trails P7.

## Users / who is affected

- **Primary:** the operator(s) driving multiple coding agents across one or more hosts (today: `/wai`
  + `/dr` running Claude Code in remote-control mode), who currently lose resumability on any system
  hiccup and must manually recover (find worktree, restart CLI cold, no context).
- **Multi-node swarm users** whose runs are corrupted by Hive sync defects (the live-data `reset_*`
  repair migrations are evidence of operator pain).
- **Downstream consumers** of the platform's later phases: the AI breakdown harness (P3) and the AI
  management agent (P6), which require a durable, queryable workstream-state substrate to exist.

## Success criteria

> North-star, program-level. Each is **owned and made testable by a child workstream**; this umbrella
> does not get decomposed into tasks directly. Ids are stable references for the children to cite.

- SC1: Every node keeps durable local state and, after a network outage, reconciles back with **zero
  silent write loss**. *(Local durability owned by `vk-swarm-node-foundations`; the ack'd
  reconciliation that guarantees no-loss owned by `vk-swarm-hive-redesign`.)*
- SC2: A crashed or restarted orchestrator **re-attaches to or re-spawns** every in-flight workstream
  in its correct worktree **with prior agent context** — no manual archaeology. *(Owned by
  `vk-swarm-node-foundations`.)*
- SC3: A high-level goal can be turned by an AI harness into correctly-scoped, independently-executable
  subtasks. *(Owned by P3.)*
- SC4: Task lifecycle changes emit events on an internal/external bus that downstream triggers consume.
  *(Owned by P4.)*
- SC5: Conflicts, workstream priority, and inter-task dependencies are computed and visualised.
  *(Owned by P5.)*
- SC6: An AI management agent consumes bus triggers and selects ready tasks by priority/conflict rank.
  *(Owned by P6.)*
- SC7: MCP/ACP connectivity lets external agents and the WednesdayAI fabric drive and observe runs.
  *(Owned by P7; P8 adapter optional.)*

## Constraints

- **Two non-negotiable design principles (designed in, not bolted on):**
  1. **Offline-first durable local state** — each node keeps a local store of executor runs/tasks and
     reconciles when the network returns. Keep vk-swarm's existing local-SQLite-as-node-of-record
     model; the part that hurt was only the coordination/transport half (the Hive sync).
  2. **Durable resumability** — the orchestrator owns workstream state on disk (worktree path, current
     phase, task graph, last-completed task, agent transcript pointers); a crash means re-attach or
     re-spawn with context, never manual recovery.
- **Keep, don't replace, the proven core:** local SQLite, the crate layering, the executor
  abstraction (`enum_dispatch` over the CLI agents), the server/MCP surface (Phase 1 audit).
- **Coordination topology = hub-and-spoke (decided).** Replace bidirectional multi-master sync with a
  central management **hive** that nodes report up to (durable ordered ack'd outbox) and that assigns
  work down (lease/atomic-checkout) — the proven control-plane shape (cf. paperclip). **Nodes manage
  only their own local work** (always-on local UI, scoped to local CRUD + read-only hive-sync
  visibility); the global board and cross-node management live on the hive. This dissolves the §2
  fan-out/conflict class at the design level.
- **Sequence Phase 2 node-first.** Get a node working 100% standalone (`vk-swarm-node-foundations`)
  *before* rebuilding the hive (`vk-swarm-hive-redesign`); the rebuilt hive re-ingests
  node-authoritative state, which is also the clean migration path.
- **Stay on the current fork base; forward-port selectively.** Rebasing onto upstream vibe-kanban is
  infeasible (diverged ~Dec 2025; upstream rewrote into 28 crates + a workspaces/sessions split).
- **Reference systems are read-only** and cited by exact path: vibe-kanban
  (`/data/Code/reference/other/vibe-kanban`), paperclip (`/data/Code/reference/agents/paperclip`,
  P5/6 governance), WednesdayAI (`src/acp/`, P8 target).
- **GitHub targeting:** open PRs only against `davidrudduck/vk-swarm`.

## Out of scope

- **Re-litigating the standalone-vs-WednesdayAI decision** — settled; WednesdayAI is a last, optional,
  thin adapter (P8), never the host.
- **Adopting upstream vibe-kanban's "workspaces" rename** (`task_attempts → workspaces/sessions`) — our
  model is coherent; only the *sessions* concept and *multi-repo-per-workspace* capability are
  candidate forward-ports, evaluated within their phases.
- **A full rebase onto current upstream.**
- **Designing P3–P8 here** — each is its own workstream; this umbrella only names them and their
  dependency order.
