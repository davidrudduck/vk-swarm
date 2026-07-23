# Adversarial Review — Phase 2 `vk-swarm-design-system`

**Branch:** `feat/vk-swarm-design-system` (b43e05de..HEAD)  
**Diff:** 1647 lines (phase2.diff)  
**Scope:** `remote-frontend/` only  
**Date:** 2026-07-23  
**Panelist:** OpenCode (DeepSeek-V4-Pro)

---

## Findings

### C1 — CRITICAL — Unlayered `components.css` prevents Tailwind utility overrides

**File/line:** `remote-frontend/src/index.css:17-20`, `remote-frontend/src/styles/components.css:1392-1582`

**Evidence:**

`index.css` places `components.css` between `tailwindcss/components` and `tailwindcss/utilities`:

```css
@import 'tailwindcss/components';

/* Design-system component rules land after Preflight and Tailwind's own
   components layer so they win the cascade over both, and before utilities
   so a consumer's utility classes can still override on a per-element basis. */
@import './styles/components.css';

@import 'tailwindcss/utilities';
```

However, `components.css` has **no `@layer` wrapper**. Tailwind v3's `@import 'tailwindcss/components'` and `@import 'tailwindcss/utilities'` resolve to CSS inside `@layer components` and `@layer utilities` respectively. In Vite (this project's bundler), CSS `@import` statements are inlined by Vite's CSS pipeline before PostCSS processes them. The inlined `components.css` content lands at the **root (unlayered)** level.

Per the CSS Cascade Layers spec, **unlayered styles always beat layered styles**, regardless of source order. The comment in `index.css:19` stating "a consumer's utility classes can still override on a per-element basis" is **incorrect**: because `components.css` is unlayered while Tailwind utilities reside in `@layer utilities`, utilities will **never** win.

**Concrete failure scenario:**

```tsx
// A consumer tries to override the Badge background with a Tailwind utility:
<Badge className="bg-red-500">Error</Badge>
// Expected: red background
// Actual:  .vks-badge--default { background: var(--primary); } (unlayered)
//          beats .bg-red-500 (in @layer utilities).
// The badge stays cyan.
```

Same for `inline-flex` vs `flex`, `cursor: pointer` vs `cursor-not-allowed`, etc. — **any** Tailwind utility class cannot override a property set in `components.css`.

**Fix:** Wrap the entire contents of `components.css` in `@layer components { ... }`. This places it in the same layer as Tailwind's built-in component rules (but after them, since it's imported later), while keeping it before the `utilities` layer so utility classes correctly override.

---

### C2 — IMPORTANT — Tabs missing keyboard navigation (WAI-ARIA tabs pattern)

**File/line:** `remote-frontend/src/components/core/Tabs.tsx:864-877`

**Evidence:**

The Tabs component renders `<button>` elements with `role="tab"` and `aria-selected`, but provides **no keyboard event handlers** (no `onKeyDown`). The WAI-ARIA tabs pattern requires:

- **Arrow Left/Arrow Right:** move focus between tabs (horizontal tablist)
- **Home/End:** move to first/last tab
- Optional: automatic activation on focus vs. manual activation on Enter/Space

Without keyboard handlers, keyboard-only and screen-reader users cannot traverse tabs. Native `<button>` click works for Enter/Space on the focused tab, but there is no way to move between tabs without a pointing device.

This is inherited from the design-source JSX (`dev-docs/designs/.../Tabs.jsx:15-29`), which also lacks keyboard handlers. The decisions-ledger states "JSX held authoritative over task prose where they disagreed" — meaning the port correctly mirrors the design-source. However, the design-source itself has an accessibility gap that carries into the TSX port.

**Fix:** Add an `onKeyDown` handler that implements the WAI-ARIA tablist keyboard model: ArrowLeft/Right to cycle through tabs, Home/End to jump to first/last.

---

### I1 — IMPORTANT — `@keyframes vks-pulse` now activates existing `swarm/NodeCard` animation (behavioral change)

**File/line:** `remote-frontend/src/styles/components.css:157`
**Affected file:** `remote-frontend/src/components/swarm/NodeCard.tsx:48`

**Evidence:**

The existing `@/components/swarm/NodeCard` (used by `Nodes.tsx`) renders an online indicator with:
```tsx
<span className="animate-[vks-pulse_2s_ease-in-out_infinite]" ... />
```

