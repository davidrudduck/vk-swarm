# Design spec — ui-overhaul (VK-Swarm "Midnight Terminal")

Source: `design-source/project/` — read that tree, not this doc, when in doubt.

---

## Tokens

### Color — dark theme (`:root` default)

| Token | Value | Role |
|---|---|---|
| `--background` | `#0a0a0f` | Void/page background |
| `--surface-card` | `#12121a` | Card / panel fill |
| `--surface-raised` | `#1a1a24` | Elevated surface (dropdown, hover bg) |
| `--surface-overlay` | `color-mix(in srgb, #0a0a0f 80%, transparent)` | Scrim overlay |
| `--foreground` | `#e4e4e7` | Body text |
| `--text-muted` | `#71717a` | Secondary/metadata text |
| `--text-dim` | `#3f3f46` | Tertiary / placeholder text |
| `--primary` | `#00d4ff` | Cyan — primary accent, buttons, links, focus rings |
| `--primary-foreground` | `#0a0a0f` | Text on primary-filled elements |
| `--accent` | `#a855f7` | Violet — secondary accent |
| `--border` | `#1a1a24` | Hairline borders (same as surface-raised) |
| `--border-strong` | `#2a2a38` | Stronger borders (inputs, buttons) |
| `--success` | `#00ff88` | Emerald — done/merged/online |
| `--warning` | `#ffb800` | Amber — in-review |
| `--danger` | `#ff6b6b` | Coral — cancelled/error/destructive |
| `--info` | `#00d4ff` | Cyan (same as primary) |
| `--status-todo` | `#a1a1aa` | Kanban: To Do column + strip |
| `--status-inprogress` | `#3b82f6` | Kanban: In Progress |
| `--status-inreview` | `#ffb800` | Kanban: In Review |
| `--status-done` | `#00ff88` | Kanban: Done |
| `--status-cancelled` | `#ff6b6b` | Kanban: Cancelled |
| `--console-bg` | `#0a0a0f` | Terminal/log viewer — always dark, never overridden by light theme |

### Color — light theme (`[data-theme="light"]`)

| Token | Value |
|---|---|
| `--background` | `#f5f6f9` |
| `--surface-card` | `#ffffff` |
| `--surface-raised` | `#eceef3` |
| `--foreground` | `#15171c` |
| `--primary` | `#0091b5` (deepened teal for AA contrast on white) |
| `--border` | `#e2e5ea` |
| `--border-strong` | `#ccd1da` |
| `--success` | `#0a8f57` |
| `--warning` | `#b06f00` |
| `--danger` | `#d23b3b` |
| `--status-todo` | `#6b7280` |
| `--status-inprogress` | `#2563eb` |
| `--status-inreview` | `#b06f00` |
| `--status-done` | `#0a8f57` |
| `--status-cancelled` | `#d23b3b` |

Console tokens (`--console-*`) intentionally NOT overridden in light mode.

### Typography

| Token | Value | Role |
|---|---|---|
| `--font-ui` | `'Inter', system-ui, sans-serif` | All product UI text |
| `--font-code` | `'JetBrains Mono', monospace` | Logs, diffs, node names, branches, durations |
| `--font-display` | `'Source Serif 4', Georgia, serif` | Section headings (Hive, Processes) |
| `--font-wordmark` | `'Chivo Mono', monospace` | VK-SWARM logo lockup |
| `--text-xs` | `0.625rem` (10px) | Micro labels, badge text, metadata |
| `--text-sm` | `0.75rem` (12px) | Secondary/muted text |
| `--text-base` | `0.875rem` (14px) | **Default body — one step down from 16px** |
| `--text-lg` | `1rem` (16px) | Card titles, drawer headings |
| `--text-xl` | `1.125rem` (18px) | Nav wordmark |
| `--text-2xl` | `1.5rem` (24px) | View headings (Hive, Processes) |
| `--weight-regular` | `400` | Body |
| `--weight-medium` | `500` | Buttons, labels |
| `--weight-semibold` | `600` | Card titles, column headers |
| `--weight-bold` | `700` | Wordmark |
| `--tracking-tight` | `-0.02em` | Headings |
| `--tracking-wider` | `0.12em` | Eyebrow / micro-caps (`.vks-eyebrow`) |

