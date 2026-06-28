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

## Appended during execute

<!-- executor appends below -->
