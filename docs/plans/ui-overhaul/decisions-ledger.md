---
topic: ui-overhaul
spec: docs/superpowers/specs/2026-06-28-ui-overhaul.md
---

# Decisions ledger — ui-overhaul

The executor appends one entry per non-trivial choice made during `/wai:execute`. Decompose
seeds the entries below (resolved at decomposition time, before any code).

## Seeded at decompose

- **Scope (C) confirmed by user.** The `--vks-*` brand palette + `--_*`→brand remap live only in
  the dead `.vks-theme` block (never applied by `ThemeProvider`), so Midnight Terminal rendered
  nowhere. User chose the full design (not SC-minimal). Task 001 promotes primitives to `:root` and
  merges the remap into `.dark`; SC22 gates it. Recorded as spec D10 + this entry. App-wide blast
  radius → SC19 (WCAG) is app-wide. See spec `## Decisions` D10.
- **`.vks-theme` retained, not deleted.** `frontend/src/__tests__/DesignSystem.test.tsx` references
  the class. Task 001 leaves the block in place; the remap is duplicated into `.dark`. Redundant but
  harmless; avoids breaking the test.
- **Stale spec paths corrected (resolve-in-decompose, advisor-sanctioned).** Real paths used in tasks:
  `VKSLogo.tsx` → `frontend/src/components/VKSLogo.tsx` (not `ui/`); `GitOperations.tsx` →
  `frontend/src/components/tasks/Toolbar/GitOperations.tsx`; `LabelBadge.tsx` →
  `frontend/src/components/labels/LabelBadge.tsx`; `daysInColumn` util →
  `frontend/src/utils/daysInColumn.ts`; nodes hook → `frontend/src/hooks/useAvailableNodes.ts`.
  Spec file-path register updated to match before re-freeze.
- **`StatusBadge` is NEW (task 014).** No generic task-status badge exists (only
  `ConnectionStatusBadge` for connection state). SC18 names `StatusBadge`; task 014 builds a minimal
  one. Sibling read: `ConnectionStatusBadge.tsx`.
- **`.light {}` block is NEW (task 001).** It does not exist today (light = `:root` default).
  `ThemeProvider` adds the `.light` class, so a created block applies.
- **Navbar currently imports `Logo` from `@/components/Logo`** (not VKSLogo). Task 015 swaps it.

## plan-lint sibling warnings acknowledged (SC6 backstop)

`wai-plan-lint.sh` emits same-directory `W:` advisories for each `create` task. The pattern-sibling
each task actually read (cross-directory where relevant) is named in the task's "sibling read" note;
the flagged same-dir neighbours are NOT pattern siblings:
- **012 ThemeToggle** vs `AgentAvailabilityIndicator.tsx` — not a sibling (availability indicator,
  not a theme control). Real pattern read: an existing ghost-icon `Button` usage + `ThemeProvider`/
  `GeneralSettings` for the `updateAndSaveConfig`+`setTheme` persistence pattern (SC20).
- **014 StatusBadge** vs `ConnectionStatusBadge.tsx` — `ConnectionStatusBadge` IS the pattern sibling
  and WAS read (static `Record<status,…>` map, `cn()` merge, named export mirrored). It is a read-only
  reference, not edited, so it is correctly absent from `files:`.
- **017 Nodes page** vs `AllProjectsTasks.tsx` — page exemplar read was `Processes.tsx` (the global
  route this one parallels), more relevant than `AllProjectsTasks`.
- **018 NodeCard** vs `MergeLabelsDialog.tsx` — not a sibling (dialog). Pattern sibling read:
  `NodeProjectsSection.tsx` (the existing node-row renderer).

## SC18 task-detail scope reductions (flagged for execute + review)

Tasks 020/021 surgically scoped SC18 against the real code (documented per "narrow-but-correct"):
- **Title** ("header renders task title"): already rendered by the parent `NewCardHeader` breadcrumb
  (`ProjectTasks.tsx`); 020 adds the `StatusBadge` dot but does NOT duplicate the title.
