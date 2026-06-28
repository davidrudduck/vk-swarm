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

### Task 002 — Complete (no decisions)
- Added five status tokens to `.dark {}` block: `--status-todo: 240 5% 63%`, `--status-inprogress: 217 91% 60%`, `--status-inreview: 43 100% 50%`, `--status-done: 152 100% 50%`, `--status-cancelled: 0 100% 71%`.
- Added three semantic surface/border aliases to `.dark {}` block: `--surface-card: var(--vks-surface)`, `--surface-raised: var(--vks-surface-bright)`, `--border-strong: 240 10% 16%`.
- Added light status overrides to `.light {}` block: all five status tokens with light palette values + three aliases with light values.
- Manual verification (SC6): `grep -A 100 '\.dark {' frontend/src/styles/index.css | grep -- '--status-todo'` → match.
- Manual verification (SC7): `grep -E '\-\-(border-strong|surface-card|surface-raised)' frontend/src/styles/index.css` → 6 matches (3 dark, 3 light).
- Light overrides present: `grep -A 60 '\.light {' frontend/src/styles/index.css | grep -- '--status-done: 153 83% 30%'` → match.
- TypeScript: `cd frontend && npx tsc --noEmit` passes.
- Gate: `WAI_TYPECHECK_CMD="cd frontend && npx tsc --noEmit" WAI_TEST_CMD="true" bash ~/.claude/wai/scripts/task-gate.sh ui-overhaul 002` → CONFORMS (all deterministic gates passed).
- No undictated choices made; task specification fully followed.

### Task 010 — Complete (no decisions)
- Anchor 1 (selected-card ring): `grep -n "isOpen && 'ring-2 ring-secondary-foreground ring-inset'" frontend/src/components/ui/shadcn-io/kanban/index.tsx` → line 118 confirmed. Changed to `ring-2 ring-primary`.
- Anchor 2 (add-button size): `grep -n 'className="m-0 p-0 h-0 text-foreground/50 hover:text-foreground"' frontend/src/components/ui/shadcn-io/kanban/index.tsx` → line 244 confirmed. Changed `h-0` to `h-6 w-6`.
- Anchor 3 (status dot): `grep -n 'className="h-2.5 w-2.5 rounded-full shrink-0"' frontend/src/components/ui/shadcn-io/kanban/index.tsx` → line 220 confirmed. Changed to `h-[9px] w-[9px]`.
- Anchor 4 (count badge bg): `grep -n 'className="ml-0.5 px-1.5 py-0.5 rounded text-xs bg-muted text-muted-foreground font-normal tabular-nums"' frontend/src/components/ui/shadcn-io/kanban/index.tsx` → line 228 confirmed. Changed `bg-muted` to `bg-[hsl(var(--surface-card))]`.
- Manual verification (post-edit): all four greps from spec lines 85–89 passed.
- TypeScript: `cd frontend && npx tsc --noEmit` passes.
- No undictated choices made; task specification fully followed.

### Task 011 — Complete (no decisions)
- Anchor confirmed: `grep -n 'font-code font-bold tracking-tight select-none' frontend/src/components/VKSLogo.tsx` → lines 22, 50 (both occurrences present).
- No other `font-code` occurrences found: `grep -n 'font-code' frontend/src/components/VKSLogo.tsx` → only the two brand mark lines.
- No existing `font-wordmark`: `grep -n 'font-wordmark' frontend/src/components/VKSLogo.tsx` → no match (pre-change).
- Replacement applied: `font-code` → `font-wordmark` in both lines using `replace_all`.
- Manual verification (post-edit): `grep 'font-wordmark' frontend/src/components/VKSLogo.tsx` → 2 matches (lines 22, 50).
- Manual verification (post-edit): `grep 'font-code' frontend/src/components/VKSLogo.tsx` → no match (confirmed removal).
- TypeScript: `cd frontend && npx tsc --noEmit` passes.
- No undictated choices made; task specification fully followed.

### Task 012 — Complete (no decisions)
- `useTheme()` verified: exports `{ theme, setTheme }` from `ThemeProvider.tsx` (lines 11, 51).
- `useUserSystem()` verified: exports `updateAndSaveConfig: (updates: Partial<Config>) => Promise<boolean>` from `ConfigProvider.tsx` (lines 38, 191–210).
- `ThemeMode` verified: enum exported from `shared/types.ts` line 617 with `LIGHT`, `DARK`, `SYSTEM` variants.
- File created: `frontend/src/components/ThemeToggle.tsx` with full spec contents (lines 1–103).
- Manual verification: `grep -E 'export default ThemeToggle|export function ThemeToggle' frontend/src/components/ThemeToggle.tsx` → both exports present.
- TypeScript: `cd frontend && npx tsc --noEmit` passes (no errors).
- No undictated choices made; task specification fully followed.

