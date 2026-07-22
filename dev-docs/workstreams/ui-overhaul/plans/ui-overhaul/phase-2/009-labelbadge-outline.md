---
id: "009"
phase: 2
title: LabelBadge outline variant
status: passed
depends_on: []
parallel: false
conflicts_with: []
files:
  - frontend/src/components/labels/LabelBadge.tsx
irreversible: false
scope_test: "N/A"
allowed_change: edit
covers_criteria: []
---
## Failing test (write first)
N/A — covered by manual verification (typecheck + call-site compile below). No existing unit test
references `LabelBadge`.

## Change

`LabelBadge` currently always renders a solid colour fill (`backgroundColor: label.color` with a
contrast-computed text colour). Add an opt-in `outline` variant for the task-card context: a
transparent background with a coloured border and coloured text. Keep it fully backward-compatible —
the new prop defaults to `'solid'`, so the seven existing call sites
(`LabelManager`, `CompactLabelList`, `LabelEditDialog`, `SwarmLabelsSection`, `LabelSelect`,
`LabelPicker`, `MergeLabelsDialog`) render exactly as before.

### File: `frontend/src/components/labels/LabelBadge.tsx`

**Anchor 1 — `LabelBadgeProps` (lines 6–12).** Add the optional `variant` prop.

- Before:
```tsx
interface LabelBadgeProps {
  label: Label;
  size?: 'sm' | 'md';
  onClick?: () => void;
  onRemove?: () => void;
  className?: string;
}
```
- After:
```tsx
interface LabelBadgeProps {
  label: Label;
  size?: 'sm' | 'md';
  /** 'solid' (default) fills with the label colour; 'outline' uses a transparent
   *  background with a coloured border + text (task-card context). */
  variant?: 'solid' | 'outline';
  onClick?: () => void;
  onRemove?: () => void;
  className?: string;
}
```

**Anchor 2 — destructuring (lines 31–37).** Add `variant` with a default of `'solid'`.

- Before:
```tsx
export function LabelBadge({
  label,
  size = 'md',
  onClick,
  onRemove,
  className,
}: LabelBadgeProps) {
```
- After:
```tsx
export function LabelBadge({
  label,
  size = 'md',
  variant = 'solid',
  onClick,
  onRemove,
  className,
}: LabelBadgeProps) {
```

**Anchor 3 — the badge `className` (~lines 52–58).** Add a border in the outline case so
`borderColor` (set in Anchor 4) renders. The solid case must NOT gain a border (keep current
appearance).

- Before:
```tsx
      className={cn(
        'inline-flex items-center rounded-full font-medium transition-opacity',
        sizeClasses[size],
        onClick && 'cursor-pointer hover:opacity-80',
        className
      )}
```
- After:
```tsx
      className={cn(
        'inline-flex items-center rounded-full font-medium transition-opacity',
        sizeClasses[size],
        variant === 'outline' && 'border',
        onClick && 'cursor-pointer hover:opacity-80',
        className
      )}
```

**Anchor 4 — the inline `style` object (~lines 59–62).** Branch the style on `variant`. Solid keeps
the existing fill; outline uses a transparent background with the label colour driving border + text.

- Before:
```tsx
      style={{
        backgroundColor: label.color,
        color: textColor,
      }}
```
- After:
```tsx
      style={
        variant === 'outline'
          ? {
              backgroundColor: 'transparent',
              color: label.color,
              borderColor: label.color,
            }
          : {
              backgroundColor: label.color,
              color: textColor,
            }
      }
```

Leave `getContrastColor`, `sizeClasses`, `iconSizes`, the icon/name/remove JSX, and `textColor`
(still used by the solid branch) UNCHANGED.

## Allowed moves
- ONLY: add the `variant?: 'solid' | 'outline'` prop, default it in the destructure, branch the
  inline `style`, and add the conditional `border` class. No new call site, no change to the seven
  existing consumers, no signature change beyond the additive optional prop.

## STOP triggers
- The `style` object or `className` differs materially from the Before text (the
  `backgroundColor: label.color` / `color: textColor` block is absent): halt — file changed since
  decompose.
- A `variant` prop already exists on `LabelBadgeProps` (halt; reconcile rather than duplicate).

## Manual verification (record in decisions-ledger)
- Read corrected source: `variant?: 'solid' | 'outline'` on props; `variant = 'solid'` default;
  outline branch sets transparent bg + `borderColor`/`color` = `label.color`; `border` class added
  only when outline.
- `cd frontend && npx tsc --noEmit` → passes (additive optional prop; the seven existing call
  sites still compile unchanged).

## Done when
`WAI_TYPECHECK_CMD="cd frontend && npx tsc --noEmit" WAI_TEST_CMD="true" bash ~/.claude/wai/scripts/task-gate.sh ui-overhaul 009` exits 0