- **Badges row** (status/node/labels): status reachable (`task.status`), node reachable
  (`task.source_node_name`); **labels are not on `AttemptHeaderActions` props** → labels deferred;
  020 scoped to the Tabs replacement + StatusBadge dot. Full badges row flagged for review.
- **Open-in-IDE** (footer ghost sm): does NOT exist in `GitOperations.tsx` (lives in `ActionsDropdown`);
  021 does not fabricate it.
- **`processes` LayoutMode**: the new 3-tab switcher covers diffs/terminal/null only; `processes`
  (and `preview`/`files`) reachability via other surfaces must be confirmed at execute (020 STOP).

## 006 parity note

Sub-edits text-xs→text-sm (description) and text-green-500→text-success (CheckCircle) were authored
for `TaskCard.tsx`; `AllProjectsTaskCard.tsx` carries identical lines. 006's `files:` includes
AllProjectsTaskCard (for the truncation removal) — parity edits there are in-scope; flagged for the
breakdown review to confirm whether visual parity across both card variants is required.

## Adversarial breakdown review — round 1 (Opus + Codex + Gemini, all REVISE) → remediated

3-model review of the breakdown (verdicts in `reviews/`). Verified findings + fixes:
- **SC18 fully implemented (user decision).** 020 expanded: literal English tab labels (Diff/Logs/
  Attempts + TODO(i18n), not the mismatched existing keys "Diffs"/"Terminal"); StatusBadge dot;
  badges cluster = `StatusBadge showLabel` + secondary node badge (`task.source_node_name`) + outline
  `LabelBadge`s sourced via `useTaskLabels(task.id)` (the hook TaskCard uses). 021 expanded: ghost-sm
  "Open in IDE" wired to `useOpenInEditor(selectedAttempt.id)` (same hook as `ActionsDropdown`).
  - **Placement nuance:** `NewCardHeader` renders its `actions` slot as a top-right inline flex row,
    so the badges render as an inline cluster in the header actions area, NOT a literal separate band
    "below header" (a literal band would require editing `new-card.tsx`/`ProjectTasks.tsx`, outside
    020's `files:`). All three badge types are present — accepted as faithful-enough; a literal band
    is cosmetic follow-up if ever wanted.
  - **021 IDE button** inherits the footer's `branchStatus &&` gate (hidden while branchStatus is
    null), unlike the dropdown which gates only on attempt id — accepted (footer is a git-ops surface).
- **Task 003 ANSI dither (Opus):** design-source `var(--background)`/`var(--border)` are bare triplets
  in this product → invalid as raw CSS colours. Fixed: wrap in `hsl(...)` (scanlines rgb() body stays).
- **SC7 "consumed" clause (Gemini):** `--border-strong` was defined (002) but consumed by no component.
  Fixed: task 006 adds `hover:border-[hsl(var(--border-strong))]` to the TaskCard root. 022 now greps
  the consumption side too.
- **Spec corrections (re-frozen via 2nd precheck):** SC10/Approach `text-muted`→`text-muted-foreground`
  (bare `text-muted` paints the muted *background* colour); status-token shorthand `bg-[var(--status-*)]`
  → `bg-[hsl(var(--status-*))]` throughout (bare var() of a triplet renders nothing); NodeCard offline
  dot `var(--text-dim)`→`hsl(var(--vks-text-dim))`; SC7 grep "expected 3"→"≥3 (6 with light)" + a
  consumption grep added.
- **Minor (Opus/Codex):** 001 Before-range relabelled ~57–79 with a note that `--_neutral`/
  `--_neutral-foreground` stay in place; 022 `files:` now lists the decisions-ledger (it records results).

## Adversarial breakdown review — round 2 (re-review) → remediated

