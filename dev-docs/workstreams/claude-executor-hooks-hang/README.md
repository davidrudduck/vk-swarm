---
workstream: claude-executor-hooks-hang
doc_type: readme
status: draft
title: "Claude executor initialize hooks payload hangs claude-code"
depends_on: []
adrs: []
staging_pointers:
  - docs/plans/vk-swarm-task-breakdown/decisions-ledger.md
---

# claude-executor-hooks-hang

**Origin:** deploy-verification finding DV-3 during `vk-swarm-task-breakdown` (PR #475). Backlog
finding **F-2026-08-09-01** (high). Pre-existing and node-wide — nothing on that branch caused it,
and nothing on that branch could fix it, so it was split out rather than carried silently.

## The finding

Every server-spawned Claude Code run stalls before initialization on a node whose
`~/.claude/settings.json` declares hooks. The executor sends a hooks payload in its `initialize`
request that claude-code 2.1.114 never answers, so the process sits pre-init forever. Anchor:
`crates/executors/src/executors/claude.rs:175`.

The blast radius is every agent run the server spawns on such a node, not just breakdown runs.
During the DV evidence pass it was worked around by temporarily removing the hooks block from
`~/.claude/settings.json` (restored afterwards).

## What this workstream owns

1. Reproduce against a pinned claude-code version with a minimal hooks block.
2. Decide whether the executor should omit the hooks payload, negotiate it by version, or send it
   in a shape the CLI answers — and whether the CLI behaviour is itself a bug to report upstream.
3. A real-seam test: a spawned run that completes with hooks present in the settings file.

## Status

Not started. Filed 2026-08-09.
