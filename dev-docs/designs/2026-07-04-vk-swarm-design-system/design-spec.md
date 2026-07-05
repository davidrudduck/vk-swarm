# Design spec — vk-swarm-design-system

**Source:** `dev-docs/designs/2026-07-04-vk-swarm-design-system/design-source/` (verbatim Claude Design handoff bundle, preserved per PRESERVE-FIRST).
**Brand language:** "Midnight Terminal" — dark-first, terminal/ANSI/BBS texture vocabulary, cyan primary on deep violet void.

## Tokens

### Color (dark-first, light opt-in)

| Token | Dark | Light | Use |
|-------|------|-------|-----|
| `--vks-void` | `#1a1a33` | `#f5f6f9` | app background |
| `--vks-surface` | `#282839` | `#ffffff` | cards, panels, raised surfaces |
| `--vks-surface-bright` | `#303040` | `#eef0f4` | hover/raised cards, inputs |
| `--vks-console` | `#0a0a0f` | `#0a0a0f` | code/terminal bg (true black both themes) |
| `--vks-text` | `#e4e4e7` | `#1a1a33` | primary text |
| `--vks-muted` | `#71717a` | `#71717a` | secondary text, labels |
| `--vks-dim` | `#3f3f46` | `#d4d4d8` | borders, dividers, faint text |
| `--vks-primary` | `#00d4ff` (cyan) | `#0091b5` (teal) | actions, focus rings, live dots |
| `--vks-accent` | `#a855f7` (violet) | `#a855f7` | accents, secondary highlights |
| `--vks-success` | `#00ff88` (emerald) | `#00ff88` | done status, success dots |
| `--vks-warning` | `#ffb800` (amber) | `#ffb800` | in-review status, warnings |
| `--vks-danger` | `#ff6b6b` (coral) | `#ff6b6b` | cancelled status, destructive actions |
| `--vks-info` | `#3b82f6` (blue) | `#3b82f6` | in-progress status |
| `--vks-neutral` | `#a1a1aa` | `#a1a1aa` | todo status |

Source: `design-source/tokens/colors.css:1-156`. Console `--vks-console` stays true-black in both themes (`colors.css:18-19`).

### Typography

Downshifted scale, base=14px. Source: `design-source/tokens/typography.css:1-49`.

| Token | Size | Use |
|-------|------|-----|
| `--vks-text-xs` | 10px | badges, labels, eyebrow |
| `--vks-text-sm` | 12px | secondary text, table cells |
| `--vks-text-base` | 14px | body, inputs, buttons |
| `--vks-text-lg` | 16px | emphasized body |
| `--vks-text-xl` | 18px | section headers |
| `--vks-text-2xl` | 24px | page titles |
| `--vks-text-3xl` | 30px | hero headings |
| `--vks-text-4xl` | 40px | display |
| `--vks-text-5xl` | 56px | splash display |

**Fonts:** Inter (UI/prose), JetBrains Mono (code/terminal), Source Serif 4 (display headings), Chivo Mono (wordmark). All Google Fonts. Source: `design-source/tokens/fonts.css`.

### Spacing (4px grid)

Source: `design-source/tokens/spacing.css:1-46`.

| Token | Value | Notes |
|-------|-------|-------|
| `--vks-space-1` | 4px | icon-text gap |
| `--vks-space-2` | 8px | tight element gap |
| `--vks-space-3` | 12px | default card padding |
| `--vks-space-4` | 16px | section padding |
| `--vks-space-5` | 20px | — |
| `--vks-space-6` | 24px | page gutter |
| `--vks-space-8` | 32px | section gap |
| `--vks-space-10` | 40px | — |
| `--vks-space-12` | 48px | — |
| `--vks-space-16` | 64px | hero spacing |

**Control heights:** xs 32px, sm 36px, md 40px (default), lg 44px. Source: `spacing.css:32-36`.
**Radius:** sm 0.25rem, md 0.375rem, lg 0.5rem (base), xl 0.75rem, full 9999px. Source: `spacing.css:38-42`.
**Borders:** hairline 1px; dashed between board columns/sections; status strip 4px (left edge of task cards). Source: `spacing.css:44-46`, `base.css`, `components.css`.
**Shadows:** sm/md/lg (pure black low alpha). Source: `spacing.css:24-28`.
**Glows:** `--glow-cyan` (primary buttons, live dots), `--glow-emerald` (success dots). Source: `spacing.css:24-28`.

### Motion

- Color/border/shadow transitions: `.15s ease`. Source: `base.css`.
- Switch thumb: `cubic-bezier(.4,0,.2,1) .18s`. Source: `components.css` `.vks-switch-thumb`.
- `vks-spin`: `.7s linear` infinite (spinner). Source: `components.css` `.vks-loader`.
- `vks-pulse`: `2s` (node online ring). Source: `components.css` `.vks-node-online`.