Opus + Gemini re-reviewed; both REVISE on real findings. (Codex's round-2 pass was a review-type
error — it checked the *source files* for the changes and reported them "missing"; but no code is
written at decompose time, so all 13 were invalid except its #13, which Gemini also raised. Discarded
the 12 category-error findings.) Verified findings + fixes:
- **SC18 status collapsed (Opus, blocking):** frozen SC18 requires TWO status renderings — a header
  `StatusBadge` dot (spec:96) AND a row "status outline+dot" badge (spec:97). Task 020 had collapsed
  to one labeled badge and asserted "exactly one." Fixed: 020 now renders a leading bare
  `<StatusBadge status={task.status} />` (header dot) PLUS an outline `<StatusBadge … showLabel
  className="…border…" />` (row badge); grep assertion → 2. Both sit in the actions-slot cluster (the
  title lives in `ProjectTasks.tsx`, out of 020's scope — same placement constraint as the rest).
- **Task 022 stale Open-in-IDE note (Gemini/Codex#13):** note said "no Open-in-IDE in footer" but 021
  now adds it. Fixed: note + SC17 item now say the ghost button is in the footer (task 021).
- **Spec residual bare shorthand (Gemini):** `bg-[var(--surface-card)]` (spec:286) → `bg-[hsl(var(--surface-card))]`.
  Swept the whole spec — no bare `(bg|text|border)-[var(--…)]` references remain. Re-frozen (3rd precheck).

## Adversarial breakdown review — round 3 → APPROVE

Opus APPROVE, Gemini APPROVE. Codex raised one prose nit (task 022 said both "no file is modified" and
"only the ledger is written" — a leftover from the files:[]→ledger change); fixed (the sentence now
states the only modified file is the ledger, listed in `files:`). Non-substantive; not re-paneled.
Gate reached: breakdown APPROVED. Ready for `/wai:execute ui-overhaul`. See `reviews/breakdown-3.md`.

## Appended during execute

### Task 001 — Complete (no decisions)
- Manual verification: all four manual checks passed (SC22 grep, --vks-void anchor, .light block, DesignSystem.test.tsx).
- TypeScript: `cd frontend && npx tsc --noEmit` passes.
- Test: `cd frontend && npx vitest run src/__tests__/DesignSystem.test.tsx` — 14 tests pass.
- No undictated choices made; task specification fully followed.

### Task 007 — Complete (no decisions)
- Manual verification: `inprogress` uses `text-blue-600 dark:text-blue-400` + `bg-blue-50/50 …`; `inreview` uses `text-amber-600 dark:text-amber-400` + `bg-amber-50/50 …`.
- SC5a check: `grep -nE 'bg-amber-500|bg-blue-500|bg-green-500|bg-red-500' frontend/src/components/projects/TaskCountPills.tsx` → no matches (exit 1).
- TypeScript: `cd frontend && npx tsc --noEmit` passes.
- No undictated choices made; task specification fully followed.

### Task 008 — Complete (no decisions)
- Removed `getDaysStyle` function from `frontend/src/utils/daysInColumn.ts` (only consumers were `DaysInColumnBadge.tsx` and test file; both in `files:` and updated).
- Removed `7d+` cap from `formatDaysInColumn`, returns literal `{n}d` for any day count >= 1.
- Flattened `DaysInColumnBadge` to use `bg-secondary text-secondary-foreground` style (no age-graduated colours).
- Updated test expectations: `7d+` → `7d`, `14d`, `100d`.
- Manual verification: `grep -rn 'getDaysStyle' frontend/src` → no matches; `grep -n 'bg-secondary text-secondary-foreground' frontend/src/components/tasks/DaysInColumnBadge.tsx` → match at line 30.
- TypeScript: `cd frontend && npx tsc --noEmit` passes.
- Test: `cd frontend && npx vitest run src/utils/__tests__/daysInColumn.test.ts` — 17 tests pass.
- No undictated choices made; task specification fully followed.
