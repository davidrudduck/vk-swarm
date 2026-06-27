# AGENTS.md - AI Agent Instructions

Read CLAUDE.md for the full development guide. This file highlights the non-negotiable rules
every AI agent must enforce before considering any PR complete.

## Finish What We Start (mandatory gate)

A PR is NOT done until all four checks are green on the final committed state:

```bash
cargo clippy --all --all-targets --all-features -- -D warnings
cargo test --workspace
cd frontend && npm run lint
cd frontend && npx tsc --noEmit
```

No CI-breaking or half-implemented code is deferred to the next session.

**Legitimate scope splits** are the only exception. They must:
1. Be explicitly named (e.g., `vk-swarm-node-ui-localize`)
2. Have a tracked follow-up workstream (`dev-docs/workstreams/<name>/README.md`)
3. Be documented in the decisions-ledger before the PR is submitted

## Safe Process Management

When running in a worktree spawned by vibe-kanban, NEVER use `pkill`, `killall`, or
pattern-based process killing — this can corrupt the parent server's database.

- Stop dev server: `pnpm run stop`
- Stop all instances: `pnpm run stop --all`
- Kill by exact PID only: `kill <PID>`

## GitHub Targeting

Open pull requests only against `davidrudduck/vk-swarm`.
Do NOT open PRs against `BloopAI/vibe-kanban` from this workspace.