### Texture utilities (ANSI/BBS vocabulary)

Source: `design-source/tokens/base.css:40-134`. Class-based:

- `.vks-ansi-dither` — subtle dot-field background
- `.vks-ansi-weave` — woven pattern
- `.vks-ansi-grid` — grid overlay
- `.vks-scanlines` — CRT scanline overlay
- `.vks-diagonal-lines` — diagonal hatch
- `.vks-wordmark` — Chivo Mono wordmark style
- `.vks-eyebrow` — uppercase tracked label (xs, letter-spacing)
- `.vks-dashed` — dashed divider

Used on empty states (`░▒ no tasks ▒░`), board column dividers, console panels.

## Components

### Core primitives

Source: `design-source/components/core/`.

#### Button
Anatomy: `<button class="vks-btn vks-btn--{variant} vks-btn--{size}">`. Variants: `primary` (cyan bg, glow), `secondary` (surface bg), `outline` (cyan border on hover), `ghost` (transparent, gains bg on hover), `destructive` (coral), `link` (text-only, cyan). Sizes: `xs` (32px), `sm` (36px), `md` (40px), `lg` (44px), `icon` (square). Source: `Button.jsx:1-32`, `components.css:.vks-btn*`.

#### Badge
`<span class="vks-badge vks-badge--{variant}">`. Variants: `default`, `secondary`, `destructive`, `outline`. Optional `dot` prop adds a leading colored dot. Source: `Badge.jsx:1-19`.

#### Card
Compound: `Card > {CardHeader > {CardTitle, CardDescription}} > CardContent > {CardFooter}`. Surface bg, hairline border, lg radius, md shadow (raised on hover). Source: `Card.jsx:1-29`.

#### Input
`<input class="vks-input">`. Optional `mono` prop switches to JetBrains Mono. Surface-bright bg, hairline border, focus → cyan ring. Source: `Input.jsx:1-7`.

#### Switch
Controlled/uncontrolled toggle. `<button class="vks-switch" role="switch" aria-checked>`. Thumb slides with cubic-bezier motion. Source: `Switch.jsx:1-27`.

#### Checkbox
Controlled/uncontrolled. `<button class="vks-checkbox" role="checkbox" aria-checked>`. Cyan check on checked. Source: `Checkbox.jsx:1-29`.

#### Tabs
Segmented control. `<div class="vks-tabs">` with `Tab` children. Active tab gets surface-bright bg + cyan text. Source: `Tabs.jsx:1-31`.

#### Select
Native `<select>` styled with `.vks-select`. Surface-bright bg, chevron icon. Source: `Select.jsx:1-24`.

#### Loader
Spinner. `<span class="vks-loader vks-loader--{size}">`. Sizes: sm/md/lg. `.7s linear` spin. Source: `Loader.jsx:1-17`.

### Board components

Source: `design-source/components/board/`.

#### TaskCard
Props: `title`, `description`, `status`, `node`, `labels`, `attempt`, `days`. Anatomy: surface card with 4px left status strip (`::before` colored by status), title, description (muted, clamped), footer row (node name + labels + `AttemptIndicator`). `AttemptIndicator` sub-component shows running/merged/failed states. Source: `TaskCard.jsx:1-53`.

#### NodeCard
Props: `name`, `os`, `online`, `meta`, `right`. Anatomy: surface card, OS glyph (mac/linux/windows map), online pulse ring (emerald, 2s), name, meta row, optional right slot. Source: `NodeCard.jsx:1-27`.

#### StatusBadge
Props: `status` (todo/inprogress/inreview/done/cancelled), `showLabel`, `label`. Dot + optional label, colored by status token. Source: `StatusBadge.jsx:1-14`.

### Settings components

Source: `design-source/components/settings/`.

#### SettingsSection
Card variant with header (title/description/icon/footer) + stacked body. Source: `SettingsSection.jsx:1-34`.

#### SettingsRow
Props: `label`, `htmlFor`, `helper`, `error`, `inline`, `nested`, `control`. Variants: stacked (default), inline (label + control horizontal), nested (indented). Error state shows coral helper. Source: `SettingsRow.jsx:1-54`.

### App UI kit

Source: `design-source/ui_kits/vk-swarm-app/`.

#### BoardView (`board.jsx`)
5-column kanban: `todo` / `inprogress` / `inreview` / `done` / `cancelled`. Each column has `ColumnHeader` (status dot + count + add button). Horizontally-scrolling grid `minmax(264px, 1fr)`. Empty state: `.vks-ansi-dither .vks-scanlines` `░▒ no tasks ▒░`. Source: `board.jsx:1-85`.

