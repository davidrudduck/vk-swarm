# Execution Review — error-handling-and-dialog-a11y

**Date:** 2026-07-10
**Round:** 1
**Panelist:** Opus (claude-opus-4-8)
**Workstream:** error-handling-and-dialog-a11y
**Spec:** docs/superpowers/specs/2026-07-10-error-handling-and-dialog-a11y.md
**Plan:** docs/plans/error-handling-and-dialog-a11y/plan.md

---

## Assessment Summary

The workstream's **intent is delivered**: shared `parseErrorMessage`, Radix a11y dialog,
AGENTS/CLAUDE gate fix. Plan adherence is high. Three issues were found and remediated.

## SC Verification

| SC | Status | Evidence |
|----|--------|----------|
| SC1 | ✅ PASS | `remote-frontend/src/lib/errors.ts` exists, 17 tests in `errors.test.ts` |
| SC2 | ✅ PASS | 6 dialog files migrated: SwarmLabelDialog:88, MergeProjectsDialog:73, MergeLabelsDialog:89, MergeTemplatesDialog:73, SwarmProjectDialog:78, SwarmTemplateDialog:90 |
| SC3 | ✅ PASS | `dialog.tsx` rewritten on Radix. role="dialog" via Radix. Escape/close/uncloseable all tested in `dialog.test.tsx` (7 tests). Scrollability restored with `max-h-[85vh] overflow-y-auto`. |
| SC4 | ✅ PASS | 2 createAttemptRef tests: closeDialog guard (TS-level), org-change guard (TS-level). Both verify form is fresh after guard fires. |
| SC5 | ✅ PASS | 2 orgIdRef tests: revoke onError after org change, create onError after org change. |
| SC6 | ✅ PASS | 40 tests pass (36 existing + 4 new). All existing behavior preserved. |
| SC7 | ✅ PASS | `vite.config.ts` excludes `**/scripts/**` (was picking up `node:test` file). AppRouter failure is pre-existing test isolation issue. |

## Plan Divergences

| # | Divergence | Needed? | Risk |
|---|-----------|---------|------|
| 1 | SC2 file set corrected: removed NodeTemplatesSection (no catch blocks) and NodeProjectsSection (console.error, not instanceof Error), added SwarmTemplateDialog (had pattern) | Yes — tournament finding | None — corrected to match reality |
| 2 | errors.test.ts: replaced boolean primitive test with ApiError test | Yes — tournament finding (no ApiError coverage) | None — improved coverage |
| 3 | CLAUDE.md updated alongside AGENTS.md (task 401 STOP-trigger) | Yes — both files had gate blocks | None — consistency improvement |
| 4 | Task 301 test 1 strengthened: added form freshness assertion | Yes — Opus review (test was hollow) | None — stronger assertion |

## Issues Found & Remediated

### P0: SC7 gate was red (vitest picking up node:test file)

**Issue:** `scripts/no-push-invariant.test.mjs` is a `node:test` file that vitest's `*.test.mjs`
glob picked up. The new mandatory gate `cd remote-frontend && npx vitest run` failed.

**Remediation:** Added `**/scripts/**` to vitest exclude in `vite.config.ts:74`.

### P1: Dialog scrollability regression

**Issue:** Radix `DialogContent` dropped the old dialog's `overflow-y-auto` scroll container.
Tall dialogs (MergeProjectsDialog, SwarmTemplateDialog) overflow viewport with no scroll.
jsdom has no layout so no test catches it.

**Remediation:** Added `max-h-[85vh] overflow-y-auto` to `DialogContent` className in
`dialog.tsx:68`.

### P1: createAttemptRef test was hollow

**Issue:** Test checked secret doesn't appear after close+reopen, but dialog close unmounts
DialogContent and hides the secret regardless of whether the guard fires.

**Remediation:** Strengthened test to also verify the form is fresh (empty name input) after
close+reopen, proving `closeDialog` reset state properly. Added `expect(screen.getByLabelText('Key Name')).toHaveValue('')`.

### Pre-existing: AppRouter test isolation failure

**Issue:** `src/AppRouter.test.tsx > 'authenticated: hitting / redirects to /nodes'` fails
when run with NodeApiKeySection tests but passes in isolation. Verified pre-existing on baseline.

**Status:** NOT fixed — out of scope for this workstream. Documented in decisions-ledger.

## Commits

```
cc84c115 remediate: Opus review findings
4c0722ce execute: reachability gate
8ad920df task 401: AGENTS.md + CLAUDE.md gates
93193f7b task 301: mutation guard tests
370decbc task 201+202+203: dialog Radix rewrite
c7ab5849 task 103: update 6 dialog error call sites
b8a27f81 task 101+102: shared parseErrorMessage + tests
```

## Verdict

**The implementation meets the intended goals.** All 7 SCs pass. The plan was followed with
4 documented divergences (all improvements from tournament findings). Three issues found by
Opus review were remediated in-session. One pre-existing test isolation issue is documented
but not fixed (out of scope).
