# Tournament Round 2 — Adversarial Code Review
**Date:** 2026-07-08
**Challengers:** GPT-5.5, MiniMax M3 (minimax/minimax-m3), GLM-5.2 (z-ai/glm-5.2)

## Scoring

| Challenger | Unique Issues | With Fixes | Peer-Review Bonus | Total |
|------------|--------------|------------|-------------------|-------|
| GPT-5.5 | 4 (#1,#3,#4,#7) | 4 | 2 (#3 approved by MiniMax, #4 approved by GLM) | **8** |
| MiniMax M3 | 5 (#1,#7,#10,#12,#14) | 4 | 1 (#12 approved by GLM) | **6** |
| GLM-5.2 | 6 (#1,#3,#7,#10,#11,#14) | 5 | 1 (#3 approved by GPT) | **7** |

**Winner: GPT-5.5** (8 points)

## Issues Identified and Remediated

### 1. copyTimeoutRef needs useEffect cleanup on unmount (High)
- **Found by:** GPT #1, MiniMax #3, GLM #4
- **Fix:** Added `useEffect(() => () => clearTimeout(copyTimeoutRef.current), [])` cleanup
- **File:** `NodeApiKeySection.tsx`

### 2. console.error leaks secret in error object (Critical/Security)
- **Found by:** MiniMax #1, GLM #2
- **Fix:** Changed `console.error('Failed to copy secret', e)` to `console.error('Failed to copy to clipboard')`
- **File:** `NodeApiKeySection.tsx:208`

### 3. No staleTime on API keys query (Medium)
- **Found by:** MiniMax #7
- **Fix:** Added `staleTime: 30_000` to match sibling `nodes` query pattern
- **File:** `NodeApiKeySection.tsx:131`

### 4. TS7 test assertion fragile (Low)
- **Found by:** MiniMax #10
- **Fix:** Changed `listSpy.mock.calls.length === callsBefore` to `toHaveBeenCalledTimes(N)`
- **File:** `NodeApiKeySection.test.tsx`

### 5. Escape key loses secret forever (High)
- **Found by:** MiniMax #12, GLM #12
- **Fix:** Added `onEscapeKeyDown` and `onPointerDownOutside` guards when `createdSecret` is set
- **File:** `NodeApiKeySection.tsx`

### 6. Copy fallback textarea leaks on exception (Critical)
- **Found by:** GLM #1
- **Fix:** Wrapped textarea select/execCommand in try/finally to ensure `removeChild` runs
- **File:** `NodeApiKeySection.tsx`

### 7. closeDialog doesn't abort in-flight create mutation (High)
- **Found by:** GLM #3
- **Fix:** Added `createMutation.reset()` to `closeDialog`
- **File:** `NodeApiKeySection.tsx`

### 8. Tooltip renders when blocked_reason is null (Medium)
- **Found by:** GLM #7
- **Fix:** Made Tooltip conditional on `blocked_reason` being present
- **File:** `NodeApiKeySection.tsx`

### 9. TS9 `as any` cast (Low)
- **Found by:** GLM #10
- **Fix:** Added guard with descriptive expect message
- **File:** `NodeApiKeySection.test.tsx`

### 10. navigator.clipboard not restored after test (Medium)
- **Found by:** GLM #11
- **Fix:** Save and restore `navigator.clipboard` in test
- **File:** `NodeApiKeySection.test.tsx`

### 11. No maxLength on key name input (Low)
- **Found by:** GLM #14, MiniMax #15
- **Fix:** Added `maxLength={100}` to Input
- **File:** `NodeApiKeySection.tsx`

## Issues Not Remediated (accepted/duplicate)
- **GPT #2:** Shared error state (duplicate of R1, accepted as-is)
- **GPT #3:** created_at.slice() timezone (low severity, API returns UTC)
- **GPT #7:** TS9 value assertion (strengthening, not a bug)
- **MiniMax #4:** confirm() vs Dialog (spec requirement)
- **GLM #5:** showCreateDialog stale state (covered by #7 fix)

## Verification
- `tsc --noEmit`: clean
- `vitest run`: 26/26 pass
