---
id: "004"
phase: 1
title: Add a wordmark key to the tailwind fontFamily map
status: ready
depends_on: []
parallel: false
conflicts_with: []
files:
  - frontend/tailwind.config.js
irreversible: false
scope_test: "N/A"
allowed_change: edit
covers_criteria: []
---
## Failing test (write first)
N/A — covered by manual verification (greppable config assertion + typecheck below).

## Change

Add a `wordmark` key to the tailwind `fontFamily` map. `VKSLogo` consumes `font-wordmark` in task
011; this task only adds the alias. A `'chivo-mono'` key already exists in the map — `wordmark` is
the spec'd alias `VKSLogo` consumes (it is intentionally a separate, named key, not a reuse of
`'chivo-mono'`).

### File: `frontend/tailwind.config.js`

**Anchor — the `fontFamily` block (≈ lines 139–147).**

- Before:
```js
      fontFamily: {
        ui: 'var(--font-ui)',
        code: 'var(--font-code)',
        prose: 'var(--font-prose)',
        mono: 'var(--font-code)', // Alias for backwards compatibility
        'chivo-mono': ['Chivo Mono', 'Noto Emoji', 'monospace'], // Keep for direct usage
        heading: 'var(--font-heading, var(--font-ui))', // VKS heading font with fallback
        serif: ['"Source Serif 4"', 'Georgia', 'serif'], // Direct serif usage
      },
```
- After (add the `wordmark` line after the `'chivo-mono'` line):
```js
      fontFamily: {
        ui: 'var(--font-ui)',
        code: 'var(--font-code)',
        prose: 'var(--font-prose)',
        mono: 'var(--font-code)', // Alias for backwards compatibility
        'chivo-mono': ['Chivo Mono', 'Noto Emoji', 'monospace'], // Keep for direct usage
        wordmark: ["'Chivo Mono'", 'monospace'],
        heading: 'var(--font-heading, var(--font-ui))', // VKS heading font with fallback
        serif: ['"Source Serif 4"', 'Georgia', 'serif'], // Direct serif usage
      },
```

## Allowed moves
- ONLY: add the single `wordmark: ["'Chivo Mono'", 'monospace'],` line inside the existing
  `fontFamily` object. Do not touch any other key, the `keyframes` block, or any other file.

## STOP triggers
- The `fontFamily` block differs from the Before text (halt — the file changed since decompose;
  re-locate and reconcile rather than blindly editing).

## Manual verification (record in decisions-ledger)
- `grep 'wordmark:' frontend/tailwind.config.js` → match.
- `cd frontend && npx tsc --noEmit` → passes.

## Done when
`WAI_TYPECHECK_CMD="cd frontend && npx tsc --noEmit" WAI_TEST_CMD="true" bash ~/.claude/wai/scripts/task-gate.sh ui-overhaul 004` exits 0
