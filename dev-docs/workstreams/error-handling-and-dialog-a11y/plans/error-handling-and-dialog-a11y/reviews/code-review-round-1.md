# Code Review — Round 1

**Target:** `artful-egret`   **Range:** `69d72ef20..HEAD`   **Effort:** high

## Findings

| # | File:line | Severity | Category | Finding | Confidence | Actionable? |
|---|-----------|----------|----------|---------|-----------|-------------|
| 1 | `remote-frontend/src/lib/errors.ts:1-7` | low | correctness | JSDoc misleadingly claims `parseErrorMessage` handles "ApiError with error_data", but the function never inspects `error_data`. `ApiError extends Error` so the function enters the `instanceof Error` branch and only uses `.message`. The JSDoc should accurately describe the behavior. | high | yes |
| 2 | `remote-frontend/src/lib/errors.test.ts:51-53, 98-100` | low | quality | Duplicate test — both assert `parseErrorMessage(new Error('42')) === '42'`. Redundant; wastes test execution time. | high | yes |
| 3 | `remote-frontend/src/lib/errors.ts:25` | low | correctness | `parseErrorMessage(new Error('   '))` returns raw whitespace string `'   '` instead of `'Failed'`. This produces a user-visible blank error message, which is poor UX. `!raw` check at line 25 should also reject whitespace-only strings. | high | yes |

## Non-actionable

| # | File:line | Severity | Category | Finding | Confidence | Why non-actionable |
|---|-----------|----------|----------|---------|-----------|---------------------|
| 4 | `remote-frontend/src/components/ui/dialog.tsx:24,46` | medium | correctness | Dialog z-50 vs alert-dialog z-[9999] — different from old custom dialog's z-[9999]. If both open simultaneously, stacking is correct (alert above dialog), but any CSS/third-party widget assuming dialog-layer at ~10000 will not find it. | medium | Intentional design — Radix shadcn convention is z-50 for dialogs, z-[9999] reserved for alert-dialogs. Documented in code. |
| 5 | `remote-frontend/src/lib/api/utils.ts:39` | low | correctness | `anySignal([])` returns a dead signal (never aborted). No current caller passes empty array; `makeRequest` only calls with truthy `options.signal`. | high | No current caller hits this; changing contract would be scope creep. |
| 6 | `remote-frontend/src/lib/api/utils.test.ts:301-316` | medium | correctness | Timeout test's signal-abort mock passes through `signal.reason` as `'Request timed out'`, but real `fetch` wraps abort in its own `AbortError`. Test is correct for the mock but gives false confidence about real-world error message shape. | medium | Standard mock-testing pattern; the mock verifies timeout-triggers-abort, not `fetch` internals. |

## Verdict: Approve (with remediations applied)

Actionable: []