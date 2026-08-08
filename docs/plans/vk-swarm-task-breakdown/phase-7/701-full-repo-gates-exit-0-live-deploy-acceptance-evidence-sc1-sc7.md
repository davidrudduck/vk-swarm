---
id: "701"
phase: 7
title: "Full repo gates (exit 0) + live deploy acceptance evidence (SC1–SC7)"
status: ready
depends_on: ["401","503","602","603"]
parallel: false
conflicts_with: []
files:
  - "docs/plans/vk-swarm-task-breakdown/decisions-ledger.md"
irreversible: false
scope_test: "N/A"
allowed_change: edit
covers_criteria: ["SC1","SC2","SC3","SC4","SC5","SC6","SC7"]
covers_tests: ["TS6"]
---
## Failing test (write first)
N/A — covered by existing tests: the full gate set below; this task adds no code.


## Change
**File:** docs/plans/vk-swarm-task-breakdown/decisions-ledger.md — append a `## Deploy verification` section containing the fenced command outputs enumerated in Manual verification. No source files change in this task.


## Allowed moves
Only appending evidence sections to the decisions ledger.


## STOP triggers
Any gate exits non-zero (fix belongs in the owning task — reopen it; PRE-EXISTING baseline failures follow CLAUDE.md: fix now, tracked scope-split, or escalate); live verification shows the breakdown flow failing in a way that contradicts the spec (escalate — spec change requires re-precheck).


## Manual verification (record in decisions-ledger)
Record ALL in the ledger, fenced; every gate MUST exit 0:
1. `cargo fmt --all -- --check`; 2. `cargo clippy --all --all-targets --all-features -- -D warnings`; 3. `cargo test --workspace`; 4. `cd frontend && npm run lint && npx tsc --noEmit && npx vitest run`; 5. `cd remote-frontend && npm run lint && npx tsc --noEmit && npx vitest run`; 6. `npm run generate-types:check`.
Live on a running node (operator evidence):
7. SC1: create a task titled 'Add CSV export' with a multi-part description; invoke Break down from the card; paste the GET /api/tasks/{id}/breakdown JSON showing ≥2 proposed items.
8. SC2: edit one item title, delete one item via the dialog; paste before/after items JSON; confirm no attempt can be started from a proposal and `sqlite3 <db> "SELECT count(*) FROM node_outbox WHERE created_at > <trigger-ts>"` shows no proposal-driven rows pre-accept.
9. SC3: accept; paste the created child tasks JSON + GET /api/tasks/{child}/dependencies output showing the edge(s); start an attempt on one child (works).
10. SC4: call the break_down_task MCP tool against a second task; paste the tool result and the matching GET output; accept via accept_breakdown.
11. SC5: with auto_breakdown_enabled ON create a described task → draft proposal appears (paste); with it OFF create another → no proposal (paste query).
12. SC6: stop the hive (or disconnect network), repeat SC1→SC3 end-to-end; paste evidence + post-reconnect hive task list showing the children synced.
13. SC7: point the project's executor profile at a bad binary, trigger a breakdown; paste the failed-proposal JSON (status failed, error set) and the retry succeeding after restoring the profile.
14. SC7b (malformed completion; CodeRabbit PR470): point the executor profile at a stub that prints non-JSON prose and exits 0; trigger; paste the failed-proposal JSON (error from the parser's NoResult path), confirm zero proposal items and zero child tasks, then a successful retry after restoring the profile.
15. Breakdown side-effect invariants (CodeRabbit PR470 R2): after the SC1 run, paste the parent task's status showing it UNCHANGED (never InReview) and `git log --oneline` of the breakdown attempt's branch showing zero commits from the run (the read-only prompt + 203's commit-path exclusion + 204's finalize exclusion under live proof).


## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-task-breakdown 701` exits 0
