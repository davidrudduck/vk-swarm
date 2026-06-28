---
id: "001"
phase: 1
title: Promote --vks-* primitives to :root, merge brand remap into .dark, create .light block
status: ready
depends_on: []
parallel: false
conflicts_with: ["002", "003"]
files:
  - frontend/src/styles/index.css
irreversible: false
scope_test: "N/A"
allowed_change: edit
covers_criteria: [SC22]
---
## Failing test (write first)
N/A — covered by manual verification (CSS cascade; no cheap unit test). The existing
`frontend/src/__tests__/DesignSystem.test.tsx` must keep passing (it references `.vks-theme`,
which this task LEAVES in place).

## Change

Scope decision C (spec D10): the `--vks-*` brand palette and the `--_*`→brand remap currently live
only in the `.vks-theme {}` block (lines ~91–144), which `ThemeProvider` never applies. Bring them
into the live cascade.

### File: `frontend/src/styles/index.css`

**Anchor 1 — `:root {` block (light defaults, opens at line ~11).** Add the `--vks-*` primitives so
they resolve in every applied selector. Insert, immediately BEFORE the closing `}` of the first
`:root {` block (after the `--_console-*` light tokens, ~line 52):

```css
    /* VKS brand primitives (Midnight Terminal) — promoted to :root so they resolve
       in the applied .dark/.light cascade (scope C / D10). Corrected values. */
    --vks-void: 240 20% 5%;
    --vks-surface: 240 18% 9%;
    --vks-surface-bright: 240 16% 12%;
    --vks-cyan: 190 100% 50%;
    --vks-amber: 43 100% 50%;
    --vks-emerald: 152 100% 50%;
    --vks-coral: 0 100% 71%;
    --vks-violet: 270 91% 65%;
    --vks-text: 240 6% 90%;
    --vks-text-muted: 240 4% 46%;
    --vks-text-dim: 240 3% 26%;
    --strip-width: 4px;
```

**Anchor 2 — `.dark {` block (opens at line ~55).** Replace the muted `--_*` mappings with the
brand remap (mirroring the existing `.vks-theme` body). Replace the CURRENT body:

- Before (the muted mappings, lines ~57–81):
```css
    --_background: 48 4% 16%;
    --_foreground: 48 7% 85%;
    --_primary: var(--_muted);
    --_primary-foreground: var(--_muted-foreground);
    --_secondary: var(--_muted);
    --_secondary-foreground: 48 2% 65%;
    --_muted: 60 2% 18%;
    --_muted-foreground: var(--_foreground);
    --_accent: var(--_background);
    --_accent-foreground: 210 40% 98%;
    --_destructive: 0 45% 55%;
    --_destructive-foreground: var(--_background-foreground);
    --_border: 60 2% 25%;
    --_input: var(--_border);
    --_ring: 212.7 26.8% 83.9%;

    /* Status (dark) */
    --_success: 138.5 76.5% 47.7%;
    --_success-foreground: 138.5 76.5% 96.7%;
    --_warning: 32.2 95% 44.1%;
    --_warning-foreground: 26 83.3% 14.1%;
    --_info: 217.2 91.2% 59.8%;
    --_info-foreground: 222.2 84% 4.9%;
```
- After:
```css
    --font-heading: 'Source Serif 4', Georgia, serif;
    --_background: var(--vks-void);
    --_foreground: var(--vks-text);
    --_primary: var(--vks-cyan);
    --_primary-foreground: var(--vks-void);
    --_secondary: var(--vks-surface-bright);
    --_secondary-foreground: var(--vks-text);
    --_muted: var(--vks-surface);
    --_muted-foreground: var(--vks-text-muted);
    --_accent: var(--vks-violet);
    --_accent-foreground: var(--vks-text);
    --_destructive: var(--vks-coral);
    --_destructive-foreground: var(--vks-void);
    --_border: var(--vks-surface-bright);
    --_input: var(--vks-surface);
    --_ring: var(--vks-cyan);

    /* Status (dark) — brand palette */
    --_success: var(--vks-emerald);
    --_success-foreground: var(--vks-void);
    --_warning: var(--vks-amber);
    --_warning-foreground: var(--vks-void);
    --_info: var(--vks-cyan);
    --_info-foreground: var(--vks-void);
```
Leave the `--_console-*` (dark) lines (~83–87) and the `color-scheme: dark;` line UNCHANGED.

**Anchor 3 — create a `.light {` block.** It does not exist today. Add it immediately AFTER the
`.dark { … }` block closes (~line 88) and BEFORE the `/* VK-Swarm "Midnight Terminal" Theme */`
comment / `.vks-theme` block. `ThemeProvider` applies `.light` via `classList.add('light')`, so this
takes effect:

```css
  /* Light theme — applied via ThemeProvider classList.add('light').
     Inherits :root defaults; overrides primary for AA contrast + status tokens (added in 002). */
  .light {
    color-scheme: light;
    --_primary: 192 100% 35%;
    --_primary-foreground: 0 0% 100%;
  }
```

Leave the `.vks-theme {}` block (~91–144) UNCHANGED — it is exercised by `DesignSystem.test.tsx`.

## Allowed moves
- ONLY: add the `--vks-*` primitive block to `:root`; replace the `.dark` `--_*` mappings as above;
  add the new `.light {}` block. Do not touch `:root` font/`--_*` light values, `--_console-*`,
  the public-token layer (`--background: var(...)` etc.), or `.vks-theme`.

## STOP triggers
- The `.dark {` block body differs materially from the Before text (re-grep `--_background: 48 4% 16%`
  to locate; if absent, halt — the file changed since decompose).
- A `.light {` block already exists (halt; reconcile rather than duplicate).
- `--vks-void` is already defined under `:root` (halt; the cascade may already be partly migrated).

## Manual verification (record in decisions-ledger)
- `grep -A 40 '\.dark {' frontend/src/styles/index.css | grep -- '--_primary: var(--vks-cyan)'` → match (SC22).
- `grep -E '^\s*--vks-void: 240 20% 5%;' frontend/src/styles/index.css` → match under `:root`.
- `grep -n '\.light {' frontend/src/styles/index.css` → match.
- `cd frontend && npx tsc --noEmit` → passes (no TS impact, sanity).
- `cd frontend && npx vitest run src/__tests__/DesignSystem.test.tsx` → passes (`.vks-theme` intact).

## Done when
`WAI_TYPECHECK_CMD="cd frontend && npx tsc --noEmit" WAI_TEST_CMD="cd frontend && npx vitest run {scope}" bash ~/.claude/wai/scripts/task-gate.sh ui-overhaul 001` exits 0
