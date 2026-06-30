# Gap analysis — ui-overhaul vs current implementation

Every finding is verified against a real source file:line. Unverified observations are excluded.

Classification:
- **(A)** Bug in shipped code — design and intended behaviour agree; impl is wrong
- **(B)** Divergent — built, but differs from design (colour, spacing, variant, copy)
- **(C)** Un-built — in the design, not yet implemented at all

---

## 1. Color tokens

| element | class | file:line | proposed action |
|---|---|---|---|
| `--vks-void` HSL resolves to `#080810` not `#0a0a0f` | B | `frontend/src/styles/index.css` `--vks-void: 240 33% 5%` | Change to `240 20% 5%` |
| `--vks-surface-bright` resolves to `#1a1a22` not `#1a1a24` | B | `index.css` `--vks-surface-bright: 240 14% 12%` | Change saturation to `16%` |
| `--vks-cyan` resolves to `#00c7ff` not `#00d4ff` | B | `index.css` `--vks-cyan: 193 100% 50%` | Change hue to `190` |
| `--border-strong` token missing entirely | C | not defined anywhere | Add `--vks-border-strong` + semantic alias |
| `--surface-card` / `--surface-raised` semantic aliases missing | C | not defined in index.css | Add aliases: `--surface-card: var(--vks-surface)`, etc. |
| Light theme `--primary` maps to `--_muted` not teal `#0091b5` | B | `index.css` light `:root` block | Set `--_primary: 192 100% 35%` in light `:root` |
| `--status-todo/inprogress/inreview/done/cancelled` tokens missing | C | not defined in index.css or tailwind.config.js | Define all `--status-*` vars in `.vks-theme` |
| `TaskCountPills` swaps inprogress/inreview colours | A | `frontend/src/components/tasks/TaskCountPills.tsx:45,52` | Swap: inprogress→blue, inreview→amber |

## 2. TaskCard