### Spacing (4px grid)

| Token | Value |
|---|---|
| `--space-1` | `0.25rem` (4px) |
| `--space-2` | `0.5rem` (8px) |
| `--space-3` | `0.75rem` (12px) |
| `--space-4` | `1rem` (16px) |
| `--space-5` | `1.25rem` (20px) |
| `--space-6` | `1.5rem` (24px) |
| `--space-8` | `2rem` (32px) |

### Controls

| Token | Value |
|---|---|
| `--control-xs` | `2rem` (32px) |
| `--control-sm` | `2.25rem` (36px) |
| `--control-md` | `2.5rem` (40px) |
| `--control-lg` | `2.75rem` (44px) |

### Radius

| Token | Value |
|---|---|
| `--radius-sm` | `0.25rem` (4px) |
| `--radius-md` | `0.375rem` (6px) |
| `--radius-lg` | `0.5rem` (8px) — default card |
| `--radius-xl` | `0.75rem` (12px) |
| `--radius-full` | `9999px` — pills, badges |

### Shadows & glow

| Token | Value |
|---|---|
| `--shadow-sm` | `0 1px 2px 0 rgb(0 0 0 / 0.4)` |
| `--shadow-md` | `0 2px 8px -1px rgb(0 0 0 / 0.5)` — card hover |
| `--shadow-lg` | `0 8px 24px -4px rgb(0 0 0 / 0.6)` — drawer |
| `--glow-cyan` | `0 0 0 1px hsl(193 100% 50% / 0.4), 0 0 16px -2px hsl(193 100% 50% / 0.35)` — primary btn hover |
| `--glow-emerald` | `0 0 12px -2px hsl(152 100% 50% / 0.5)` — done status dot |
| `--strip-width` | `4px` — task card left status strip |

---

## Components

Source files: `design-source/project/components/`

### Button (`components/core/Button.jsx`, CSS class `.vks-btn`)

**Anatomy:** inline-flex, gap-2, font-ui, weight-medium, radius-md, 1px border, 0.15s transitions.

| Variant | Class | Appearance |
|---|---|---|
| `primary` | `.vks-btn--primary` | Cyan fill, void text; hover: `--glow-cyan` box-shadow |
| `secondary` | `.vks-btn--secondary` | Raised fill, border-strong; hover: border-strong bg |
| `outline` | `.vks-btn--outline` | Transparent, border-strong; hover: raised bg + cyan border |
| `ghost` | `.vks-btn--ghost` | Transparent, muted text; hover: raised bg, foreground text |
| `destructive` | `.vks-btn--destructive` | Transparent, danger text+border; hover: coral/12% bg |
| `link` | `.vks-btn--link` | Transparent, primary text, no border; hover: underline |

| Size | Class | Height | Padding |
|---|---|---|---|
| `xs` | `.vks-btn--xs` | 32px | 0 8px; text-sm |
| `sm` | `.vks-btn--sm` | 36px | 0 12px |
| `md` | `.vks-btn--md` | 40px | 0 16px (default) |
| `lg` | `.vks-btn--lg` | 44px | 0 32px |
| `icon` | `.vks-btn--icon` | 40×40px | 0 |

States: focus → 1px cyan ring; disabled → opacity 0.5, pointer-events none.

---

### Badge (`components/core/Badge.jsx`, CSS class `.vks-badge`)

**Anatomy:** inline-flex, radius-full, 1px border, padding 2px 10px, text-xs, weight-semibold.  
Optional `dot` prop renders a 7px circle in `currentColor`.

| Variant | Class | Appearance |
|---|---|---|
| `default` | `.vks-badge--default` | Cyan fill, void text |
| `secondary` | `.vks-badge--secondary` | Raised fill, body text |
| `destructive` | `.vks-badge--destructive` | Danger fill, void text |
| `outline` | `.vks-badge--outline` | Transparent, border-strong, body text |

Used for: labels on task cards, node agent counts, task status in drawer header.

