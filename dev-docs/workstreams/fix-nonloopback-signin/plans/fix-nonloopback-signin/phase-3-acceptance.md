# Phase 3 — Hardline acceptance verification

Run every required automated and manual check. This phase is deliberately not optional: it is the acceptance boundary for the workstream.

Tasks:

- `301` — Record full gates and manual LAN OAuth verification.

Exit criteria:

- Targeted remote-frontend tests pass.
- Whole remote-frontend test suite, lint, and typecheck pass.
- AGENTS.md mandatory gate passes.
- Manual LAN OAuth checks pass for both normal login and invitation acceptance, or the executor stops and escalates why the environment cannot perform them.
