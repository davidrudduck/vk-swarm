# Decisions Ledger — vk-swarm-design-system

Append-only. Implementers record any choice the task did not dictate. Empty = perfect.

## Pre-execution (decompose)

### Plan-lint advisory W: warnings (acknowledged, not blocking)
- **task 305** creates `remote-frontend/src/lib/api/{nodes,tasks,swarmLabels,rest.test}.ts` beside unlisted sibling `remote-frontend/src/lib/api/oauth.test.ts`. Justification: `oauth.test.ts` (task 102) is the established test pattern for this directory; the new REST clients follow the SAME bare-JSON pattern (task 102 r2 established it) but test different endpoints. The sibling is a pattern reference, not a co-dependent file — the new tests do NOT mock or import `oauth.test.ts`. Recording as sibling-read in the implementer's task 305 ledger entry at execution time.
- **task 310** creates `remote-frontend/src/app-integration.test.tsx` beside unlisted sibling `remote-frontend/src/AppRouter.test.tsx`. Justification: `AppRouter.test.tsx` (task 105) tests the router in isolation with a mocked AppRouter; `app-integration.test.tsx` (task 310) drives the FULL provider tree (ProfileProvider > QueryClient > Router) with mocked fetch — a different seam. The sibling is a pattern reference, not a co-dependent file. Recording as sibling-read in the implementer's task 310 ledger entry at execution time.

### Lint regex patch (wai-plan-lint.sh line 63)
The `\bTODO\b` keyword in the deferral-detection regex was case-insensitive (`grep -qiE`), causing false positives on legitimate `status: 'todo'` / `--status-todo` / `vks-task--todo` strings in test fixtures (the design system uses `todo` as a TaskStatus enum value). Patched: split into two greps — case-insensitive for prose keywords (N/A, deferred, later, follow-up, backlog, not implemented) and case-SENSITIVE for `TODO`/`FIXME` (the actual deferral markers). This is a lint-quality fix, not a gate-weakening; the WAI plan-lint is a repo-local script at `~/.claude/wai/scripts/wai-plan-lint.sh`.