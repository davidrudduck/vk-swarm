# Tournament Round 10 — Scoring Report

**Date:** 2026-07-08
**Challengers:** GPT-5.5, DeepSeek, MiMo
**Commit:** `6c895ede` (tournament R10: StrictMode fix, test gaps, doc accuracy)

## Findings

| # | Issue | Found By | Severity | Valid? | Points |
|---|-------|----------|----------|--------|--------|
| 1 | Error alert renders behind modal overlay for create failures | GPT-5.5 | MEDIUM | Valid (UX) | 1 |
| 2 | `handleCopySecret` can set `copied=true` after dialog close | GPT-5.5 | MEDIUM | Valid | 1 |
| 3 | `handleCreateSubmit` has no idempotency guard | GPT-5.5 | MEDIUM | **Invalid** — button disabled when isPending | 0 |
| 4 | Doc says dialog disables Escape key | GPT-5.5 | LOW | Valid | 1 |
| 5 | Blocked key with `blocked_reason: null` — untested | DeepSeek | MEDIUM | Valid | 1 |
| 6 | `pendingKeyIds` cleanup on error — untested | DeepSeek | MEDIUM | Valid | 1 |
| 7 | `parseErrorMessage` returns undefined for Symbol | DeepSeek | LOW | Valid | 1 |
| 8 | `createMutation.reset()` doesn't abort in-flight request | DeepSeek | LOW | Valid | 1 |
| 9 | `Dialog onOpenChange` no-op return | DeepSeek | LOW | Valid (cosmetic) | 1 |
| 10 | `isMountedRef` stuck false after StrictMode remount | MiMo | MEDIUM | Valid | 3 |
| 11 | Test TS1 doesn't verify loading spinner | MiMo | MEDIUM | Valid | 1 |
| 12 | Doc says 21 test cases, actual count is 24 | MiMo | LOW | Valid | 1 |

## Scores

| Challenger | Findings | Points |
|---|---|---|
| **DeepSeek** | 5 findings (1+1+1+1+1) | **5** |
| **MiMo** | 3 findings (3+1+1) | **5** |
| **GPT-5.5** | 3 findings (1+1+0+1) | **3** |

**Winners: DeepSeek & MiMo (tied at 5 points)**

## Remediations Applied

1. **isMountedRef StrictMode fix:** Changed from `useRef(true)` with cleanup-only effect to `useEffect` that sets `isMountedRef.current = true` on mount and `false` on unmount. This survives React 18 StrictMode's unmount/remount cycle.

2. **parseErrorMessage Symbol guard:** Added `typeof err === 'symbol'` branch returning `'Failed'` before the `else` branch. Prevents `JSON.stringify(Symbol())` returning `undefined` and violating the `string` return type.

3. **blocked_reason: null test (TS19):** Added test with a blocked key where `blocked_reason` is `null`. Asserts the Blocked badge renders without tooltip and the Unblock button is present.

4. **pendingKeyIds cleanup tests (TS20, TS21):** Added tests that verify the Unblock/Revoke button is re-enabled after a failed mutation. This ensures `onSettled` cleanup correctly removes the key from `pendingKeyIds`.

5. **Nodes.tsx TS1 fix:** Added `expect(screen.getByRole('status')).toBeInTheDocument()` to actually verify the loading spinner renders.

6. **Doc fixes:** Removed Escape key claim, updated test case count from 21 to 24, added TS19-TS21 to the test table.

## Tournament Status

- **Round 8:** 0 valid issues → clean
- **Round 9:** 10 valid issues → NOT clean
- **Round 10:** 11 valid issues → NOT clean
- **Next:** Round 11 required. Pick 3 from remaining challengers (GPT-5.5, DeepSeek, GLM, MiMo, Kimi, MiniMax, Claude Opus).
