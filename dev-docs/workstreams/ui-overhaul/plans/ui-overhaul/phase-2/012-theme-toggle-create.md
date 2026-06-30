---
id: "012"
phase: 2
title: Create ThemeToggle component (ghost icon, DARK↔LIGHT)
status: ready
depends_on: []
parallel: false
conflicts_with: []
files:
  - frontend/src/components/ThemeToggle.tsx
irreversible: false
scope_test: "N/A"
allowed_change: create
covers_criteria: [SC16]
---
## Failing test (write first)
N/A — covered by manual verification (scope_test N/A). Wiring a vitest is non-trivial: ThemeToggle
depends on BOTH `ThemeProvider` (`useTheme`) and `ConfigProvider` (`useUserSystem`), so a render
test must mock/wrap two providers. Per the plan's Phase-1 note, visual/provider-heavy tasks use a
`## Manual verification` section instead. `WAI_TEST_CMD="true"`.

## Change

NEW component per the plan's Shared-interface contract: **default export `ThemeToggle`** from
`frontend/src/components/ThemeToggle.tsx`, props `{ className?: string }`, no required props. A ghost
icon button (Sun/Moon from `lucide-react`) that toggles **DARK ↔ LIGHT only** (it never sets
SYSTEM).

### Verified APIs (read before writing — do NOT re-derive)
- `useTheme()` (from `@/components/ThemeProvider`) returns `{ theme, setTheme }`;
  `setTheme(newTheme: ThemeMode)`. **`setTheme` only updates React state — it does NOT persist.**
- `ThemeMode` is an **enum** in `shared/types` (`ThemeMode.DARK`, `ThemeMode.LIGHT`,
  `ThemeMode.SYSTEM`). Import path: `import { ThemeMode } from 'shared/types';` (exactly as
  `ThemeProvider.tsx` imports it).
- Persistence lives in `ConfigProvider`: `useUserSystem()` (from `@/components/ConfigProvider`)
  exposes `updateAndSaveConfig(updates: Partial<Config>) => Promise<boolean>`. `GeneralSettings`
  persists theme by calling `updateAndSaveConfig({ theme })` then `setTheme(theme)`. ThemeToggle
  MUST mirror this so SC20 (persistence) holds — calling `setTheme` alone would NOT persist.
- Ghost icon Button pattern (match `Navbar.tsx`): `import { Button } from '@/components/ui/button';`
  used as `<Button variant="ghost" size="icon" className="h-9 w-9" onClick={…} aria-label={…}>`.

### Behaviour
- Read `theme` via `useTheme()`. A theme is "dark" when `theme === ThemeMode.DARK` (SYSTEM falls
  through to the light branch, so the first click from SYSTEM goes to DARK — acceptable; the toggle
  is a binary DARK↔LIGHT control per the contract).
- On click compute `next = isDark ? ThemeMode.LIGHT : ThemeMode.DARK`, then
  `void updateAndSaveConfig({ theme: next });` and `setTheme(next);` (persist + apply, mirroring
  GeneralSettings).
- Icon: show `Moon` when dark, `Sun` when light.
- `aria-label`: English literal with a `// TODO(i18n): vk-swarm-node-ui-localize` comment (spec D7).
- Merge `className` via `cn()` from `@/lib/utils`.

### Full file contents (allowed_change: create)
```tsx
import { Moon, Sun } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';
import { useTheme } from '@/components/ThemeProvider';
import { useUserSystem } from '@/components/ConfigProvider';
import { ThemeMode } from 'shared/types';

interface ThemeToggleProps {
  className?: string;
}

/**
 * Ghost icon button toggling between DARK and LIGHT themes.
 *
 * Persistence: `setTheme` only updates React state, so we also call
 * `updateAndSaveConfig({ theme })` (mirroring GeneralSettings) to persist the
 * choice to config (SC20). Toggles binary DARK<->LIGHT only (never SYSTEM).
 */
export function ThemeToggle({ className }: ThemeToggleProps) {
  const { theme, setTheme } = useTheme();
  const { updateAndSaveConfig } = useUserSystem();

  const isDark = theme === ThemeMode.DARK;

  const handleToggle = () => {
    const next = isDark ? ThemeMode.LIGHT : ThemeMode.DARK;
    void updateAndSaveConfig({ theme: next });
    setTheme(next);
  };

  return (
    <Button
      variant="ghost"
      size="icon"
      className={cn('h-9 w-9', className)}
      onClick={handleToggle}
      // TODO(i18n): vk-swarm-node-ui-localize
      aria-label="Toggle theme"
    >
      {isDark ? (
        <Moon className="h-4 w-4" />
      ) : (
        <Sun className="h-4 w-4" />
      )}
    </Button>
  );
}

export default ThemeToggle;
```

## Allowed moves
- ONLY: create `frontend/src/components/ThemeToggle.tsx` with the contents above. No edits to
  `ThemeProvider`, `ConfigProvider`, `Navbar`, or any other file (placement is task 015).

## STOP triggers
- `frontend/src/components/ThemeToggle.tsx` already exists (halt; reconcile rather than overwrite).
- `useUserSystem` does NOT export `updateAndSaveConfig`, or `useTheme` does NOT return `setTheme`
  (halt — the provider API changed since decompose; re-derive the persistence wiring).
- `ThemeMode` is no longer an enum exported from `shared/types` (halt — import would break).

## Manual verification (record in decisions-ledger)
- `cd frontend && npx tsc --noEmit` → passes.
- `grep -E 'export default ThemeToggle|export function ThemeToggle' frontend/src/components/ThemeToggle.tsx` → match.
- Rendered behaviour (once placed by 015): clicking flips the `light`/`dark` class on `<html>` and
  persists `theme` to config (reload retains the choice — SC20).

## Done when
`WAI_TYPECHECK_CMD="cd frontend && npx tsc --noEmit" WAI_TEST_CMD="true" bash ~/.claude/wai/scripts/task-gate.sh ui-overhaul 012` exits 0
