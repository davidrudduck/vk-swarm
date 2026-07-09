# Code Review Round 1 — Pre-graduation gate
**Date:** 2026-07-09
**Effort:** high (3 parallel reviewers: DeepSeek, MiniMax, Kimi)
**Diff range:** `cd883293..HEAD` (backlog-node-api branch)

## Findings

| # | File:line | Severity | Finding | Classification |
|---|-----------|----------|---------|----------------|
| 1 | `NodeApiKeySection.tsx:307` | MEDIUM | Stale error from prior mutation shown inside create dialog | Non-actionable (already fixed in `10ffadc1`) |
| 2 | `NodeApiKeySection.tsx:535` | LOW | Copy-failure message says "select manually" but secret is hidden | **Actionable** |
| 3 | `NodeApiKeySection.tsx:52-61` | LOW | `parseErrorMessage` leaks JSON quotes for bare JSON-string bodies | **Actionable** |
| 4 | `NodeApiKeySection.tsx:34-63` | LOW | `parseErrorMessage` not shared across codebase | Non-actionable (pre-existing pattern) |
| 5 | `NodeApiKeySection.tsx:74-543` | LOW | i18n key prefix `settings.swarm.apiKeys.*` misnamed for Nodes page | Non-actionable (pre-existing naming) |
| 6 | `NodeApiKeySection.tsx:317-330` | MEDIUM | iOS Safari `execCommand('copy')` fallback may silently fail | Non-actionable (edge case, modern browsers use Clipboard API) |
| 7 | `NodeApiKeySection.tsx:346` | LOW | Redundant `TooltipProvider` wrapper | Non-actionable (established pattern) |
| 8 | `NodeApiKeySection.tsx:181-185` | LOW | `isMountedRef.current = true` reassignment is dead | Non-actionable (cosmetic) |
| 9 | `NodeApiKeySection.tsx:486-493` | LOW | `data-secret-wrapper`/`data-hidden` are test-only attributes | Non-actionable (cosmetic) |
| 10 | `NodeApiKeySection.tsx:52-62` | LOW | `parseErrorMessage` returns `'Failed'` for many shapes → "Failed: Failed" | Non-actionable (display polish) |
| 11 | `NodeApiKeySection.tsx:220-239` | LOW | `onMutate` return value repurposed as attempt-id context | Non-actionable (works correctly) |
| 12 | `NodeApiKeySection.test.tsx:8` | LOW | Test imports locale JSON across package boundary | Non-actionable (pre-existing structural debt) |
| 13 | `dialog.tsx:17-43` | LOW | Custom Dialog does not trap focus or handle Escape | Non-actionable (pre-existing component limitation) |
| 14 | `NodeApiKeySection.tsx:317-331` | LOW | Fallback clipboard copy steals focus | Non-actionable (minor UX edge case) |
| 15 | `Nodes.tsx:29-45` | LOW | Non-i18n hard-coded strings | Non-actionable (pre-existing, unrelated) |
| 16 | `NodeApiKeySection.test.tsx:109,174` | LOW | Dynamic test imports inside test bodies | Non-actionable (cosmetic test style) |
| 17 | `NodeApiKeySection.tsx:241-287` | MEDIUM | Duplicate revoke/unblock mutation handlers | Non-actionable (maintainability, no functional defect) |

## Non-actionable

All non-actionable findings are pre-existing patterns, cosmetic issues, or edge cases that do not affect correctness or security. They are logged above for follow-up.

## Remediation

| Finding | Action |
|---------|--------|
| #2 (copy-failure message) | Updated message to "Click Reveal, then select and copy the secret manually." — `NodeApiKeySection.tsx:537`, `en/settings.json:754` |
| #3 (parseErrorMessage JSON quotes) | Added `typeof parsed === 'string'` branch before object check — `NodeApiKeySection.tsx:53` |

## Verdict

Actionable: [2, 3] → remediated in this round.
Non-actionable: [1, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17] → logged for follow-up.

Actionable: []