### Task 019 — Complete (no decisions)
- Anchor 1 (lucide import, line 3): `grep -n "import { Check, ChevronsUpDown } from 'lucide-react';" frontend/src/components/layout/ProjectSwitcher.tsx` → confirmed.
- Anchor 2 (trigger button, lines 104–116): `sed -n '104,116p' frontend/src/components/layout/ProjectSwitcher.tsx` → confirmed exact match to Before text.
- Anchor 3 (SearchBar width, line 23): `grep -n "w-64 sm:w-72" frontend/src/components/SearchBar.tsx` → confirmed present.
- Edit 1: Added `FolderOpen` to lucide-react import (line 3); `grep 'FolderOpen' frontend/src/components/layout/ProjectSwitcher.tsx` → 2 matches (import + JSX).
- Edit 2: Added `<FolderOpen className="mr-2 h-4 w-4 shrink-0 opacity-70" />` before `<span className="truncate">{displayValue}</span>` in trigger.
- Edit 3: Changed width from `w-64 sm:w-72` to `w-[260px]` in SearchBar wrapper div.
- Manual verification: `grep -c 'w-64 sm:w-72' frontend/src/components/SearchBar.tsx` → 0 (old width removed); `grep 'w-\[260px\]' frontend/src/components/SearchBar.tsx` → match.
- TypeScript: `cd frontend && npx tsc --noEmit` passes.
- Gate: `WAI_TYPECHECK_CMD="cd frontend && npx tsc --noEmit" WAI_TEST_CMD="true" bash ~/.claude/wai/scripts/task-gate.sh ui-overhaul 019` → CONFORMS (all deterministic gates passed).
- No undictated choices made; task specification fully followed.

### Task 003 — Complete (no decisions)
- Added shadow/glow token block inside `.dark {}` block (after status tokens, before semantic aliases): `--shadow-sm`, `--shadow-md`, `--shadow-lg`, `--glow-cyan`, `--glow-emerald`.
- Added `vks-pulse` keyframe at file top level (after @import, before @tailwind base). Keyframe defines 0%/100% and 50% states with opacity + box-shadow variants.
- Added four ANSI texture utility classes at file top level: `.vks-ansi-dither`, `.vks-ansi-dither-dense`, `.vks-scanlines`, `.vks-scanlines::after`. Colour tokens (`--background`, `--border`) wrapped in `hsl(...)` (product tokens are bare HSL triplets); `.vks-scanlines::after` rgb() body copied as-is from design-source.
- Manual verification: `grep 'vks-pulse' frontend/src/styles/index.css` → match (SC15).
- Manual verification: `grep 'vks-ansi-dither' frontend/src/styles/index.css` → 2 matches (dither + dither-dense).
- Manual verification: `grep 'glow-cyan' frontend/src/styles/index.css` → match.
- Manual verification: `grep 'vks-ansi-dither {' frontend/src/styles/index.css | grep -q 'hsl(var(--background))'` → match (token wrapper applied).
- TypeScript: `cd frontend && npx tsc --noEmit` passes.
- No undictated choices made; task specification fully followed.

