# Tournament Round 18 — Scoring Report

**Date:** 2026-07-08
**Challengers:** GPT-5.5, DeepSeek, Claude Opus
**Commit:** `cff622b5` (tournament R18: always invalidate cache on create success)

## Findings

| # | Issue | Found By | Severity | Valid? | Points |
|---|-------|----------|----------|--------|--------|
| 1 | Create skips cache invalidation if dialog closed or org changed | Claude Opus | MEDIUM | Valid | 1 |

**GPT-5.5:** No findings
**DeepSeek:** No findings

## Scores

| Challenger | Findings | Points |
|---|---|---|
| **Claude Opus** | 1 finding (1) | **1** |
| **GPT-5.5** | 0 findings | **0** |
| **DeepSeek** | 0 findings | **0** |

**Winner: Claude Opus (1 point)**

## Remediations Applied

1. **Always invalidate cache on create success:** Moved `invalidateQueries` before the epoch/org/mounted guards in `createMutation.onSuccess`. The cache is now always invalidated when the server successfully creates a key, even if the user closed the dialog or switched orgs mid-flight. UI state updates (secret, error, name) are still guarded.

## Tournament Status

- **Round 8:** 0 → clean
- **Round 9-18:** NOT clean
- **Next:** Round 19 required. Pick 3 from remaining challengers.