---

### StatusBadge (`components/board/StatusBadge.jsx`, CSS class `.vks-status`)

**Anatomy:** inline-flex, gap-6px, font-ui text-sm weight-medium. Dot is 9×9px circle.

| Status | Dot color | Glow |
|---|---|---|
| `todo` | `--status-todo` (#a1a1aa) | none |
| `inprogress` | `--status-inprogress` (#3b82f6) | `0 0 8px -1px #3b82f6` |
| `inreview` | `--status-inreview` (#ffb800) | none |
| `done` | `--status-done` (#00ff88) | `0 0 8px -1px #00ff88` |
| `cancelled` | `--status-cancelled` (#ff6b6b) | none |

---

### Card (`components/core/Card.jsx`, CSS class `.vks-card`)

Flat surface-card fill, 1px border, radius-lg. **No shadow at rest; shadow-md on hover.**

Sub-elements:
- `.vks-card__header` — padding space-5 space-5 0
- `.vks-card__title` — text-lg weight-semibold tracking-tight
- `.vks-card__desc` — text-sm text-muted
- `.vks-card__content` — padding space-5
- `.vks-card__footer` — flex row, gap space-2

---

### Input (`components/core/Input.jsx`, CSS class `.vks-input`)

Height: control-md (40px). Background: `--input`. Border: border-strong, radius-md.  
Focus: border-color cyan + `0 0 0 1px hsl(193 100% 50% / 0.35)` ring.  
Placeholder: text-dim. Disabled: opacity 0.5.  
Modifier `.vks-input--mono` switches to font-code text-sm (for code/branch inputs).

---

### Switch (`components/core/Switch.jsx`, CSS class `.vks-switch`)

40×22px, radius-full. Off: raised bg, border-strong. On: cyan fill+border.  
Thumb: 16×16px, void bg, `left: 2px` → translateX(18px) on checked.  
Transition: `cubic-bezier(.4,0,.2,1)` at 0.18s.

---

### Checkbox (`components/core/Checkbox.jsx`, CSS class `.vks-checkbox`)

18×18px, radius-sm. Off: input bg, border-strong, SVG check hidden.  
On: cyan fill+border, SVG check visible. Disabled: opacity 0.5.

---

### Tabs (`components/core/Tabs.jsx`, CSS class `.vks-tabs`)

List: inline-flex, 2px gap, 3px padding, surface-card bg, 1px border, radius-md.  
Trigger: muted text by default; hover → foreground; active (`data-active="true"`) → raised bg + foreground.  
Padding: 6px 14px; radius-sm.

---

### Select (`components/core/Select.jsx`, CSS class `.vks-select`)

Native `<select>` with custom chevron overlay. Appearance: none; 40px height; padding 0 32px 0 12px.  
Focus: cyan border + ring. Chevron: absolute right space-3, pointer-events none, text-muted.

---

### Loader (`components/core/Loader.jsx`, CSS class `.vks-loader`)

Inline-block, border-radius 50%. Border: 2px solid border-strong; top: cyan.  
Animation: `vks-spin` 0.7s linear infinite (full rotation).

---

### TaskCard (`components/board/TaskCard.jsx`, CSS class `.vks-task`)

**The primary board element.** Compact card with left status strip.

**Anatomy:**
- Outer `.vks-task.vks-task--{status}`: surface-card, 1px border, radius-md, padding space-3 with extra `--strip-width` (4px) left indent
- `::before` pseudo: absolute 4px left strip colored by `--status-{status}`
- `.vks-task__title`: text-base weight-medium foreground
- `.vks-task__desc`: text-sm text-muted, single-line ellipsis (below title)
- `.vks-task__meta` row: flex between; left = node tag + labels; right = days badge
  - `.vks-task__node`: font-code text-xs text-muted
  - Labels: `.vks-badge.vks-badge--outline` at text-xs, max 2 shown
  - Days: `.vks-badge.vks-badge--secondary` text-xs, format `{n}d`
- `AttemptIndicator` (top-right of title row):
  - `running` → 14×14px Loader (cyan)
  - `merged` → 16×16px SVG circle + check in `--success`
  - `failed` → 16×16px SVG circle + X in `--danger`

**States:** hover → shadow-md + border-strong; selected → `box-shadow: 0 0 0 2px var(--primary)` + border-primary.

---

### NodeCard (`components/board/NodeCard.jsx`, CSS class `.vks-node`)

Row layout for a swarm node.

**Anatomy:**
- Outer: flex items-center, gap space-3, surface-card, 1px border, radius-lg, padding space-4
- OS glyph: 36×36px raised bg, radius-md, centered; SVG glyphs for mac/linux/windows
- Name: `.vks-node__name` font-code text-base weight-medium foreground
- Pulse dot: 8×8px circle; online → success bg + `vks-pulse` 2s animation; offline → text-dim, no animation
- Meta: text-sm text-muted below name row
- Right slot: arbitrary React node (agent count badge, offline badge)

---

## UI surfaces

Source: `design-source/project/ui_kits/vk-swarm-app/`

### Navbar (`chrome.jsx`)

- Height: 48px. Background: `--background`. Bottom border: 1px solid border.
- **Left:** Wordmark (`VK` cyan, `-SWARM` foreground, Chivo Mono bold 18px; collapses to `VKS` on mobile)
  - 1px vertical divider → project switcher button (ghost sm, folder icon + project name + chevron)
- **Right (desktop):** search input 260px → bolt icon button → `+ Task` primary sm → divider → theme toggle → activity icon → settings icon → menu icon
- **Right (tablet):** search icon, no activity icon
- **Right (mobile):** search icon, `+` only (no "Task" label), menu icon
- **Tab row** (below main bar): Board / Nodes / Processes tabs; active tab has 2px cyan bottom border, margin-bottom -1px to bleed into main border

### Kanban board (`board.jsx`)

- Grid: `grid-auto-flow: column; grid-auto-columns: minmax(264px, 1fr)`. Horizontally scrollable.
- Left border: 1px border; each column right border: 1px border.
- 5 columns: To Do · In Progress · In Review · Done · Cancelled.
- **Column header** (sticky top): flex items-center, gap 8px, padding 10px 12px; background `--background` + linear-gradient `color-mix(in srgb, {statusColor} 8%, transparent)` top wash; bottom: 1px dashed border.
  - Left: 9px status-color dot · column label text-sm weight-semibold · count badge (surface-card bg, text-xs, padding 1px 7px, radius 4px)
  - Right: ghost + icon button (24px), adds a task
- **Card list**: flex column, gap 8px, padding 10px, overflow-y auto
- **Empty state**: `.vks-ansi-dither.vks-scanlines`, radius-md, 1px border, min-height 80px; centered text `░▒ no tasks ▒░` in font-code text-xs text-muted

### Task drawer (`panels.jsx`)

- `aside`: absolute right-0 top-0 bottom-0, width 460px, max-width 90vw, z-index 11
- Background: surface-card; left border: 1px border-strong; shadow-lg
- **Header** (padding 16px 18px, bottom border):
  - StatusBadge (dot only) + title (text-lg weight-semibold) + close X button (ghost icon 28px)
  - Below: flex-wrap badges row — status text badge (outline+dot), node badge (secondary), label badges (outline)
- **Tab bar**: Diff / Logs / Attempts — `Tabs` component, padding 14px 18px
- **Content**: flex-1 overflow-y-auto, padding 0 18px 18px
- **Footer action bar** (padding 16px, top border): Merge (primary sm, flex-1) · Rebase (outline sm) · Open in IDE (ghost sm)
- **Overlay**: absolute inset-0, `--surface-overlay`, z-index 10

### Nodes view (`panels.jsx`)

- Padding 20px; overflow-y auto
- Heading: `<h2>` font-display text-2xl weight-semibold + StatusBadge showing online count
- Grid: `repeat(auto-fill, minmax(320px, 1fr))`, gap 12px, maxWidth 1000px

### Processes view (`panels.jsx`)

- Padding 20px
- Heading: `<h2>` font-display text-2xl weight-semibold
- Card-style list (`vks-card`), each row: flex items-center, gap 12px, padding 12px 16px, bottom border except last
  - Left: Loader (running) or done-status-dot
  - Name: font-code text-sm
  - Node: font-code text-xs text-muted
  - Duration: font-code text-xs text-dim, width 56px, text-right

---

## Interaction & state matrix

| Component | State | Trigger | Visual / behaviour |
|---|---|---|---|
| Button primary | default | — | Cyan fill, void text |
| Button primary | hover | mouseenter | `--glow-cyan` box-shadow |
| Button primary | focus | keyboard focus | 1px cyan ring (box-shadow) |
| Button primary | disabled | disabled attr | opacity 0.5, pointer-events none |
| Button outline | hover | mouseenter | raised bg, cyan border |
| Button ghost | hover | mouseenter | raised bg, foreground text |
| Button destructive | hover | mouseenter | coral/12% bg wash |
| Input | default | — | border-strong |
| Input | focus | user focus | cyan border + `0 0 0 1px` cyan/35% ring |
| Input | disabled | disabled attr | opacity 0.5, cursor not-allowed |
| Switch | off | — | raised bg, border-strong, thumb left |
| Switch | on | click | cyan bg + border, thumb translateX(18px) |
| Switch | disabled | disabled attr | opacity 0.5 |
| Checkbox | off | — | input bg, border-strong, check hidden |
| Checkbox | on | click | cyan fill + border, check visible |
| Tabs trigger | default | — | text-muted |
| Tabs trigger | hover | mouseenter | foreground text |
| Tabs trigger | active | `data-active=true` | raised bg, foreground |
| TaskCard | default | — | surface-card, 1px border, 4px status strip |
| TaskCard | hover | mouseenter | shadow-md + border-strong |
| TaskCard | selected | onClick | `0 0 0 2px var(--primary)` outer ring + cyan border |
| NodeCard | online | `online=true` | 8px emerald pulse dot, `vks-pulse` 2s animation |
| NodeCard | offline | `online=false` | 8px text-dim dot, no animation |
| Navbar tab | default | — | text-muted, no underline |
| Navbar tab | active | view switch | foreground text, 2px cyan bottom border |
| Card | hover | mouseenter | shadow-md |
| Task drawer | open | card click | slides in from right, overlay scrim |
| Task drawer | close | overlay click / X | dismissed |
| AttemptIndicator running | — | `attempt='running'` | 14px cyan Loader spinner |
| AttemptIndicator merged | — | `attempt='merged'` | success circle + check SVG |
| AttemptIndicator failed | — | `attempt='failed'` | danger circle + X SVG |
| Empty column | — | 0 tasks | ansi-dither + scanlines texture, `░▒ no tasks ▒░` label |

### ANSI/BBS texture utilities

Applied to empty states, drop zones, and hero panels — **not** to functional product chrome:

| Class | Pattern |
|---|---|
| `.vks-ansi-dither` | Radial-gradient dot field (░ ANSI light shade) |
| `.vks-ansi-dither-dense` | Denser variant (▒ medium shade) |
| `.vks-ansi-weave` | Cross-hatch (▚▞), ±45° repeating lines |
| `.vks-ansi-grid` | Box-drawing lattice, 10×16px cell ratio |
| `.vks-diagonal-lines` | Original −45° diagonal stripe |
| `.vks-scanlines` | `::after` CRT overlay (scanlines + vignette) |
| `.vks-wordmark` | Chivo Mono; `.vk` in primary cyan, `.swarm` in foreground |
| `.vks-eyebrow` | font-code text-xs tracking-wider uppercase text-muted |

### Responsive breakpoints

| Name | Width | Key adaptations |
|---|---|---|
| mobile | < 640px | Wordmark compacts to `VKS`; search → icon; project label drops; fewer nav icons |
| tablet | 640–1023px | Search → icon; activity icon drops from right bar |
| desktop | ≥ 1024px | Full chrome; 260px search input visible |

Board columns: always min 264px, horizontal scroll rather than crush.  
Task drawer: `min(460px, 90vw)`.  
Node grid: `repeat(auto-fill, minmax(320px, 1fr))`.
