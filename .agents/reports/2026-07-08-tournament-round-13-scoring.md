# Tournament Round 13 — Scoring Report

**Date:** 2026-07-08
**Challengers:** Kimi, MiniMax, GLM
**Commit:** `c074f063` (tournament R13: full org-change state reset, lint fix)

## Findings

| # | Issue | Found By | Severity | Valid? | Points |
|---|-------|----------|----------|--------|--------|
| 1 | `react-hooks/exhaustive-deps` lint warning breaks lint gate | Kimi | MEDIUM | Valid | 1 |
| 2 | Org change doesn't reset secret-reveal state or pending-key state | Kimi | MEDIUM | Valid | 1 |

**MiniMax:** No findings
**GLM:** No findings

## Scores

| Challenger | Findings | Points |
|---|---|---|
| **Kimi** | 2 findings (1+1) | **2** |
| **MiniMax** | 0 findings | **0** |
| **GLM** | 0 findings | **0** |

**Winner: Kimi (2 points)**

## Remediations Applied

1. **Full org-change state reset:** Updated the useEffect to reset ALL create-related state when `organizationId` changes, including `createdSecret`, `showSecret`, `copied`, and `pendingKeyIds`. Previously it only reset when the dialog was open without a secret.

2. **Lint fix:** Moved `eslint-disable-next-line` directive to the correct line (before the dependency array, not before the useEffect call). Added explanatory comment.

## Tournament Status

- **Round 8:** 0 → clean
- **Round 9-13:** NOT clean
- **Next:** Round 14 required. Pick 3 from remaining challengers.
