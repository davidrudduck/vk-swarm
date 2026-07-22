---
id: "002"
phase: 1
title: Add --status-* tokens, semantic aliases, --strip-width consumption
status: passed
depends_on: ["001"]
parallel: false
conflicts_with: ["001", "003"]
files:
  - frontend/src/styles/index.css
irreversible: false
scope_test: "N/A"
allowed_change: edit
covers_criteria: [SC6, SC7]
---
## Failing test (write first)
N/A — covered by manual verification (greppable token assertions below).

## Change

### File: `frontend/src/styles/index.css`

**Anchor 1 — `.dark {` block (after the brand remap added in 001, before `--_console-*`).** Add the
five status tokens + three semantic aliases:
```css
    /* Status colours (dark) */
    --status-todo: 240 5% 63%;
    --status-inprogress: 217 91% 60%;
    --status-inreview: 43 100% 50%;
    --status-done: 152 100% 50%;
    --status-cancelled: 0 100% 71%;
    /* Semantic surface/border aliases */
    --surface-card: var(--vks-surface);
    --surface-raised: var(--vks-surface-bright);
    --border-strong: 240 10% 16%;
```

**Anchor 2 — `.light {` block (created in 001).** Add the light status overrides:
```css
    --status-todo: 220 9% 46%;
    --status-inprogress: 221 83% 53%;
    --status-inreview: 38 100% 34%;
    --status-done: 153 83% 30%;
    --status-cancelled: 0 62% 52%;
    --surface-card: 0 0% 100%;
    --surface-raised: 210 40% 96%;
    --border-strong: 214 20% 80%;
```

## Allowed moves
- ONLY add the token declarations above inside the existing `.dark {}` and `.light {}` blocks. No
  other selector, no public-token-layer change, no component file.

## STOP triggers
- The `.light {}` block from task 001 is absent (halt — 001 not applied).
- `--status-todo` already defined anywhere (halt — duplicate).

## Manual verification (record in decisions-ledger)
- `grep -A 100 '\.dark {' frontend/src/styles/index.css | grep -- '--status-todo'` → match (SC6).
- `grep -E '\-\-(border-strong|surface-card|surface-raised)' frontend/src/styles/index.css` → ≥3 matches (SC7).
- Light overrides present: `grep -A 60 '\.light {' frontend/src/styles/index.css | grep -- '--status-done: 153 83% 30%'` → match.
- `cd frontend && npx tsc --noEmit` → passes.

## Done when
`WAI_TYPECHECK_CMD="cd frontend && npx tsc --noEmit" WAI_TEST_CMD="true" bash ~/.claude/wai/scripts/task-gate.sh ui-overhaul 002` exits 0
