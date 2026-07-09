# Tournament Round 19 — Scoring Report

**Date:** 2026-07-08
**Challengers:** Kimi, MiniMax, GLM
**Commit:** `29ddfb59` (tournament R19: pendingKeyIds counter, parseErrorMessage normalization)

## Findings

| # | Issue | Found By | Severity | Valid? | Points |
|---|-------|----------|----------|--------|--------|
| 1 | Concurrent revoke/unblock re-enables button early | Kimi | MEDIUM | Valid | 1 |
| 2 | `parseErrorMessage` can return empty/undefined | Kimi | MEDIUM | Valid | 1 |

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

1. **pendingKeyIds counter:** Changed from `Set<string>` to `Map<string, number>`. Each mutation increments the counter on dispatch and decrements on settle. The button is disabled when the counter > 0. This correctly handles concurrent mutations for the same key.

2. **parseErrorMessage normalization:** Added `?? 'Failed'` for `JSON.stringify` result, `if (!raw) return 'Failed'` guard, and `|| 'Failed'` fallbacks for all return paths. Also added empty-string checks for `parsed.message` and `parsed.error`. The function now always returns a non-empty string.

## Tournament Status

- **Round 8:** 0 → clean
- **Round 9-19:** NOT clean
- **Next:** Round 20 required. Need two consecutive clean rounds.
