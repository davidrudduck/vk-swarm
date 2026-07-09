# Tournament Round 3 — Adversarial Code Review
**Date:** 2026-07-08
**Challengers:** DeepSeek V4 Pro, GLM-5.2, Xiaomi Mimo V2.5 Pro

## Scoring

| Challenger | Unique Valid Issues | With Fixes | Peer-Review Bonus | Total |
|------------|--------------------|------------|-------------------|-------|
| DeepSeek | 3 (#1 query error, #2 spinner, #5 execCommand) | 3 | 1 (#2 approved by GLM) | **5** |
| GLM | 3 (#2 revoke/unblock pending, #3 clipboard feedback, #11 uncloseable test) | 3 | 0 | **4** |
| Mimo | 2 (#2 DialogDescription, #3 aria-live) | 2 | 0 | **3** |

**Winner: DeepSeek** (5 points)

## Issues Identified and Remediated

### 1. Query error silently swallowed — shows empty state instead of error (High)
- **Found by:** DeepSeek #1
- **Fix:** Added `isError: isListError` to useQuery destructure; added error Alert in content area
- **File:** `NodeApiKeySection.tsx:136,268-273`

### 2. Nodes.tsx inaccessible loading spinner (Medium)
- **Found by:** DeepSeek #2, GLM #1, Mimo (partial)
- **Fix:** Added `role="status"` and `<span className="sr-only">Loading nodes...</span>`
- **File:** `Nodes.tsx:34-36`

### 3. Revoke/unblock buttons not disabled during pending (Medium)
- **Found by:** GLM #2
- **Fix:** Added `isRevokePending`/`isUnblockPending` props to ApiKeyItem; disabled buttons during pending
- **File:** `NodeApiKeySection.tsx:29-33,97-115,281-283`

### 4. Missing DialogDescription in create form (High)
- **Found by:** Mimo #2
- **Fix:** Added `DialogDescription` to create form dialog header
- **File:** `NodeApiKeySection.tsx:302-304`

### 5. No aria-live for copy feedback (Medium)
- **Found by:** Mimo #3
- **Fix:** Added `aria-live="polite"` to copy button
- **File:** `NodeApiKeySection.tsx:375`

### 6. document.execCommand mock not restored in test (Low)
- **Found by:** DeepSeek #5
- **Fix:** Save and restore `document.execCommand` in TS4 test
- **File:** `NodeApiKeySection.test.tsx:102-103,146`

### 7. New i18n keys: createDescription, loadError
- Added to en locale and TS9 test

## Verification
- `tsc --noEmit`: clean
- `vitest run`: 26/26 pass
