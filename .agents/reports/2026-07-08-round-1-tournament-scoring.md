# Tournament Round 1 — Adversarial Code Review
**Date:** 2026-07-08
**Challengers:** Claude Opus (claude-opus-4-8), DeepSeek V4 Pro (deepseek/deepseek-v4-pro), Kimi K2.7 Code (moonshotai/kimi-k2.7-code)

## Scoring

| Challenger | Issues Found | With Fixes | Peer-Review Bonus | Total |
|------------|-------------|------------|-------------------|-------|
| Claude Opus | 5 (P3,P4,P6,P10,P12) | 5 | 1 (P3 fix approved by DeepSeek) | **11** |
| DeepSeek V4 Pro | 5 (#1,#2,#5,#9,#15) | 5 | 1 (#1 useRef cleanup approved by Kimi) | **11** |
| Kimi K2.7 Code | 6 (#1,#2,#5,#6,#7,#10) | 5 | 1 (#2 conditional render approved by DeepSeek) | **12** |

**Winner: Kimi K2.7 Code** (12 points)

## Issues Identified and Remediated

### 1. Uncleared setTimeout → setState on unmount (Critical)
- **Found by:** DeepSeek #1, Kimi #1
- **Fix:** Added `useRef` for `copyTimeoutRef`, clear in `closeDialog` and before setting new timeout
- **File:** `NodeApiKeySection.tsx:1,127,183,201`

### 2. Secret in DOM when "hidden" — CSS blur only (High)
- **Found by:** Kimi #2
- **Fix:** Changed to conditional render: `{showSecret ? createdSecret : '••••••••••••••••••••'}`
- **File:** `NodeApiKeySection.tsx:330`

### 3. Stale error Alert persists across dialog close (Low-Medium)
- **Found by:** Claude P3, DeepSeek #5
- **Fix:** Added `setError(null)` to `closeDialog`
- **File:** `NodeApiKeySection.tsx:183`

### 4. Untrimmed key name sent to backend (Low)
- **Found by:** Claude P4
- **Fix:** Changed `createMutation.mutate(newKeyName)` to `createMutation.mutate(newKeyName.trim())`
- **File:** `NodeApiKeySection.tsx:308`

### 5. i18n mock returns key instead of fallback (Medium)
- **Found by:** Kimi #7
- **Fix:** Changed mock to `return fallback || key` and updated all test assertions to use fallback strings
- **File:** `NodeApiKeySection.test.tsx:25,55,83-84,110-113,125-128,130,133,137,139,167,203,207,223-226`

### 6. Missing `asChild` on TooltipTrigger (Medium)
- **Found by:** Kimi #4
- **Fix:** Added `asChild` to `TooltipTrigger`
- **File:** `NodeApiKeySection.tsx:50`

### 7. Unnecessary `as string` cast (Medium)
- **Found by:** Kimi #5, Claude P9
- **Fix:** Removed `as string` cast from placeholder
- **File:** `NodeApiKeySection.tsx:296`

### 8. Unused `loading` i18n key + inaccessible spinner (Low)
- **Found by:** Claude P6, Kimi #6
- **Fix:** Added `role="status"` and `<span className="sr-only">` with `t('settings.swarm.apiKeys.loading')` to loading spinner
- **File:** `NodeApiKeySection.tsx:248-251`

### 9. `blocked_reason` tooltip shows when null (Low)
- **Found by:** Claude P5
- **Fix:** Guarded tooltip content with `apiKey.blocked_reason &&`
- **File:** `NodeApiKeySection.tsx:57`

### 10. `createTitle` default diverges from en locale value (Nit)
- **Found by:** Claude P10
- **Status:** Documented — the component renders the fallback "Generate API Key" (matching SC1); the en locale has "Create Node API Key" as the dialog title. Pre-existing pattern; locale is not loaded at runtime.

## Verification
- `tsc --noEmit`: clean
- `vitest run`: 26/26 pass

## Issues Not Remediated (accepted as-is)
- **Claude P1/P2:** i18n non-functional in remote-frontend (pre-existing architectural pattern, documented in decisions-ledger)
- **DeepSeek #3:** Clipboard stays in OS clipboard (browser limitation)
- **Kimi #3:** ApiError details swallowed (existing error handling pattern)
- **DeepSeek #9/Kimi #6:** Test TS9 checks unused `loading` key (now used after fix #8)
- **DeepSeek #10:** No maxLength on key name input (server validates)
- **Kimi #8:** Cross-package relative import in test (pre-existing pattern)
