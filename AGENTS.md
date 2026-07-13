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
cd remote-frontend && npm run lint
cd remote-frontend && npx tsc --noEmit
cd remote-frontend && npx vitest run
```

No CI-breaking or half-implemented code is deferred to the next session.

**Legitimate scope splits** are the only exception. They must:
1. Be explicitly named (e.g., `vk-swarm-node-ui-localize`)
2. Have a tracked follow-up workstream (`dev-docs/workstreams/<name>/README.md`)
3. Be documented in the decisions-ledger before the PR is submitted

## No Deferred Remediation (mandatory)

Code review findings — from adversarial panels, Gemini review, `/dr:pr`, the WAI reachability gate, or any other review step — must be fixed **in the same session** before the PR is pushed or merged.

- **False positive?** Document why in the decisions-ledger. Include the specific evidence (grep output, file:line) that disproves the finding.
- **Real finding?** Fix it now. "Fix in the next session" is not permitted.
- **Ambiguous?** Escalate to the user. Do not silently carry it forward.

This rule exists because deferred findings compound: the next session inherits stale context, the fix is harder without the original reasoning, and PR review history becomes misleading.

### Pre-existing debt discovered during a session (no carry-forward)

When a session discovers pre-existing failures (tests, lint, typecheck, or any gate red on the baseline before the session's changes) — whether surfaced by a review panel, the mandatory gate, or ad-hoc investigation — they are **not** silently handed to the next session. The session that finds them MUST do one of the following before it ends:

1. **Fix now** — remediate the pre-existing failure in this session, even if it falls outside the session's primary workstream; OR
2. **Split as a legitimate scope split** — explicitly named, with a tracked follow-up workstream (`dev-docs/workstreams/<name>/README.md`) created in THIS session, and documented in the decisions-ledger before the PR is submitted; OR
3. **Escalate to the user** — if the fix is architecturally entangled or requires a decision the agent cannot make.

A remediation prompt written for "the next session" does NOT satisfy this rule. "We finish what we start" means the debt is resolved (fixed, split, or escalated) before the session closes — never carried forward silently. The next session must inherit a clean ledger, not a backlog of "fix this later" notes.

Globally disabling a quality gate, linter, or entire test category via configuration (e.g. `doctest = false` in `Cargo.toml`, `#[cfg_attr(..., skip)]` on a whole module, or removing a test from the workspace) to bypass compilation or execution errors is itself a **silent deferral** and is prohibited unless paired with a tracked follow-up workstream created in THIS session or explicit user approval. Broken tests or documentation examples must be resolved at the source level — fixed, or selectively marked with the standard per-item attributes (e.g. `#[ignore]`, `rust,ignore`, `no_run`) so the remaining tests in the category continue to run and catch regressions.

Per-item `#[ignore]` or `rust,ignore` markers are the sanctioned source-level path for tests that cannot currently run (e.g., requiring a live database, network endpoint, or PTY device unavailable in CI). Their use is legitimate PROVIDED the session either: (a) makes at least one test in the category live so the suite is not entirely dead; AND (b) creates a tracked follow-up workstream (`dev-docs/workstreams/<name>/README.md`) documenting which tests remain ignored and what is required to bring them live. Marking tests ignored without (b) is a deferred deferral — it satisfies the letter of "source-level per-item attribute" while violating the spirit of "clean ledger."

## Testing standards

When a test requires a populated SQLite database, use one of:
- `db::test_utils::create_test_pool()` — fast template-copy approach; prefer for the majority of tests.
- `db::test_utils::create_test_pool_with_migrations()` — fresh migrations per test; use when the test exercises migration behavior itself or needs the full production schema.

Never manually duplicate `CREATE TABLE` SQL in test helpers (e.g., `setup_db()`-style functions). Schema defined outside `crates/db/migrations/` will drift from production and produce false-green tests that mask real regressions.

## Post-Phase Integrated Adversarial Review (mandatory)

Per-task adversarial panels verify each task in isolation. They **cannot** catch cross-task
interaction bugs — e.g. a fencing guard (task 205) + a reclaim path (task 209) + a completion path
combining to produce a query that returns `None` at the wrong time. After completing each WAI
phase, run an **integrated adversarial review** (Gemini or cross-model) over the full phase diff
before moving to the next phase. Findings are subject to the No Deferred Remediation rule above:
fix in-session or dismiss with ledger evidence. No exceptions for "I'll catch it in the next phase."

Report path: `.agents/reports/YYYY-MM-DD-round-N-<panelist>-<2-word-description>.md`.

## Safe Process Management

When running in a worktree spawned by vibe-kanban, NEVER use `pkill`, `killall`, or
pattern-based process killing — this can corrupt the parent server's database.

- Stop dev server: `pnpm run stop`
- Stop all instances: `pnpm run stop --all`
- Kill by exact PID only: `kill <PID>`

## GitHub Targeting

Open pull requests only against `davidrudduck/vk-swarm`.
Do NOT open PRs against `BloopAI/vibe-kanban` from this workspace.