| element | class | file:line | proposed action |
|---|---|---|---|
| Status strip uses Tailwind `bg-*` not `--status-*` tokens | B | `TaskCard.tsx:27-33` `statusStripColors` map | Switch to `var(--status-{status})` after tokens added |
| `done` strip: `bg-green-500` (#22c55e) not `#00ff88` | B | `TaskCard.tsx:31` | Use `--status-done` token (emerald #00ff88) |
| `cancelled` strip: `bg-red-500` (#ef4444) not `#ff6b6b` | B | `TaskCard.tsx:32` | Use `--status-cancelled` token (coral #ff6b6b) |
| `inreview` strip: `bg-amber-500` (#f59e0b) not `#ffb800` | B | `TaskCard.tsx:30` | Use `--status-inreview` token |
| Card title: `font-light text-sm` not `font-medium text-base` | B | `TaskCardHeader.tsx:52` | Change to `font-medium text-base` |
| Description: `text-xs` not `text-sm` | B | `TaskCard.tsx:268` | Change to `text-sm` |
| Description: double-truncation (JS + CSS) | A | `TaskCard.tsx:268` + `truncateDescription` util | Remove JS pre-truncation; rely on CSS `truncate` |
| Node tag: missing `font-code` class | B | `TaskCard.tsx:278-280` | Add `font-mono`/`font-code` to node tag `<span>` |
| Label badges: solid colour fill not outline variant | B | `LabelBadge.tsx:51-63` | Add outline variant for task card context |
| Days badge: age-graduated colours instead of flat secondary | B | `DaysInColumnBadge.tsx:37-46` | Use flat `secondary` variant; remove colour graduation |
| Days format: capped `7d+` not literal `{n}d` | B | `daysInColumn.ts:33-44` | Return literal day count without cap |
| AttemptIndicator merged: `text-green-500` not `text-success` | B | `TaskCard.tsx:244-246` | Change to `text-success` for theme portability |
| Selected state: `ring-inset ring-secondary-foreground` not `ring-primary` (outset) | B | `kanban/index.tsx:118` | Change to `ring-2 ring-primary` (no `ring-inset`) |
| Hover: no `border-strong` on hover | B | `TaskCard.tsx:212` | Add `hover:border-border-strong` once token exists |
| `--strip-width: 4px` token missing | C | not defined | Add CSS custom property (optional) |

## 3. Kanban board

| element | class | file:line | proposed action |
|---|---|---|---|
| Board grid: `minmax(200px,400px)` not `minmax(264px,1fr)` | B | `kanban/index.tsx:343` `auto-cols-[minmax(200px,400px)]` | Change to `auto-cols-[minmax(264px,1fr)]` |
| Column status dot: 10px (`h-2.5`) not 9px | B | `kanban/index.tsx:220` | Change to `h-[9px] w-[9px]` |
| Column count badge: `bg-muted` not `surface-card` | B | `kanban/index.tsx:226-232` | Add `--surface-card` token and use it |
| Column add button: `h-0` collapses it visually | A | `kanban/index.tsx:243-250` | Fix to `h-6 w-6` ghost icon button |
| Column empty state: ansi-dither texture + `░▒ no tasks ▒░` | C | not implemented — empty column renders nothing | Add empty state div with `.vks-ansi-dither.vks-scanlines` |

## 4. Navbar / chrome

| element | class | file:line | proposed action |
|---|---|---|---|
| Wordmark: uses SVG `<Logo>` not `<VKSLogo>` component | B | `Navbar.tsx:142` | Swap to `<VKSLogo>` |
| Wordmark: Chivo Mono not applied (uses `font-code` = JetBrains Mono) | B | `VKSLogo.tsx:22` | Apply `font-family: 'Chivo Mono'` inline or add `font-wordmark` Tailwind token |
| Tab row (Board / Nodes / Processes) with 2px cyan underline | C | not implemented in `Navbar.tsx` | Add second `<nav>` row with 3 tabs |
| Nodes and Processes views / routes | C | no `/nodes` route exists | Create route + view component |
| Theme toggle (sun/moon) | C | not found in `Navbar.tsx` | Build `ThemeToggle` ghost icon button |
| `+ Task` button: ghost icon, not `primary sm` with text | B | `Navbar.tsx:203-211` | Change to `variant="default" size="sm"` + "+ Task" label |
| Project switcher: missing folder icon | B | `ProjectSwitcher.tsx:104-116` | Add `<FolderOpen>` icon inside trigger |
| Search input: `w-64`/`w-72` not 260px | B | `SearchBar.tsx:23` | Change to `w-[260px]` |

## 5. NodeCard component

| element | class | file:line | proposed action |
|---|---|---|---|
| `NodeCard` component | C | not found in `frontend/src/components/` | Build component per spec anatomy |
| OS SVG glyphs (mac/linux/windows) in raised-bg 36×36 container | C | `NodeProjectsSection.tsx:313-339` uses plain `<Monitor>` | Add per-OS SVG paths to NodeCard |
| Node name with `font-code` | C | `NodeProjectsSection.tsx:324` no `font-code` | Apply in new NodeCard |
| Online pulse dot: 8px emerald + `vks-pulse` 2s animation | C | keyframe not defined in `index.css` | Define `vks-pulse` keyframe; add to NodeCard |
| Offline dot: text-dim, no animation | C | not implemented | Add offline variant to NodeCard |
| Nodes grid view: `repeat(auto-fill, minmax(320px, 1fr))` | C | no Nodes view page | Build as part of Nodes view |

## 6. Task drawer / detail panel

| element | class | file:line | proposed action |
|---|---|---|---|
| 460px slide-from-right drawer architecture | C | detail is a resizable split panel (`TasksLayout.tsx`, `TaskAttemptPanel.tsx`) | Evaluate: add Sheet/Drawer wrapper OR keep panel and align internals to spec |
| Drawer header: StatusBadge dot + title + close X | B | `AttemptHeaderActions.tsx` — icon toggles only, no status dot | Add `StatusBadge` + task title in header |
| Badges row: status + node + labels | C | not found in panel header | Build badges row below drawer header |
| Tabs: Diff / Logs / Attempts (labeled) | B | `AttemptHeaderActions.tsx:59-147` — icon-only `ToggleGroup`; no Attempts tab | Replace with labeled `Tabs` component |
| Merge button: `primary sm flex-1` (currently `outline xs`) | B | `GitOperations.tsx:401-453` | Change Merge to `variant="default" size="sm" className="flex-1"` |
| Rebase button: `outline sm` (currently `outline xs`) | B | `GitOperations.tsx` | Change to `size="sm"` |
| Open in IDE in footer (ghost sm) | C | not in footer action bar | Add ghost sm "Open in IDE" button to footer |

## 7. Typography

| element | class | file:line | proposed action |
|---|---|---|---|
| Body 14px base | A ✓ | `tailwind.config.js:47` | Already correct |
| Inter UI font | A ✓ | `index.css:15` | Already correct |
| JetBrains Mono code font | A ✓ | `index.css:16` | Already correct |
| Source Serif 4 defined but applied nowhere | B | `index.css:95` `.vks-theme` only — no heading uses `font-heading` class | Apply `font-heading` to section headings (Hive, Processes, etc.) |
| Chivo Mono for wordmark (not Tailwind `font-code`) | B | `VKSLogo.tsx:22` | Inline `font-family: 'Chivo Mono'` or add `font-wordmark` Tailwind key |

---

## Summary counts

| Class | Count | Interpretation |
|---|---|---|
| (A) Bug — wrong in shipped code | 5 | Must fix |
| (B) Divergent — built but differs | 35 | Fidelity pass |
| (C) Un-built — not implemented | 17 | New scope |

**Decision: C-class scope present → WAI pipeline required.**

Routing to WAI spec at `docs/superpowers/specs/2026-06-28-ui-overhaul.md`.
