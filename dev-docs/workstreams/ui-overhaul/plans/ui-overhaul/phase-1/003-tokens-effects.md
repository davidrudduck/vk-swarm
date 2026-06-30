---
id: "003"
phase: 1
title: Add shadow/glow tokens, the vks-pulse keyframe, and ANSI texture utility classes
status: ready
depends_on: ["002"]
parallel: false
conflicts_with: ["001", "002"]
files:
  - frontend/src/styles/index.css
irreversible: false
scope_test: "N/A"
allowed_change: edit
covers_criteria: [SC15]
---
## Failing test (write first)
N/A — covered by manual verification (greppable token/keyframe/class assertions below).

## Change

Add the effect tokens (shadows/glows), the `vks-pulse` keyframe, and the ANSI texture utility
classes to the live stylesheet. The shadow/glow tokens join the brand cascade inside `.dark {}`
(alongside the 002 tokens); the keyframe and ANSI classes live at file top level so they are
globally available regardless of theme.

### File: `frontend/src/styles/index.css`

**Anchor 1 — `.dark {` block (after the status tokens added in 002).** Add the shadow/glow tokens.
These values are translated from design-source `project/tokens/spacing.css`; note that the source's
`--vks-cyan-hsl` / `--vks-emerald-hsl` are renamed to the product's `--vks-cyan` / `--vks-emerald`.
Insert these EXACT lines inside the `.dark {}` block:
```css
    /* Shadows + glows (dark) */
    --shadow-sm: 0 1px 2px 0 rgb(0 0 0 / 0.4);
    --shadow-md: 0 2px 8px -1px rgb(0 0 0 / 0.5);
    --shadow-lg: 0 8px 24px -4px rgb(0 0 0 / 0.6);
    --glow-cyan: 0 0 0 1px hsl(var(--vks-cyan) / 0.4), 0 0 16px -2px hsl(var(--vks-cyan) / 0.35);
    --glow-emerald: 0 0 12px -2px hsl(var(--vks-emerald) / 0.5);
```

**Anchor 2 — file top level (the `vks-pulse` keyframe).** Add the keyframe at FILE TOP LEVEL, i.e.
outside any `@layer` / selector / rule — NOT inside `@layer base`. A good location is after the
`@import url('https://fonts.googleapis.com/...')` line (line 2) / near other top-level rules. Add
this EXACT block:
```css
@keyframes vks-pulse {
  0%, 100% { opacity: 1; box-shadow: 0 0 0 0 hsl(152 100% 50% / 0.6); }
  50%      { opacity: 0.7; box-shadow: 0 0 0 4px hsl(152 100% 50% / 0); }
}
```
Duration/easing/iteration are NOT declared here — they are applied at the call site via
`animate-[vks-pulse_2s_ease-in-out_infinite]` (consumed by later tasks). Do not add an
`animation:` shorthand.

**Anchor 3 — file top level (the ANSI texture utility classes).** Add the four ANSI texture utility
classes at file top level (alongside the keyframe, outside any `@layer`/selector). The design-source
`dev-docs/designs/2026-06-28-ui-overhaul/design-source/project/tokens/base.css` (≈ lines 65–105) is
the structural reference, BUT it must be **token-translated, not copied verbatim**: in the design
source, `--background`/`--border` are full `hsl()` colours, whereas in THIS product they are bare HSL
triplets (`240 20% 5%`), so a raw `background-color: var(--background)` is **invalid CSS and renders
nothing**. Wrap the colour tokens in `hsl(...)` (the same wrapper applied to the glow tokens in
Anchor 1). Do NOT add the design-source file to `files:` — it is read-only; only `index.css` is
editable. The required result is:
```css
.vks-ansi-dither { background-color: hsl(var(--background)); background-image: radial-gradient(hsl(var(--border)) 1.1px, transparent 1.4px); background-size: 6px 6px; }
.vks-ansi-dither-dense { background-color: hsl(var(--background)); background-image: radial-gradient(hsl(var(--border)) 1.2px, transparent 1.5px); background-size: 4px 4px; }
.vks-scanlines { position: relative; }
.vks-scanlines::after { content: ""; position: absolute; inset: 0; pointer-events: none; background: repeating-linear-gradient(0deg, rgb(0 0 0 / 0.16) 0 1px, transparent 1px 4px), radial-gradient(120% 120% at 50% 0%, transparent 62%, rgb(0 0 0 / 0.4) 100%); }
```
(The `.vks-scanlines::after` body uses literal `rgb(...)` values and is copied as-is. Only the
dither `--background`/`--border` references need the `hsl()` wrapper.)

## Allowed moves
- ONLY: add the shadow/glow token block inside the existing `.dark {}` block; add the `vks-pulse`
  keyframe at file top level; add the four ANSI texture utility classes at file top level. Do not
  touch `:root`, `.light`, `.vks-theme`, the public-token layer, or any component file. Do not edit
  the read-only design-source file.

## Steps
1. Confirm the 002 status tokens are present in `.dark {}` (`grep -- '--status-todo' frontend/src/styles/index.css`).
2. Add the shadow/glow token block inside `.dark {}` (Anchor 1).
3. Add the `vks-pulse` keyframe at file top level (Anchor 2).
4. Read design-source base.css for structure, then add the four ANSI classes at file top level
   with the `--background`/`--border` references wrapped in `hsl(...)` (Anchor 3 — NOT verbatim;
   the product tokens are bare triplets). The `.vks-scanlines::after` rgb() body is copied as-is.

## STOP triggers
- The 002 tokens are absent (`grep -- '--status-todo' frontend/src/styles/index.css` → no match; halt — 002 not applied).
- A `vks-pulse` keyframe already exists (`grep 'vks-pulse' frontend/src/styles/index.css` → match before edit; halt — reconcile rather than duplicate).

## Manual verification (record in decisions-ledger)
- `grep 'vks-pulse' frontend/src/styles/index.css` → match (SC15).
- `grep 'vks-ansi-dither' frontend/src/styles/index.css` → match.
- `grep 'glow-cyan' frontend/src/styles/index.css` → match.
- Dither uses wrapped colour tokens (NOT bare triplets):
  `grep 'vks-ansi-dither {' frontend/src/styles/index.css | grep -q 'hsl(var(--background))'` → match
  (a bare `var(--background)` here renders nothing — invalid CSS colour).
- `cd frontend && npx tsc --noEmit` → passes (no TS impact, sanity).

## Done when
`WAI_TYPECHECK_CMD="cd frontend && npx tsc --noEmit" WAI_TEST_CMD="true" bash ~/.claude/wai/scripts/task-gate.sh ui-overhaul 003` exits 0