Before Phase 2, there was **no** `@keyframes vks-pulse` defined anywhere. Browsers silently ignore animations referencing undefined keyframes. After Phase 2, `components.css:157` defines:
```css
@keyframes vks-pulse {
  0%   { box-shadow: 0 0 0 0 hsl(var(--vks-emerald-hsl) / 0.5); }
  70%  { box-shadow: 0 0 0 6px hsl(var(--vks-emerald-hsl) / 0); }
  100% { box-shadow: 0 0 0 0 hsl(var(--vks-emerald-hsl) / 0); }
}
```

As a result, **every online node dot on the existing `/nodes` page will now pulse a green glow**. This is likely intended (the class was placed there in anticipation of this keyframe), but it is a visual change to production pages. Confirm this is desired.

---

### M1 — MINOR — StatusBadge drops `?? status` label fallback from design-source

**File/line:** `remote-frontend/src/components/board/StatusBadge.tsx:231`

**Evidence:**

Design-source (`StatusBadge.jsx:11`):
```jsx
{showLabel && <span>{label ?? LABELS[status] ?? status}</span>}
```

TSX port (`StatusBadge.tsx:231`):
```tsx
{showLabel && <span>{label ?? LABELS[status]}</span>}
```

The design-source has a three-stage fallback: `label` → `LABELS[status]` → raw `status` string. The TSX port drops the final fallback. In TypeScript with the `TaskStatus` union type, `LABELS` is a `Record<TaskStatus, string>` covering every valid value, so the third fallback is unreachable at the type level. At runtime (JavaScript), passing an invalid status string would render `undefined` text content instead of the raw status string.

This is low-risk since TypeScript strict checking prevents invalid status values. The decisions-ledger already notes "JSX held authoritative over task prose where they disagreed" for task 205, so this diff is accounted for.

---

### M2 — MINOR (Speculative) — Checkbox/Switch `onCheckedChange` fires in controlled mode even when value doesn't change

**File/line:** `remote-frontend/src/components/core/Checkbox.tsx:633`, `remote-frontend/src/components/core/Switch.tsx:809`

**Evidence:**

In controlled mode, `onCheckedChange?.(!on)` is always called on toggle, even if the parent handler doesn't update the `checked` prop:
```tsx
const on = isControlled ? checked : internal;
const toggle = () => {
  if (disabled) return;
  if (!isControlled) setInternal(!on);
  onCheckedChange?.(!on);  // fires even in controlled mode
};
```

If a parent provides `checked={true}` without an `onCheckedChange` that updates state, the component appears to toggle visually (`data-checked` doesn't update since it's derived from `checked`) but the callback fires with the toggled value. This matches the design-source behavior, so it's a faithful port. No practical defect; noted for completeness.

---

### M3 — MINOR — `Badge.tsx` imports `HTMLAttributes` as value, not type

**File/line:** `remote-frontend/src/components/core/Badge.tsx:464`

```ts
import { HTMLAttributes } from 'react';
```

Other component files use `import type { HTMLAttributes } from 'react';` (e.g., StatusBadge.tsx:198, Tabs.tsx:834, etc.). The value import is not wrong (HTMLAttributes is only used at the type level so bundlers remove it), but inconsistent style. Same pattern in `Button.tsx:499`, `Card.tsx:547`, `Input.tsx:665`.

---

## Axes Checked — Clean

### 1. Port fidelity (TSX vs design-source JSX)

All 14 components were diffed against their design-source JSX siblings:

| Component | Verdict | Notes |
|-----------|---------|-------|
| Button | Clean | `cn()` vs `.filter(Boolean).join(' ')` equivalent |
| Badge | Clean | Minor import style (M3) |
| Card + subcomponents | Clean | `cn()` equivalent |
| Input | Clean | — |
| Switch | Clean | `onCheckedChange?.()` vs `&&` equivalent |
| Checkbox | Clean | — |
| Tabs | Clean (port) | Accessibility gap inherited (C2) |
| Select | Clean | — |
| Loader | Clean | role="status" added (improvement over design-source) |
| StatusBadge | Minor diff (M1) | `?? status` fallback dropped, typed-safe |
| TaskCard | Clean | Uses `<Badge>` instead of raw `<span>`, equivalent output |
| NodeCard | Clean | OS_GLYPH converted from JSX elements to string paths + conditional transform; equivalent output |
| SettingsSection | Clean | — |
| SettingsRow | Clean | — |

