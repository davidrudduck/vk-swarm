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

| Phase | Scope | Child workstream(s) | State (2026-08-07) |
|---|---|---|---|
| P1 | Deep analysis | — (spec: `docs/specs/2026-06-25-vk-swarm-phase1-analysis.md`) | ✅ done |
| P2a | Node correct/durable/crash-resumable (SC1 local, SC2) | `vk-swarm-node-foundations` (+ `foundations-followup1`) | ✅ shipped |
| P2b | Hub-and-spoke central hive (SC1 reconcile) | `vk-swarm-hive-redesign` | ✅ shipped |
| P2c | Hive console UI + hardening | `vk-swarm-hive-ui`, `vk-swarm-hive-ui-polish`, `hive-node-api-key-ui`, `fix-nonloopback-signin`, `error-handling-and-dialog-a11y`, `remote-docker-build-fix`, `ui-overhaul` | ✅ shipped |
| P2 tail | UI kit + node-UI cleanup + test-debt revival | `vk-swarm-design-system` (✅ PR #466), `vk-swarm-node-ui-localize` (✅ PR #467, verify PASS), `node-ui-localize-followups`, `hive-oauth-sw-bypass` (✅ PR #469; task 301 deploy evidence pending), `node-task-delete-dangling-shared-id` (✅ PR #468; deploy verify pending), `worktree-orphan-sweep-guard`, `remote-services-doctest-revival` (0/32), `terminal-session-pty-tests` (0/5) | 🔶 cleanup tail |
| P3 | AI task breakdown harness (SC3) | `vk-swarm-task-breakdown` (PRD 2026-08-07 — needs `/wai:spec`) | 🔶 intent captured |
| P4 | Task-lifecycle event bus (SC4) | `vk-swarm-event-bus` (PRD 2026-08-07 — needs `/wai:spec`) | 🔶 intent captured |
| P5 | Conflict/priority/dependency automation (SC5; paperclip ref) | _unmapped — after P3_ | ⬜ not started |
| P6 | AI management agent (SC6; consumes P4+P5) | _unmapped_ | ⬜ not started |
| P7 | MCP/ACP connectivity (SC7; rides P3–P5) | _unmapped_ | ⬜ not started |
| P8 | WednesdayAI adapter (optional, thin, never the host) | _unmapped_ | ⬜ not started |

## Recommended order from here

1. `/wai:spec vk-swarm-task-breakdown` → precheck → decompose → execute (P3; first
   "use the board to manage work" capability, gates P5).
2. `/wai:spec vk-swarm-event-bus` → precheck → decompose → execute (P4 ⟂ P3; gates P6/P7).
3. Close the P2 cleanup tail: hive-oauth-sw-bypass task 301 deploy evidence,
   node-task-delete deploy verify, backend/frontend hygiene bundles
   (`worktree-orphan-sweep-guard`, `node-ui-localize-followups`).
4. Burn down test-debt trackers (`remote-services-doctest-revival`,
   `terminal-session-pty-tests`) opportunistically alongside.
