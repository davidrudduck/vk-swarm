# Tournament Round 15 — Scoring Report

**Date:** 2026-07-08
**Challengers:** Kimi, MiniMax, GLM
**Commit:** `e9500920` (tournament R15: remove setError(null) from closeDialog)

## Findings

| # | Issue | Found By | Severity | Valid? | Points |
|---|-------|----------|----------|--------|--------|
| 1 | `closeDialog` clobbers errors from unrelated mutations | MiniMax | MEDIUM | Valid | 1 |

**Kimi:** No findings
**GLM:** Incomplete result

## Scores

| Challenger | Findings | Points |
|---|---|---|
| **MiniMax** | 1 finding (1) | **1** |
| **Kimi** | 0 findings | **0** |
| **GLM** | 0 findings (incomplete) | **0** |

**Winner: MiniMax (1 point)**

## Remediations Applied

1. **Remove setError(null) from closeDialog:** The create dialog's lifecycle is now orthogonal to the list-management error display. Errors from revoke/unblock persist until the next successful mutation or org change.

## Tournament Status

- **Round 8:** 0 → clean
- **Round 9-15:** NOT clean
- **Next:** Round 16 required. Pick 3 from remaining challengers.