#### Chrome (`chrome.jsx`)
Navbar: Logo + ThemeToggle + NavIcon (lucide-style) + NavTab. `useBreakpoint` hook: mobile <640, tablet 640-1023, desktop ≥1024. `ICONS` map. Source: `chrome.jsx:1-132`.

#### Panels (`panels.jsx`)
- **NodesView** — NodeCard grid `repeat(auto-fill, minmax(320px, 1fr))`.
- **ProcessesView** — rows: loader + status + name + node + duration.
- **TaskDrawer** — slide-in drawer capped `min(460px, 90vw)`. StatusBadge + title + labels + Tabs(diff/logs/attempts). **DiffPanel** (console bg, add/del/ctx/meta line classes). **LogsPanel** (console bg, muted/ok/fg/cy/err line classes). **AttemptsPanel**. Footer: Merge/Rebase/Open-in-IDE buttons. Source: `panels.jsx:1-151`.

## Interaction & state matrix

| Component | State | Trigger | Visual/behaviour | Source ref |
|-----------|-------|---------|------------------|------------|
| Button (all variants) | default | — | per-variant bg/border | `components.css:.vks-btn` |
| Button | hover | mouseenter | primary: brighter cyan + glow; secondary: surface→bright; outline: cyan border; ghost: gains surface bg; destructive: brighter coral | `components.css:.vks-btn:hover` |
| Button | focus | focus | 1px cyan ring box-shadow, no browser outline | `components.css:.vks-btn:focus-visible` |
| Button | active | mousedown | 2px cyan ring around element | `components.css:.vks-btn:active` |
| Button | disabled | `disabled` prop | opacity 0.5, pointer-events none | `components.css:.vks-btn:disabled` |
| Badge | default | — | per-variant bg/text | `components.css:.vks-badge` |
| Card | default | — | surface bg, hairline border, lg radius, md shadow | `components.css:.vks-card` |
| Card | hover | mouseenter | shadow raises (md→lg), surface→bright | `components.css:.vks-card:hover` |
| Input | default | — | surface-bright bg, hairline border | `components.css:.vks-input` |
| Input | focus | focus | cyan ring box-shadow | `components.css:.vks-input:focus` |
| Input | error | `error` prop | coral border + coral helper text | `components.css:.vks-input[aria-invalid]` |
| Switch | default | — | surface track, dim thumb | `components.css:.vks-switch` |
| Switch | checked | toggle | track→cyan, thumb slides right (cubic-bezier .18s) | `components.css:.vks-switch[aria-checked=true]` |
| Switch | disabled | `disabled` | opacity 0.5 | `components.css:.vks-switch:disabled` |
| Checkbox | default | — | surface-bright bg, hairline border | `components.css:.vks-checkbox` |
| Checkbox | checked | toggle | cyan bg + white check | `components.css:.vks-checkbox[aria-checked=true]` |
| Tabs | default | — | dim text | `components.css:.vks-tabs-tab` |
| Tabs | active | click/select | surface-bright bg + cyan text | `components.css:.vks-tabs-tab[aria-selected=true]` |
| TaskCard | default | — | surface card, 4px status strip left | `components.css:.vks-task` |
| TaskCard | hover | mouseenter | shadow raises | `components.css:.vks-task:hover` |
| NodeCard | online | `online` prop | emerald pulse ring (2s) | `components.css:.vks-node-online` |
| NodeCard | offline | `!online` | dim ring, no pulse | `components.css:.vks-node-offline` |
| StatusBadge | per-status | `status` prop | dot colored: todo=neutral, inprogress=blue, inreview=amber, done=emerald, cancelled=coral | `components.css:.vks-status-{x}` |
| BoardView column | empty | no tasks | `.vks-ansi-dither .vks-scanlines` + `░▒ no tasks ▒░` | `board.jsx` empty state |
| TaskDrawer | open | task click | slide-in from right, width `min(460px, 90vw)` | `panels.jsx` TaskDrawer |
| Loader | spinning | — | `.7s linear` spin | `components.css:.vks-loader` |
| SettingsRow | error | `error` prop | coral helper text | `components.css:.vks-settings-row-error` |

## Responsive

- **Mobile** <640px: bottom nav, stacked panels, board horizontal scroll.
- **Tablet** 640-1023px: compact nav.
- **Desktop** ≥1024px: full nav, NodeCard grid `repeat(auto-fill, minmax(320px, 1fr))`, board `minmax(264px, 1fr)`.
- Hit targets ≥34px (44px touch). Drawer caps `min(460px, 90vw)`.

Source: `design-source/README.md` responsive section, `chrome.jsx` `useBreakpoint`, `board.jsx`, `panels.jsx`.