# Code Review — Round 1

**Target:** worktree-bridge-cse_01SHcfL9XSiArcLSCnKmJ49p   **Range:** `main..HEAD`   **Effort:** high

## Findings

| # | File:line | Severity | Category | Finding | Confidence | Actionable? |
|---|-----------|----------|----------|---------|-----------|-------------|
| 1 | `frontend/src/components/layout/Navbar.tsx:379` | high | correctness | `<DialogHeader>/<DialogTitle>` placed outside `<DialogContent>` in the mobile-search Dialog. Radix's `DialogContent` renders in a portal (body-appended). The `aria-labelledby` the portal emits cannot reach a title that lives in the main document flow; the sr-only accessible name is silently broken for assistive tech. | high | yes |
| 2 | `frontend/src/App.tsx:134` | medium | correctness | `isSignedIn` is listed in the `useEffect` dependency array (line 134) but is never read inside the effect body (declared at line 87). The effect re-runs on every auth-state change, potentially re-triggering the disclaimer / onboarding / release-notes dialog sequence immediately after a sign-in transition. | high | yes |
| 3 | `frontend/src/components/tasks/AllProjectsTaskCard.tsx:123` | low | quality | `CheckCircle` still uses hardcoded `text-green-500` instead of semantic `text-success`. Task 006 intentionally scoped this sub-edit to `TaskCard.tsx` only, but the parity note in the decisions-ledger explicitly deferred resolution to the code-review gate. The fix is one class rename. | high | yes |
| 4 | `frontend/src/components/labels/LabelBadge.tsx:91` | medium | correctness | `onClick` is forwarded to a `<span>` with no `role="button"`, `tabIndex={0}`, or `onKeyDown` handler. The element is not keyboard-focusable and screen readers will not identify it as interactive — fails WCAG 2.1 SC 4.1.2 and SC 2.1.1. Task 009 (variant="outline") modified this file; the gap is within scope. | high | yes |
| 5 | `frontend/src/styles/index.css:137` | high | correctness | The `.light` block (created by Task 001) omits `--_secondary-foreground`, so it inherits `:root`'s `215.4 16.3% 70.9%` (medium-light bluish-gray). The secondary background also falls back to `:root` white (`0 0% 100%`). Computed contrast ≈ 2.1:1, below WCAG AA (4.5:1 normal text). Affects `DaysInColumnBadge`, `badge.tsx` secondary variant, `button.tsx` secondary variant, and form placeholder text in explicit light mode. | high | yes |

## Non-actionable

| # | File:line | Severity | Category | Finding | Confidence | Why non-actionable |
|---|-----------|----------|----------|---------|-----------|---------------------|
| 6 | `frontend/src/components/labels/LabelBadge.tsx:18` + `frontend/src/components/swarm/SwarmLabelDialog.tsx:213` | low | quality | `getContrastColor` is implemented identically in both files. Extracting to a shared util would improve maintainability. | high | `SwarmLabelDialog.tsx` is outside this diff's scope; duplication pre-dates the overhaul. Log for a dedicated cleanup session. |
| 7 | `frontend/src/components/tasks/Toolbar/GitOperations.tsx:226` | low | quality | `CreatePRDialog.show()` is called without `await`. Other async handlers in the same function `await` their dialog calls. | medium | Pre-existing code path; Task 021 did not touch the `CreatePRDialog.show()` call site. `show()` on an imperative dialog controller is fire-and-forget by design in this codebase. Not introduced by this diff. |
| 8 | `frontend/src/styles/index.css:35` | medium | quality | `:root` default `--_secondary-foreground: 215.4 16.3% 70.9%` produces only ≈ 2.1:1 contrast on white — the same design-system default that shipped with shadcn/ui. This value is now superseded in explicit `.light` mode by the fix for finding #5 (which adds `--_secondary-foreground: 222.2 47.4% 11.2%` to the `.light` block). The `:root` value remains active only when no theme class is applied (OS-level system-light without explicit toggle). | low | Pre-existing shadcn default; rendered moot in explicit light mode by the finding #5 fix. The no-class-applied edge case is pre-existing and out of scope. |

## Verdict: Request changes

Actionable: [1,2,3,4,5]
