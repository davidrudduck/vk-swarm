# Tournament Round 17 — Scoring Report

**Date:** 2026-07-08
**Challengers:** Kimi, MiniMax, GLM
**Commit:** `1e9f331f` (tournament R17: Badge ref fix, isMountedRef guards, doc accuracy)

## Findings

| # | Issue | Found By | Severity | Valid? | Points |
|---|-------|----------|----------|--------|--------|
| 1 | Badge ref warning in TooltipTrigger | Kimi | MEDIUM | Valid | 1 |
| 2 | Asymmetric `isMountedRef` guards | Kimi | MEDIUM | Valid | 1 |
| 3 | Doc mis-describes org ID capture | Kimi | LOW | Valid | 1 |

**MiniMax:** No findings
**GLM:** No findings

## Scores

| Challenger | Findings | Points |
|---|---|---|
| **Kimi** | 3 findings (1+1+1) | **3** |
| **MiniMax** | 0 findings | **0** |
| **GLM** | 0 findings | **0** |

**Winner: Kimi (3 points)**

## Remediations Applied

1. **Badge ref fix:** Wrapped `<Badge>` in a `<span>` element inside `<TooltipTrigger asChild>` to fix the ref forwarding warning. Badge doesn't forward refs, so Radix can't attach its trigger ref directly.

2. **isMountedRef guards:** Added `isMountedRef` guards to all mutation success/error callbacks. Previously only revoke/unblock `onError` had the guard. Now all callbacks consistently check `isMountedRef` before calling `setState`.

3. **Doc accuracy:** Updated architecture doc to clarify that the org ID comes from mutation variables, not from `orgIdRef`. `orgIdRef` is used to ignore callbacks whose org context has changed.

## Tournament Status

- **Round 8:** 0 → clean
- **Round 9-17:** NOT clean
- **Next:** Round 18 required. Pick 3 from remaining challengers.
