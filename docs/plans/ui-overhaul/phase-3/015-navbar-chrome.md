---
id: "015"
phase: 3
title: "Navbar chrome: Logo→VKSLogo, +Task text button, ThemeToggle placement"
status: ready
depends_on: ["011", "012"]
parallel: false
conflicts_with: ["016"]
files:
  - frontend/src/components/layout/Navbar.tsx
irreversible: false
scope_test: "N/A"
allowed_change: edit
covers_criteria: [SC12, SC13, SC16]
forbid_after: ["@/components/Logo"]
---
## Failing test (write first)
N/A — covered by manual verification (visual navbar chrome; greppable assertions below). A vitest is
not cheap here (Navbar depends on many providers — router, ProjectContext, SearchContext,
ConfigProvider) and adds no coverage a `tsc --noEmit` + grep gate doesn't already give.
`WAI_TEST_CMD="true"`.

## Change

Three surgical sub-edits to `frontend/src/components/layout/Navbar.tsx`, all in the single existing
nav row (this task does NOT add a second row — that is task 016). Producers: `VKSLogo` (task 011,
wordmark font) and `ThemeToggle` (task 012, default export `{ className? }`).

### File: `frontend/src/components/layout/Navbar.tsx`

**Sub-edit A (SC12) — swap `Logo` → `VKSLogo`.**

Anchor 1 — the import at line 24.
- Before:
```tsx
import { Logo } from '@/components/Logo';
```
- After:
```tsx
import { VKSLogo } from '@/components/VKSLogo';
```
(`VKSLogo` is a named export from `frontend/src/components/VKSLogo.tsx`; props `{ className?: string;
alwaysFull?: boolean }`. A `Logo` default/named export is no longer referenced anywhere in the file —
grep confirmed it is used ONLY at lines 24 and 142 — so removing its import is correct. `forbid_after`
guards against a re-introduced `@/components/Logo` reference.)

Anchor 2 — the JSX at line 142.
- Before:
```tsx
              <Logo className="h-4 sm:h-6 w-auto" />
```
- After:
```tsx
              <VKSLogo className="text-sm sm:text-base" />
```
NOTE (pinned decision, do NOT copy the old classes): the original `h-4 sm:h-6 w-auto` are
height/width classes for the old **image** `Logo`. `VKSLogo` is a **text** component (`font-code`,
sized by font-size), so height/width classes are inert on it. Use the responsive text-size classes
above instead. This intentional divergence keeps the wordmark legible at both breakpoints.

**Sub-edit B (SC13) — `+ Task` button (icon-only → text label).**

Anchor 3 — the Plus button at lines 203-211 (inside the `projectId ? (...)` branch).
- Before:
```tsx
                {/* Plus button - always visible for quick task creation */}
                <Button
                  variant="ghost"
                  size="icon"
                  className="h-9 w-9"
                  onClick={handleCreateTask}
                  aria-label="Create new task"
                >
                  <Plus className="h-4 w-4" />
                </Button>
```
- After:
```tsx
                {/* Task creation button - text label for discoverability */}
                <Button
                  variant="default"
                  size="sm"
                  onClick={handleCreateTask}
                  aria-label="Create new task"
                >
                  {/* TODO(i18n): vk-swarm-node-ui-localize */}
                  + Task
                </Button>
```
The `<Plus className="h-4 w-4" />` glyph is removed (replaced by the literal text `+ Task`). `Plus`
is now UNUSED in the file (it appeared only here). `tsconfig.json` sets `noUnusedLocals: true`, so it
MUST be removed from the lucide-react import or `tsc --noEmit` fails.

Anchor 4 — the lucide-react import block (lines 11-23).
- Before:
```tsx
import {
  FolderOpen,
  Settings,
  BookOpen,
  MessageCircleQuestion,
  Menu,
  Plus,
  LogOut,
  LogIn,
  Archive,
  Activity,
  Search,
} from 'lucide-react';
```
- After:
```tsx
import {
  FolderOpen,
  Settings,
  BookOpen,
  MessageCircleQuestion,
  Menu,
  LogOut,
  LogIn,
  Archive,
  Activity,
  Search,
} from 'lucide-react';
```

**Sub-edit C (SC16) — place `<ThemeToggle />` in the right action cluster.**

Anchor 5 — add the import. Insert immediately AFTER the `ProjectSwitcher` import (line 49):
- Before:
```tsx
import { ProjectSwitcher } from './ProjectSwitcher';
```
- After:
```tsx
import { ProjectSwitcher } from './ProjectSwitcher';
import ThemeToggle from '@/components/ThemeToggle';
```
(`ThemeToggle` is the **default** export from `frontend/src/components/ThemeToggle.tsx`, props
`{ className?: string }` — task 012 contract.)

Anchor 6 — render it in the right action cluster, immediately before `<ActivityFeed />` (line 217).
- Before:
```tsx
            <div className="flex items-center gap-1">
              <ActivityFeed />
```
- After:
```tsx
            <div className="flex items-center gap-1">
              <ThemeToggle />
              <ActivityFeed />
```

## Allowed moves
- ONLY the six anchors above in `frontend/src/components/layout/Navbar.tsx`:
  1. swap the `Logo` import for the `VKSLogo` import (line 24);
  2. swap the `<Logo … />` JSX for `<VKSLogo className="text-sm sm:text-base" />` (line 142);
  3. convert the Plus icon button to the `+ Task` text button (lines 203-211);
  4. remove the now-unused `Plus` from the lucide-react import block (lines 11-23);
  5. add the `ThemeToggle` default import (after line 49);
  6. render `<ThemeToggle />` before `<ActivityFeed />` (line 217).
- Do NOT add a second nav row (task 016), touch the search bar, the dropdown menu, the settings
  button, or any other file.

## STOP triggers
- The `import { Logo } from '@/components/Logo';` line is absent (halt — file changed since decompose;
  re-derive the swap).
- The Plus button body differs materially from the Before text (re-grep `aria-label="Create new task"`
  to locate; if absent, halt).
- `frontend/src/components/VKSLogo.tsx` does not export `VKSLogo`, or
  `frontend/src/components/ThemeToggle.tsx` does not exist / lacks a default export (halt — producers
  011/012 not applied; re-derive the imports).

## Manual verification (record in decisions-ledger)
- `grep '<VKSLogo' frontend/src/components/layout/Navbar.tsx` → match (SC12).
- `grep -c '@/components/Logo' frontend/src/components/layout/Navbar.tsx` → `0` (Logo import gone;
  satisfies `forbid_after`).
- `grep '+ Task' frontend/src/components/layout/Navbar.tsx` → match (SC13 text label).
- `grep -c 'Plus' frontend/src/components/layout/Navbar.tsx` → `0` (unused icon import removed).
- `grep '<ThemeToggle' frontend/src/components/layout/Navbar.tsx` → match (SC16).
- `cd frontend && npx tsc --noEmit` → passes (also enforces no unused `Logo`/`Plus` imports via
  `noUnusedLocals`).

## Done when
`WAI_TYPECHECK_CMD="cd frontend && npx tsc --noEmit" WAI_TEST_CMD="true" bash ~/.claude/wai/scripts/task-gate.sh ui-overhaul 015` exits 0
