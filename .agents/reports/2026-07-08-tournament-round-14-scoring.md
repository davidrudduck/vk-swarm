# Tournament Round 14 — Scoring Report

**Date:** 2026-07-08
**Challengers:** GPT-5.5, DeepSeek, Claude Opus
**Commit:** `4bad9ecc` (tournament R14: org guard for revoke/unblock mutation callbacks)

## Findings

| # | Issue | Found By | Severity | Valid? | Points |
|---|-------|----------|----------|--------|--------|
| 1 | Stale revoke/unblock failures surface on wrong org | GPT-5.5 + Claude Opus | MEDIUM | Valid | 1 each |

**DeepSeek:** No findings

## Scores

| Challenger | Findings | Points |
|---|---|---|
| **GPT-5.5** | 1 finding (1) | **1** |
| **Claude Opus** | 1 finding (1) | **1** |
| **DeepSeek** | 0 findings | **0** |

**Winners: GPT-5.5 & Claude Opus (tied at 1 point)**

## Remediations Applied

1. **Org guard for revoke/unblock callbacks:** Added `if (orgId !== orgIdRef.current) return;` to `onError` callbacks and `if (orgId === orgIdRef.current) setError(null);` to `onSuccess` callbacks. This prevents stale errors from old orgs appearing in new org context. The `invalidateQueries` still targets the correct org regardless.

## Tournament Status

- **Round 8:** 0 → clean
- **Round 9-14:** NOT clean
- **Next:** Round 15 required. Pick 3 from remaining challengers.