### Task 021 — Complete (no decisions)
- Anchor 1 (Merge button outline styling): `grep -n "border-success text-success hover:bg-success" frontend/src/components/tasks/Toolbar/GitOperations.tsx` → line 415 (pre-change).
- Edit 1: Merge button changed from `variant="outline" size="xs" className="border-success text-success hover:bg-success gap-1 shrink-0"` to `variant="default" size="sm" className="flex-1 gap-1 shrink-0"`.
- Edit 2: Rebase button changed `size="xs"` to `size="sm"`; `variant="outline"` and classes unchanged.
- Edit 3: Added `import { useOpenInEditor } from '@/hooks/useOpenInEditor';` (line 31).
- Edit 3: Added hook call `const openInEditor = useOpenInEditor(selectedAttempt.id);` (line 59).
- Edit 3: Added Open-in-IDE ghost-`sm` button after Rebase button (lines 466–476) with `onClick={() => openInEditor()}`, `ExternalLink` icon, literal "Open in IDE" label, and TODO(i18n) comment.
- Manual verification: `grep -n 'variant="default"' frontend/src/components/tasks/Toolbar/GitOperations.tsx` → line 415 (Merge button filled).
- Manual verification: `grep -n 'flex-1' frontend/src/components/tasks/Toolbar/GitOperations.tsx` → line 417 (Merge button full-width).
- Manual verification: `grep -c 'size="sm"' frontend/src/components/tasks/Toolbar/GitOperations.tsx` → 3 (Merge + Rebase + Open-in-IDE).
- Manual verification: `grep -n 'variant="ghost"' frontend/src/components/tasks/Toolbar/GitOperations.tsx` → lines 294, 469 (Settings button + new IDE button).
- Manual verification: `grep -n 'useOpenInEditor' frontend/src/components/tasks/Toolbar/GitOperations.tsx` → lines 31 (import), 59 (call).
- Manual verification: `grep -n 'aria-label="Open in IDE"' frontend/src/components/tasks/Toolbar/GitOperations.tsx` → line 472 (Open-in-IDE button).
- TypeScript: `cd frontend && npx tsc --noEmit` passes (no errors).
- Gate: `WAI_TYPECHECK_CMD="cd frontend && npx tsc --noEmit" WAI_TEST_CMD="true" bash ~/.claude/wai/scripts/task-gate.sh ui-overhaul 021` → CONFORMS (all deterministic gates passed).
- No undictated choices made; task specification fully followed.

