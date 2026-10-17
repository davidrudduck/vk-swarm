---
workstream: local-node-first-use
doc_type: readme
status: active
title: "Local node first use — usable local execution before further orchestration"
staging_pointers:
  - docs/superpowers/specs/2026-08-21-local-node-first-use-roadmap-design.md
---

# local-node-first-use

Umbrella release train that makes the standalone node useful for daily work before resuming the
remaining orchestration refactor. M0–M3 gate the first release: per-browser Hive OAuth, shared
execution-lifecycle correctness, and a certified pre-existing OpenCode server path. Each milestone
is a separate WAI child workstream; this umbrella is not a monolithic decomposition target.

## Milestone state

| Milestone | Child workstream | State |
|---|---|---|
| M0 | this umbrella's tracker/backlog reconciliation | active — task 301 live stalled-flow evidence remains |
| M1 | `local-node-browser-oauth` | shipped and live-verified |
| M2 | `local-execution-lifecycle-correctness` | selected; blocked on M0 and the test-construction safety predecessor |
| M3 | `opencode-attached-server` | not started; consumes M2's settled lifecycle contract |
| M4 | `opencode-direct-cli` | not started; after first release |
| M5 | `claude-cli` | not started; after first release |
| M6 | `automatic-crash-resume` | not started; reuses M2 identity |
| M7 | Hive usability and remaining P5–P8 refactor expansion | paused until the local release train is usable |

## Release and artifact rules

M0–M3 gate the first local-node release. No M1–M3 child closes with live or E2E evidence
outstanding. Each child has one canonical spec and passes independently through intent, design,
precheck, execution, deterministic gates, live evidence, integrated review, and same-workstream
remediation. The umbrella is sequencing authority only and must never be decomposed as one plan.
