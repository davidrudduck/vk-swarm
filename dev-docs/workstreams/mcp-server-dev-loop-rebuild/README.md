---
workstream: mcp-server-dev-loop-rebuild
doc_type: readme
status: draft
title: "Dev loop never rebuilds or restarts vks-mcp-server"
depends_on: []
adrs: []
staging_pointers:
  - docs/plans/vk-swarm-task-breakdown/decisions-ledger.md
---

# mcp-server-dev-loop-rebuild

**Origin:** deploy-verification finding DV-5 during `vk-swarm-task-breakdown` (PR #475). Backlog
finding **F-2026-08-09-02** (medium). Pre-existing tooling gap, split out rather than carried
silently.

## The finding

The cargo-watch dev loop rebuilds and restarts `vks-node-server` but never `vks-mcp-server`. A node
therefore keeps serving whatever MCP binary was last built by hand, so backend changes that add or
change MCP tools are invisible until someone rebuilds and restarts that binary manually.

Observed during DV evidence: the new breakdown MCP tools (`break_down_task`, `get_breakdown`,
`accept_breakdown`) were absent from the node's tool list until `vks-mcp-server` was rebuilt and
restarted by hand. Anchor: the dev scripts in `package.json`.

## What this workstream owns

1. Extend the dev watch/restart sequence to cover `vks-mcp-server`, respecting the documented
   graceful-shutdown ordering (backend cleanup completes before dev processes are killed).
2. Confirm the MCP port registration in `/tmp/vibe-kanban/instances/` stays correct across restarts.
3. A check that a newly added MCP tool appears in a running node's tool list after one dev-loop
   iteration, with no manual rebuild.

## Status

Not started. Filed 2026-08-09.