### Task 005 — Complete (no decisions)
- Anchor 1 (TaskCard.tsx statusStripColors): `grep -n "done: 'before:bg-green-500'" frontend/src/components/tasks/TaskCard.tsx` → line 31 (pre-change).
- Anchor 2 (AllProjectsTaskCard.tsx statusStripColors): `grep -n "done: 'before:bg-green-500'" frontend/src/components/tasks/AllProjectsTaskCard.tsx` → line 16 (pre-change).
- Both maps verified identical (Before text matched exactly).
- Edit 1 (TaskCard.tsx): Replaced statusStripColors map lines 27–33 with token-based hsl(var(--status-*)) values.
- Edit 2 (AllProjectsTaskCard.tsx): Replaced statusStripColors map lines 12–18 with identical token-based values.
- Sibling alignment: Maps are identical by spec (no divergence to justify).
- Width difference (4px vs 3px) noted; outside scope.
- Manual verification (SC5a): `grep -rE 'bg-green-500|bg-red-500|bg-amber-500|bg-blue-500' frontend/src/components/tasks/TaskCard.tsx frontend/src/components/tasks/AllProjectsTaskCard.tsx` → zero matches (old classes removed).
- Manual verification (SC5b): `grep 'var(--status-' frontend/src/components/tasks/TaskCard.tsx frontend/src/components/tasks/AllProjectsTaskCard.tsx` → 10 matches (5 per file, all status tokens present).
- TypeScript: `cd frontend && npx tsc --noEmit` passes.
- Reconciliation note: Task spec mentions shorthand `before:bg-[var(--status-done)]` quoted in plan.md contracts but correctly applied as `before:bg-[hsl(var(--status-done))]` (HSL wrapper required for bare triplet tokens; satisfies SC5b's `var(--status-` grep).
- Gate: `WAI_TYPECHECK_CMD="cd frontend && npx tsc --noEmit" WAI_TEST_CMD="true" bash ~/.claude/wai/scripts/task-gate.sh ui-overhaul 005` → CONFORMS (all deterministic gates passed).
- No undictated choices made; task specification fully followed.

### Task 014 — Complete (no decisions)
- Pre-flight: `frontend/src/components/common/StatusBadge.tsx` did not exist (DOES_NOT_EXIST confirmed).
- Pre-flight: TaskStatus verified as `"todo" | "inprogress" | "inreview" | "done" | "cancelled"` in `shared/types.ts` line 352.
- Pre-flight: Status tokens verified in `frontend/src/styles/index.css`: all five tokens present in both `.dark {}` and `.light {}` blocks (task 002 applied).
- File created: `frontend/src/components/common/StatusBadge.tsx` with full spec contents (lines 1–49).
- Sibling pattern verified: static `Record<TaskStatus, { dotClass: string; label: string }>` map per `ConnectionStatusBadge` pattern; named export `export function StatusBadge`; `cn()` merge for className; literals vs template-literal confirmed.
- Manual verification: `grep -E 'export function StatusBadge|export const StatusBadge' frontend/src/components/common/StatusBadge.tsx` → "export function StatusBadge({" match (line 34).
- Manual verification: `grep 'bg-\[hsl(var(--status-' frontend/src/components/common/StatusBadge.tsx` → 5 literal matches (todo, inprogress, inreview, done, cancelled); confirms no template-literal interpolation.
- TypeScript: `cd frontend && npx tsc --noEmit` passes (no errors).
- Gate: `WAI_TYPECHECK_CMD="cd frontend && npx tsc --noEmit" WAI_TEST_CMD="true" bash ~/.claude/wai/scripts/task-gate.sh ui-overhaul 014` → CONFORMS (all deterministic gates passed).
- No undictated choices made; task specification fully followed.

### Task 006 — Complete (sub-edit 5 decision)

Sub-edits 1–4 applied without undictated choice: title font-medium text-base; description text-sm; node-tag font-mono; CheckCircle text-success.

**Sub-edit 5 divergence (documented as required):**
- **TaskCard.tsx (5a):** `truncateDescription` → `cleanDescription` (renamed for clarity). Kept markdown header stripping + whitespace collapse (cleaning is valuable). Dropped maxLength parameter and length-based truncation cap. Raw cleaned string flows to `<p className="... truncate">` where CSS handles visual cap.
- **AllProjectsTaskCard.tsx (5b):** Deleted pure length-cap helper entirely (no markdown cleaning lost). Call site binds raw `task.description` to `truncatedDesc` (guarded on null); `<p className="... truncate">` does visual cap via CSS.

Handling diverges per spec's intent: TaskCard retains cleaning logic; AllProjects drops a simple length cap. Both now rely on CSS truncate for visual bound instead of JS substring.

Sub-edit 6: Appended `hover:border-[hsl(var(--border-strong))]` to KanbanCard hover classes, consuming `--border-strong` token (SC7's "defined and consumed" requirement satisfied). Token recolours existing `border-b` edge on hover.

- Manual verification: all six greps from spec lines 221–226 passed.
- Manual verification: truncateDescription removed from AllProjectsTaskCard (grep -c = 0).
- TypeScript: `cd frontend && npx tsc --noEmit` passes (no unused-parameter error).
- Gate: `WAI_TYPECHECK_CMD="cd frontend && npx tsc --noEmit" WAI_TEST_CMD="true" bash ~/.claude/wai/scripts/task-gate.sh ui-overhaul 006` → CONFORMS (all deterministic gates passed).
- Noted (out of scope per spec): AllProjectsTaskCard carries same text-xs description and text-green-500 CheckCircle unmodified; asymmetry flagged as required by spec (sub-edits 2 & 4 scoped to TaskCard.tsx only).

### Task 013 — Complete (no decisions)
- Anchor 1 (line 21): `grep -n "import { type ReactNode, type Ref, type KeyboardEvent } from 'react'" frontend/src/components/ui/shadcn-io/kanban/index.tsx` → confirmed.
- Anchor 2 (line 343): `grep -n "minmax(200px,400px)" frontend/src/components/ui/shadcn-io/kanban/index.tsx` → confirmed.
- Anchor 3 (lines 144–146): `sed -n '144,146p' frontend/src/components/ui/shadcn-io/kanban/index.tsx` → `KanbanCards` renders `{children}` in single `<div>` — confirmed.
- Edit 1: Added `Children` value import (line 21); `grep -- 'import { Children,' frontend/src/components/ui/shadcn-io/kanban/index.tsx` → match.
- Edit 2: Changed grid minmax from `minmax(200px,400px)` to `minmax(264px,1fr)` (line 343); `grep 'minmax(264px' frontend/src/components/ui/shadcn-io/kanban/index.tsx` → match (SC9).
- Edit 3: Added empty-state block to KanbanCards with `Children.count(children) === 0` check; renders ANSI dither/scanlines block with "░▒ no tasks ▒░" text on empty; `grep 'no tasks' frontend/src/components/ui/shadcn-io/kanban/index.tsx` → match (SC10).
- Manual verification: `grep 'vks-ansi-dither vks-scanlines' frontend/src/components/ui/shadcn-io/kanban/index.tsx` → match.
- TypeScript: `cd frontend && npx tsc --noEmit` passes.
- Gate: `WAI_TYPECHECK_CMD="cd frontend && npx tsc --noEmit" WAI_TEST_CMD="true" bash ~/.claude/wai/scripts/task-gate.sh ui-overhaul 013` → CONFORMS (all deterministic gates passed).
- No undictated choices made; task specification fully followed.

### Task 018 — Complete (no decisions)
- Pre-flight: `frontend/src/components/swarm/NodeCard.tsx` does not exist (FILE_DOES_NOT_EXIST confirmed).
- Pre-flight: `Node` type in `@/types/nodes` verified: exposes `name: string` and `status: NodeStatus` (both required fields present).
- Pre-flight: `vks-pulse` keyframe verified: `grep 'vks-pulse' frontend/src/styles/index.css` → match (task 003 applied).
- Pre-flight: CSS tokens verified: `grep -E '(--surface-raised|--surface-card|--status-done)' frontend/src/styles/index.css` → 6 matches (task 002 applied).
- File created: `frontend/src/components/swarm/NodeCard.tsx` with full spec contents (lines 1–68). Exports named `NodeCard` function and type `export type NodeForCard = Node`.
- Decision recorded: `NodeForCard = Node` per task spec analysis. No global agent-count field exists on `Node` type; org-scoped source only. Right slot degrades to offline badge or muted status label (no fabricated agent-count).
- Manual verification: `grep -E 'export function NodeCard|export const NodeCard' frontend/src/components/swarm/NodeCard.tsx` → "export function NodeCard" match (SC14).
- Manual verification: `grep 'vks-pulse' frontend/src/components/swarm/NodeCard.tsx` → match (SC15 application in animate class).
- Manual verification: `grep -F 'export type NodeForCard = Node' frontend/src/components/swarm/NodeCard.tsx` → match (type for task 017 import).
- TypeScript: `cd frontend && npx tsc --noEmit` passes (no errors).
- No undictated choices made; task specification fully followed.

### Task 017 — Complete (no decisions)
- Pre-flight: `frontend/src/pages/Nodes.tsx` does not exist (FILE_DOES_NOT_EXIST confirmed).
- Pre-flight: `nodesApi.list(organizationId)` verified in `frontend/src/lib/api/nodes.ts` line 17; returns `Promise<Node[]>`.
- Pre-flight: `Node` type verified in `frontend/src/types/nodes.ts` line 18: exposes `id: string`, `name: string`, `status: NodeStatus` (all required fields present).
- Pre-flight: `NodeCard` named export verified: `grep 'export function NodeCard' frontend/src/components/swarm/NodeCard.tsx` → match (018 applied).
- Pre-flight: `Processes` is named export: `grep 'export function Processes' frontend/src/pages/Processes.tsx` → match (line 21).
- File created: `frontend/src/pages/Nodes.tsx` with full spec contents (lines 1–41). Exports named `Nodes` function.
- Edit 1 (App.tsx): Added import `import { Nodes } from '@/pages/Nodes';` after Processes import (line 11).
- Edit 2 (App.tsx): Added route `<Route path="/nodes" element={<Nodes />} />` after `/processes` route (line 155).
- Manual verification (SC21): `grep -F 'path="/nodes"' frontend/src/App.tsx` → match (line 155).
- Manual verification: `grep -F "import { Nodes } from '@/pages/Nodes'" frontend/src/App.tsx` → match (line 11).
- Manual verification (SC14 named export): `grep -E 'export function Nodes' frontend/src/pages/Nodes.tsx` → match (line 6).
- TypeScript: `cd frontend && npx tsc --noEmit` passes (no errors).
- Gate: `WAI_TYPECHECK_CMD="cd frontend && npx tsc --noEmit" WAI_TEST_CMD="true" bash ~/.claude/wai/scripts/task-gate.sh ui-overhaul 017` → CONFORMS (all deterministic gates passed).
- No undictated choices made; task specification fully followed.
- **Decision (data source):** Org-scoped node list used; default org selected per `useOrganizationSelection` logic (first non-personal org, fallback first org). No global cross-org endpoint needed (out of scope per D4).

### Task 015 — Complete (no decisions)
- Pre-flight: All six anchors confirmed byte-for-byte match before edits.
  - Anchor 1 (line 24): `import { Logo } from '@/components/Logo';` confirmed.
  - Anchor 2 (line 142): `<Logo className="h-4 sm:h-6 w-auto" />` confirmed.
  - Anchor 3 (lines 202-211): Plus button block with variant="ghost" size="icon" and `<Plus className="h-4 w-4" />` confirmed.
  - Anchor 4 (lines 11-23): lucide-react import with `Plus` in list confirmed.
  - Anchor 5 (line 49): `import { ProjectSwitcher } from './ProjectSwitcher';` confirmed.
  - Anchor 6 (lines 216-217): `<div className="flex items-center gap-1">` and `<ActivityFeed />` confirmed.
- Pre-flight: Producer files verified:
  - `frontend/src/components/VKSLogo.tsx` exports `VKSLogo` named export (line 18) and default export (line 60).
  - `frontend/src/components/ThemeToggle.tsx` exports `default ThemeToggle` (line 49).
- Edit 1: Line 24 — swapped `Logo` import for `VKSLogo` import: `import { VKSLogo } from '@/components/VKSLogo';`.
- Edit 2: Line 142 — swapped component: `<VKSLogo className="text-sm sm:text-base" />` (responsive font-size classes per spec note on wordmark legibility).
- Edit 3: Lines 202-211 — Plus icon button converted to text button: `variant="default"` `size="sm"`, text content `+ Task`, removed size="icon" and `h-9 w-9` classes, added TODO(i18n) comment.
- Edit 4: Lines 11-23 — removed `Plus` from lucide-react import (now unused after button change).
- Edit 5: Line 49 — added import: `import ThemeToggle from '@/components/ThemeToggle';` (default export).
- Edit 6: Lines 216-217 — rendered `<ThemeToggle />` in action cluster immediately before `<ActivityFeed />`.
- Manual verification (SC12): `grep '<VKSLogo' frontend/src/components/layout/Navbar.tsx` → match found.
- Manual verification (forbid_after): `grep -c '@/components/Logo' frontend/src/components/layout/Navbar.tsx` → 0 (Logo import removed).
- Manual verification (SC13): `grep '+ Task' frontend/src/components/layout/Navbar.tsx` → match found.
- Manual verification: `grep -c 'Plus' frontend/src/components/layout/Navbar.tsx` → 0 (Plus import removed, satisfies noUnusedLocals).
- Manual verification (SC16): `grep '<ThemeToggle' frontend/src/components/layout/Navbar.tsx` → match found (positioned before ActivityFeed).
- TypeScript: `cd frontend && npx tsc --noEmit` passes (no errors, confirms no unused imports).
- Gate: `WAI_TYPECHECK_CMD="cd frontend && npx tsc --noEmit" WAI_TEST_CMD="true" bash ~/.claude/wai/scripts/task-gate.sh ui-overhaul 015` → CONFORMS (all deterministic gates passed).
- No undictated choices made; task specification fully followed.

### Task 016 — Complete (no decisions)
- Pre-flight: All three anchors confirmed byte-for-byte match before edits.
  - Anchor 1 (line 2): `import { useCallback, useState } from 'react';` confirmed.
  - Anchor 2 (line 134): `const isOAuthLoggedIn = loginStatus?.status === 'loggedin';` followed by blank line and `return (` confirmed.
  - Anchor 3 (lines 317–322): closing divs `</div></div></div></div>` before `{/* Mobile search dialog */}` comment confirmed.
- Pre-flight: STOP triggers verified:
  - `grep '<VKSLogo' frontend/src/components/layout/Navbar.tsx` → match found (task 015 applied).
  - `grep "const { projectId, project } = useProject();" frontend/src/components/layout/Navbar.tsx` → match found (projectId available).
  - Close sequence matches Before text exactly (row structure unchanged).
- Edit 1 (line 2): Added `useEffect` to react import: `import { useCallback, useEffect, useState } from 'react';`.
- Edit 2 (after line 134): Added persistence effect with `useEffect`, `localStorage.setItem`, and `boardTo` computation.
- Edit 3 (after line 317): Inserted second nav row with three tabs (Board/Nodes/Processes), active-state classes with `border-b-2 border-primary`, and TODO(i18n) comments.
- Manual verification (Nodes tab): `grep -i 'Nodes' frontend/src/components/layout/Navbar.tsx` → 3 matches (route, pathname check, label).
- Manual verification (active underline): `grep 'border-b-2 border-primary' frontend/src/components/layout/Navbar.tsx` → 3 matches (one per tab).
- Manual verification (localStorage tracking): `grep "localStorage.setItem('lastVisitedProjectId'" frontend/src/components/layout/Navbar.tsx` → match found.
- TypeScript: `cd frontend && npx tsc --noEmit` passes (no errors).
- Gate: `WAI_TYPECHECK_CMD="cd frontend && npx tsc --noEmit" WAI_TEST_CMD="true" bash ~/.claude/wai/scripts/task-gate.sh ui-overhaul 016` → CONFORMS (all deterministic gates passed).
- No undictated choices made; task specification fully followed.
