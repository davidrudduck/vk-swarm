# VK-Swarm Design System

> "Midnight Terminal" — the brand language of **VK-Swarm**, a kanban-based code
> executor orchestration system. A swarm-based fork of vibe-kanban.

---

## 1. Product context

**VK-Swarm** lets engineers orchestrate fleets of AI coding agents (Claude Code,
Codex, OpenCode, …) across a distributed **swarm** of machines. Each machine runs
a local **node**; nodes connect over a persistent WebSocket to a central
**hive** (PostgreSQL + activity broker + node registry). Work is organized on a
**kanban board** of tasks that flow `To Do → In Progress → In Review → Done`
(with a `Cancelled` lane). Agents pick up tasks, run in isolated git worktrees,
stream logs back live, and surface diffs for human review and merge.

Core surfaces:
- **Board** — the kanban of tasks, the primary view.
- **Nodes / Hive** — the registry of connected machines and their agents.
- **Processes** — running agents, dev servers, and test runs across the swarm.
- **Task drawer** — diff review, log stream, and attempt history per task.

### Sources
This system was reverse-engineered from the product's own frontend. Explore it
to design with higher fidelity:
- **GitHub:** [davidrudduck/vk-swarm](https://github.com/davidrudduck/vk-swarm)
  — esp. `frontend/tailwind.config.js`, `frontend/src/styles/index.css` (the
  "Midnight Terminal" theme), and `frontend/src/components/{ui,tasks,swarm,layout}`.
- Upstream project: vibe-kanban (`davidrudduck/vibe-kanban`).

> The repo is the source of truth for tokens and component anatomy. When in
> doubt, read it.

---

## 2. Content fundamentals — voice & copy

- **Audience:** developers. Copy is terse, technical, and assumes fluency with
  git, agents, CI, and the terminal.
- **Tone:** calm, precise, operational. It reads like good CLI output — status
  first, no marketing fluff. "✓ worktree created", "3 nodes online",
  "last seen 4m ago".
- **Voice:** mostly imperative and label-like. Buttons are bare verbs:
  *Create task*, *Merge*, *Rebase*, *Fix All Issues*, *Open in IDE*. Avoid
  "Please" / "Let's".
- **Person:** product-neutral. Addresses the user as "you" only in docs
  ("Connect your local instance…"); the UI itself avoids pronouns.
- **Casing:** Title Case for nav items, view names and primary buttons
  ("In Progress", "Show archived"). sentence case for descriptions and hints.
  Status labels are Title Case ("To Do", "In Review").
- **Numbers & units:** compact and monospaced — `2d` (days in column), `41m`,
  `2m 14s`, counts as bare integers in pill badges.
- **Identifiers:** node names, branches, file paths, and agent names always in
  the **code font** (JetBrains Mono), often truncated to a short form
  (`justX.raverx.net` → `justX`).
- **Emoji:** not used in product chrome. The upstream README uses 🦞 playfully
  in sibling projects, but VK-Swarm UI itself relies on **icons**, not emoji.
  Terminal-style glyphs (`✓ ✗ → $`) appear inside log/console output only.
- **Vibe:** "mission control for a swarm of agents." Quiet until something needs
  attention, then a single accent color does the signalling.

---

## 3. Visual foundations

### Palette — "Midnight Terminal"
A near-black, blue-shifted dark theme. Depth comes from **lightening surfaces**
(void → surface → raised), not from shadows. One accent (cyan) carries almost
all emphasis; other hues are reserved for signals.

| Role | Token | Hex |
|---|---|---|
| Background (void) | `--background` | `#0a0a0f` |
| Card / panel | `--surface-card` | `#12121a` |
| Elevated surface | `--surface-raised` | `#1a1a24` |
| **Primary accent** | `--primary` | `#00d4ff` cyan |
| Secondary accent | `--accent` | `#a855f7` violet |
| Success | `--success` | `#00ff88` emerald |
| Warning | `--warning` | `#ffb800` amber |
| Danger | `--danger` | `#ff6b6b` coral |
| Text | `--foreground` | `#e4e4e7` |
| Muted text | `--text-muted` | `#71717a` |
| Dim text | `--text-dim` | `#3f3f46` |

**Task-status colors** (board columns + the 4px left strip on cards):
`todo` neutral · `inprogress` blue `#3b82f6` · `inreview` amber · `done` emerald
· `cancelled` coral. Running/done dots get a soft outer **glow**.

### Type
- **UI / prose:** Inter. The product **downshifts the scale by one step** — body
  text is **14px**, not 16. Sizes: xs 10 · sm 12 · base 14 · lg 16 · xl 18.
- **Code / mono:** JetBrains Mono — logs, diffs, node names, branches,
  durations, metadata. The **log viewer is skinnable**: *Light* and *Dark* are
  clean, normal terminals; *System* follows the OS color scheme; *ANSI / BBS* is
  an explicit opt-in skin that adds the reverse-video title bar, CRT scanlines,
  cyan glow and blinking cursor. BBS is a power-user choice, never forced — the
  default normal skins stay highly legible.
- **Display:** Source Serif 4 — section headings ("Hive", "Processes") and any
  editorial/marketing surface. A serif against mono is the signature contrast.
- **Wordmark:** Chivo Mono, bold. `VK` in cyan, `-SWARM` in foreground; collapses
  to `VKS` at narrow widths.

### Spacing, radius, borders
- 4px base grid; the UI is **dense** (controls h-8/9/10, cards padded 12px).
- Radius base `0.5rem` with `-2/-4px` steps; pills use full radius.
- **Hairline 1px borders** everywhere; **dashed** borders separate kanban
  columns and section headers — a defining texture.
- Status strip: a **4px** colored bar down the left edge of every task card.

### Backgrounds & texture
- Flat void background — **no gradients** as decoration. The terminal/BBS
  texture utilities supply the brand's surface pattern: `.vks-ansi-dither`
  (ANSI ░ shade-block dot field), `.vks-ansi-weave` (▚▞ cross-hatch),
  `.vks-ansi-grid` (character-cell lattice) and the original
  `.vks-diagonal-lines` — all theme-aware, used for **empty states**, drop
  zones and hero panels. Pair any of them with `.vks-scanlines` for the CRT
  overlay seen on the wordmark.

### ANSI / BBS motif — where it applies (and where it doesn't)
The 80s/90s ANSI-art treatment (figlet block letters, reverse-video status
strips, `▚▞ ░▒▓` glyphs, CRT scanlines + glow) is a deliberate, **scoped accent
layer** — it signals "terminal / mission-control" without compromising the clean
product UI. Apply it to:
- the **wordmark / logo** lockup,
- **empty states, drop zones and hero panels** (the texture utilities),
- **console / log-stream surfaces** — the code-executor log viewer reads as a
  BBS terminal (reverse-video title bar, scanlines, blinking cursor).

Keep it **off** functional product chrome and foundation specimens — buttons,
inputs, cards, kanban columns, color/spacing/type swatches stay clean and highly
legible. Earlier direction was explicit: *clear, easy-to-read fonts throughout
the interface.* The ANSI flavor is seasoning, not the meal.
- Subtle column tint: each column header carries a ~3% wash of its status color.

### Elevation, depth & glow
- Shadows are restrained (`--shadow-sm/md/lg`, pure black at low alpha). Used on
  hover of cards and on the task drawer.
- **Glow is the brand's "shine":** primary buttons and live dots emit a soft
  cyan/emerald glow (`--glow-cyan`, `--glow-emerald`) rather than a bevel.
- Overlays use `--surface-overlay` (void at 80%) — flat scrim, no blur by
  default. Reserve blur for transient popovers if needed.

### Motion
- Quick and functional: `.15s ease` on color/border/shadow transitions; the
  switch thumb uses `cubic-bezier(.4,0,.2,1)` at `.18s`.
- Two living animations: the **spinner** (`vks-spin`, 0.7s linear) for running
  attempts, and the **node pulse** (`vks-pulse`, 2s) ring on online nodes.
- No bounces, no parallax, no decorative looping on content.

### Interaction states
- **Hover:** surfaces lighten (`surface → raised`), ghost buttons gain a raised
  background, outline buttons adopt a cyan border, cards raise a shadow.
- **Focus:** 1px cyan ring (`box-shadow` inset/offset), never a browser outline.
- **Active/selected:** a 2px cyan ring around the element (selected task card,
  active attempt).
- **Disabled:** opacity 0.5, `pointer-events: none`.

### Cards
Flat `--surface-card` fill, 1px `--border`, `0.5rem` radius, **no drop shadow at
rest** (shadow appears on hover). Task cards additionally carry the left status
strip and are deliberately compact.

### Theming — dark & light
The brand is **dark-first**; "Midnight Terminal" *is* the identity. Light mode is
a fully-supported alternate, opt-in per region:

```html
<html data-theme="dark">   <!-- default; can be omitted -->
<html data-theme="light">  <!-- or .theme-light on any wrapper -->
```

Every product token (`--background`, `--surface-card`, `--primary`, status
colors…) re-points under `[data-theme='light']` — build against the **semantic
aliases**, never the raw `--vks-*` brand values, and components flip themes for
free. In light mode the cyan primary deepens to a teal `#0091b5` so it holds
**AA contrast on white**; status hues deepen similarly. To follow the OS setting,
set the attribute from JS on load (one line — see the comment in
`tokens/colors.css`); the UI kit persists the user's choice to `localStorage`.

**The console / terminal stays dark in both themes** — `--console-*` tokens are
intentionally *not* overridden in light mode. Code-executor log viewers, diffs
and agent streams read best on near-black, so keep them on `--console-bg` with
the JetBrains Mono `--font-code` regardless of the surrounding theme.

### Responsive
The product is a desktop-dense tool that must also hold up on tablet and phone.
Breakpoints used in the UI kit (`useBreakpoint`): **mobile < 640px · tablet
640–1023px · desktop ≥ 1024px** (designs target 1080p and up at the high end).
Patterns:
- **Board:** columns are a horizontally-scrolling track
  (`grid-auto-columns: minmax(264px, 1fr)`) — never crush below ~264px; scroll
  instead. The drawer caps at `min(460px, 90vw)`.
- **Navbar:** below desktop the search field collapses to an icon, the project
  label and secondary actions drop, and the wordmark compacts to `VKS`.
- **Grids:** node/process lists use `repeat(auto-fill, minmax(320px, 1fr))` so
  they reflow from multi-column to single naturally.
- Hit targets stay **≥ 34px** (44px on touch-primary surfaces).

---

## 4. Iconography

- **Library:** the product uses **[lucide-react](https://lucide.dev)** (1.5–2px
  stroke, rounded line icons) for UI glyphs, and **simple-icons** for brand
  marks (IDEs, agents). This design system mirrors that with inline lucide-style
  stroke icons in the UI kit (`chrome.jsx` → `ICONS`).
- **To match the product:** pull icons from lucide — `FolderOpen`, `Server`,
  `Activity`, `Settings`, `Plus`, `Search`, `Menu`, `CheckCircle`, `XCircle`,
  `Loader2`, `Link`, `Archive`, `AlertTriangle`, `Wrench`. Keep stroke ~1.6px on
  a 24px grid. Lucide is CDN-available — link it rather than hand-drawing.
- **Brand / IDE marks:** real SVGs are copied into `assets/icons/ide/`
  (VS Code, Cursor, Zed, Windsurf, IntelliJ) — use these, don't redraw.
- **Logo:** the wordmark is **typographic** (Chivo Mono), not a glyph. A
  barcode-style favicon lockup lives in `assets/logos/` (`vk-favicon-dark.svg`,
  `vk-favicon-light.svg`).
- **Emoji:** not used as UI iconography. Terminal glyphs (`✓ ✗ → $`) are used
  only inside rendered console/log output.

---

## 5. Index / manifest

**Root**
- `styles.css` — the single entry point consumers link. `@import`s only.
- `tokens/` — `fonts.css`, `colors.css`, `typography.css`, `spacing.css`,
  `base.css` (element defaults + helpers), `components.css` (component classes).
- `README.md` — this guide. · `SKILL.md` — Claude Code skill manifest.

**Components** (`window.VKSwarmDesignSystem_067861.*`)
- `components/core/` — `Button`, `Badge`, `Card` (+ Header/Title/Description/
  Content/Footer), `Input`, `Switch`, `Checkbox`, `Tabs`, `Select`, `Loader`.
- `components/board/` — `StatusBadge`, `TaskCard`, `NodeCard`.

**UI kits**
- `ui_kits/vk-swarm-app/` — interactive kanban board + nodes + processes +
  task drawer. See its `README.md`.

**Guidelines / specimen cards** (`guidelines/*.html`)
- Colors: surfaces, accents, semantic, task-status.
- Type: display, UI, code. · Spacing: scale, radius/borders.
- Brand: wordmark, diagonal-line texture, dark & light themes.

**Assets**
- `assets/logos/` — favicon lockups. · `assets/icons/ide/` — IDE brand marks.

---

## 6. Caveats / substitutions
- **Fonts** are loaded from **Google Fonts** (Inter, JetBrains Mono, Source
  Serif 4, Chivo Mono), matching the app's own `@import`. No self-hosted binaries
  ship with this system, so the compiler reports zero `@font-face` rules — this
  is expected, not a defect.
- **Icons** are lucide-style (the product's real library). The UI kit inlines a
  small subset; link lucide from CDN for full coverage.