### 2. `cn()`/`tailwind-merge` hazards

All `.vks-*` class names are BEM-style custom tokens that `twMerge` does not recognize as Tailwind utility classes. `twMerge` passes non-Tailwind classes through unchanged. No deduplication or mangling risk. **Clean.**

### 3. Cascade — existing page impact

- `Nodes.tsx` uses only `@/components/swarm/NodeCard` (Tailwind utility classes + `animate-[vks-pulse_...]`), no `.vks-*` classes. No clash.
- `components.css` selectors are all namespaced under `.vks-*` prefixes. No un-namespaced global selectors (no `body`, `*`, `div`, etc.).
- `textarea.vks-input` requires the `vks-input` class explicitly, won't match random textareas.
- `@keyframes vks-spin` and `@keyframes vks-pulse` are namespaced; no collision with existing animations.
- The only behavioral change is I1 (`vks-pulse` keyframe now activates on existing swarm/NodeCard).

### 4. Accessibility/behavior

- Switch: `role="switch"`, `aria-checked`, `data-checked` — correct.
- Checkbox: `role="checkbox"`, `aria-checked`, `data-checked` — correct.
- Loader: `role="status"`, `aria-label="Loading"` — correct.
- Tabs: `role="tablist"`, `role="tab"`, `aria-selected` — correct attributes, but missing keyboard navigation (C2).
- Select: native `<select>` with `onChange`, standard `<option>` children — correct.

### 5. Type surface

- All barrels (`core/index.ts`, `board/index.ts`, `settings/index.ts`) use `export *`.
- Every component exports its `Props` interface and any type aliases (variants, enums).
- `TaskCard.tsx` re-exports `TaskStatus` from `StatusBadge` (duplicate export via barrel, TypeScript resolves to single type — harmless).
- `SelectProps` extends `SelectHTMLAttributes` which includes `children`; spread `{...props}` onto `<select>` could receive children. Speculative edge case only. **Clean.**

### 6. Tests

- `components.test.ts`: checks CSS string presence. Can fail if file is missing or class names are removed.
- `button-badge-card.test.tsx`: class name assertions, native prop passthrough, subcomponent existence. All assertions test real conditions.
- `input-switch-checkbox.test.tsx`: class names, toggle state, callback firing. All valid.
- `tabs-select-loader.test.tsx`: class names, data-active, callback, size styling. All valid.
- `statusbadge-taskcard.test.tsx`: class names, variants, label hiding, description/node/badge/attempt rendering. All valid.
- `nodecard.test.tsx`: class names, OS glyph, pulse states, meta/right slots. All valid.
- `settings.test.tsx`: class names, layout variants, error/helper, footer. All valid.
- `render-parity.test.tsx`: overbroad smoke test (checks no-throw only, not output correctness). **Cannot catch silent rendering bugs**, but intentionally so (phase 2 smoke test, not fidelity test). Naming is accurate: "render parity — every component mounts without throwing."
- `tokens/index.test.ts`: added assertion for `@import './styles/components.css'` ordering. Correct.

No test that "cannot fail" in the sense of always-green assertions.

### 7. Collision: `board/NodeCard` vs `swarm/NodeCard`

- Existing: `@/components/swarm/NodeCard` — imported by `Nodes.tsx:3` via explicit path `@/components/swarm/NodeCard`.
- New: `@/components/board/NodeCard` — exported via `@/components/board` barrel.
- **No** code imports `NodeCard` from `@/components/board` except `render-parity.test.tsx:4`.
- No import path ambiguity: the two modules have distinct resolved paths.
- No accidental usage swap in current codebase. **Clean.**

---

## Verdict: **FIX-FIRST**

### Blocker

**C1 — Unlayered `components.css` prevents Tailwind utility overrides**

This contradicts the documented design intent (index.css comment claims utilities can override, but they cannot). Every consumer who tries to tweak a design-system component with a Tailwind utility class (e.g., `className="hidden"`, `className="flex-1"`) will find their class ignored. The fix is a single-line change: wrap `components.css` content in `@layer components { ... }`.

### Should-fix (not blocking merge, but file a tracking ticket)

**C2 — Tabs keyboard navigation** — gap inherited from design-source; should be addressed before Tabs are consumed in production.

