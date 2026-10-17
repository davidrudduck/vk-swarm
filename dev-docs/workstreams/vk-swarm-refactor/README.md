---
workstream: vk-swarm-refactor
doc_type: readme
status: draft
title: "vk-swarm orchestration platform — 8-phase program umbrella"
staging_pointers:
  - docs/superpowers/specs/2026-06-25-vk-swarm-refactor.md
---

# vk-swarm-refactor

Program umbrella: evolve vk-swarm into a kanban-driven, offline-first, multi-host coding-agent
orchestration platform across 8 phases. Thin north-star intent; each phase is its own child
workstream. Spec: `docs/superpowers/specs/2026-06-25-vk-swarm-refactor.md` (success criteria
SC1–SC7, dependency graph, constraints).

## Phase → workstream map

Dependency graph: P2 blocks everything; P3 ⟂ P4; P5 needs P3 (+rides P4); P6 consumes P4+P5;
P7 rides P3–P5; P8 trails P7 (optional).

| Phase | Scope | Child workstream(s) | State (2026-09-14) |
|---|---|---|---|
| P1 | Deep analysis | — (spec: `docs/specs/2026-06-25-vk-swarm-phase1-analysis.md`) | ✅ done |
| P2a | Node correct/durable/crash-resumable (SC1 local, SC2) | `vk-swarm-node-foundations` (+ `foundations-followup1`) | ✅ shipped |
| P2b | Hub-and-spoke central hive (SC1 reconcile) | `vk-swarm-hive-redesign` | ✅ shipped |
| P2c | Hive console UI + hardening | `vk-swarm-hive-ui`, `vk-swarm-hive-ui-polish`, `hive-node-api-key-ui`, `fix-nonloopback-signin`, `error-handling-and-dialog-a11y`, `remote-docker-build-fix`, `ui-overhaul` | ✅ shipped |
| P2 tail | UI kit + node-UI cleanup + test-debt revival | `vk-swarm-design-system`, `vk-swarm-node-ui-localize`, `node-ui-localize-followups`, `hive-oauth-sw-bypass`, `node-task-delete-dangling-shared-id`, `worktree-orphan-sweep-guard`; later test-debt drafts retained separately | 🔶 only hive-oauth task 301 live evidence remains |
| P3 | AI task breakdown harness (SC3) | `vk-swarm-task-breakdown` | ✅ shipped (PR #475) |
| P4 | Task-lifecycle event bus (SC4) | `vk-swarm-event-bus` | ✅ shipped (PR #477) |
| P5 | Conflict/priority/dependency automation (SC5; paperclip ref) | future M7 child under `local-node-first-use` | ⏸ paused behind M7 |
| P6 | AI management agent (SC6; consumes P4+P5) | future M7 child under `local-node-first-use` | ⏸ paused behind M7 |
| P7 | MCP/ACP connectivity (SC7; rides P3–P5) | future M7 child under `local-node-first-use` | ⏸ paused behind M7 |
| P8 | WednesdayAI adapter (optional, thin, never the host) | future M7 child under `local-node-first-use` | ⏸ paused behind M7 |

## Recommended order from here

1. Complete M0 tracker truth and the remaining `hive-oauth-sw-bypass` task 301 live evidence.
2. Complete `local-deployment-test-orphan-cleanup-safety` so deterministic M2 tests cannot touch
   operator worktrees or a live Hive.
3. Settle and execute M2 `local-execution-lifecycle-correctness`, then M3
   `opencode-attached-server`, under the governing local-node-first-use roadmap.
4. Certify the M0–M3 first-release gate; resume P5–P8 only through M7 afterward.
