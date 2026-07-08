# Tournament Round 5 — Adversarial Code Review
**Date:** 2026-07-08
**Challengers:** GPT-5.5, DeepSeek V4 Pro, GLM-5.2

## Scoring

| Challenger | Unique Valid Issues | With Fixes | Peer-Review Bonus | Total |
|------------|--------------------|------------|-------------------|-------|
| GPT-5.5 | 2 (#2 org error, #4 secret aria-label) | 2 | 0 | **3** |
| DeepSeek | 2 (#1 pendingKeyId race, #2 error clearing) | 2 | 1 (#1 approved by GLM) | **4** |
| GLM | 3 (#1-#3 dialog a11y, #7 aria-live) | 3 | 0 | **3** |

**Winner: DeepSeek** (4 points)

## Issues Identified and Remediated

### 1. pendingKeyId race condition — single ID across two mutations (High)
- **Found by:** DeepSeek #1, GLM (partial)
- **Fix:** Changed to `Set<string>` for per-key tracking; each mutation adds/removes independently
- **File:** `NodeApiKeySection.tsx:135,184-195,289`

### 2. Error not cleared on new mutation attempt (Low-Medium)
- **Found by:** DeepSeek #2
- **Fix:** Added `setError(null)` at start of handleRevoke/handleUnblock
- **File:** `NodeApiKeySection.tsx:185,192`

### 3. Secret code element has no accessible label (Low-Medium)
- **Found by:** GPT #4
- **Fix:** Added `aria-label` with i18n keys (secretVisible/secretHidden)
- **File:** `NodeApiKeySection.tsx:362-368`

## Issues Not Remediated (pre-existing/duplicate)
- **GPT #1:** Hardcoded strings in Nodes.tsx (pre-existing i18n workstream)
- **GPT #2:** useOrganizations error unhandled (pre-existing, out of scope)
- **GLM #1-#3:** Dialog a11y (Escape, focus trap, ARIA roles) — pre-existing dialog.tsx architecture
- **GLM #4:** Blocked reason redundancy (duplicate from R1-R4)
- **GLM #5:** Shared error state (accepted pattern)
- **GLM #6:** Test cleanup (duplicate from R4)
- **GLM #7:** aria-live on button (valid but minor)
- **GLM #8:** Org query error (same as GPT #2)

## Verification
- `tsc --noEmit`: clean
- `vitest run`: 26/26 pass
