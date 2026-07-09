# Tournament Round 4 — Adversarial Code Review
**Date:** 2026-07-08
**Challengers:** Claude Opus (disqualified — wrong codebase), MiniMax M3, Kimi K2.7 Code

## Scoring

| Challenger | Unique Valid Issues | With Fixes | Peer-Review Bonus | Total |
|------------|--------------------|------------|-------------------|-------|
| Claude Opus | N/A (wrong codebase) | — | — | **0** |
| MiniMax M3 | 3 (#1 isPending, #3 test cleanup, #4 alert role) | 3 | 1 (#1 approved by Kimi) | **5** |
| Kimi K2.7 | 3 (#1 isPending, #2 Escape key, #4 alert role) | 3 | 0 | **4** |

**Winner: MiniMax M3** (5 points)

## Issues Identified and Remediated

### 1. Global isPending disables all buttons, not just the targeted key (High)
- **Found by:** MiniMax #1, Kimi #1
- **Fix:** Changed to per-key `pendingKeyId` tracking; only the specific key's button is disabled
- **File:** `NodeApiKeySection.tsx:135,183-187,97-115,287`

### 2. Nodes.tsx error lacks role="alert" (Medium)
- **Found by:** MiniMax #4, Kimi #4
- **Fix:** Added `role="alert"` to error `<p>` element
- **File:** `Nodes.tsx:43`

## Issues Not Remediated (accepted/duplicate)
- **MiniMax #2:** Clipboard failure silent (duplicate from R3, low priority)
- **MiniMax #3:** Test cleanup not in try/finally (valid but test passes reliably)
- **Kimi #2:** Custom Dialog no Escape key (UI component change, out of scope for this PR)
- **Kimi #3:** Hardcoded strings in Nodes.tsx (pre-existing, i18n workstream)
- **MiniMax #5:** Hardcoded strings (same as Kimi #3)
- **MiniMax #6:** confirm() inconsistency (spec requirement)

## Verification
- `tsc --noEmit`: clean
- `vitest run`: 26/26 pass
