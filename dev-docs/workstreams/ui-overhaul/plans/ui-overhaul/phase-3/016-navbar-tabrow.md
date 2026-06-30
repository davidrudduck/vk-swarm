---
id: "016"
phase: 3
title: "Navbar second tab row (Board/Nodes/Processes) + lastVisitedProjectId tracking"
status: ready
depends_on: ["015", "017"]
parallel: false
conflicts_with: ["015"]
files:
  - frontend/src/components/layout/Navbar.tsx
irreversible: false
scope_test: "N/A"
allowed_change: edit
covers_criteria: [SC11]
---
## Failing test (write first)
N/A — covered by manual verification (visual tab row + active underline; greppable assertions below).
A vitest is not cheap here (Navbar depends on router + many providers) and the active-underline / row
render is a visual concern. Per the plan's Phase-1 note, visual tasks use a `## Manual verification`
section. `WAI_TEST_CMD="true"`.

## Change

Add a SECOND nav row below the existing single nav row, holding three tabs — **Board / Nodes /
Processes** — and persist the current project id to `localStorage` under the key `'lastVisitedProjectId'`
so the Board tab can route back to the last-open project.

This file is ALSO edited by task 015 (the conflict + `depends_on: ["015"]` order this task after it).
The two tasks' anchors are **disjoint**: 015 touches the imports (lines 2-49), the logo JSX (line 142),
the Plus button (~203-211), and inserts `<ThemeToggle/>` (~217); this task edits the `react` import
(line 2) and inserts a new `<nav>` block after the main row closes (~line 318). The `react` import
line (2) is the only shared neighbourhood — 015 leaves it untouched, so the Before text below (current
main) is valid even though 015 commits first.

`projectId` is available in this component via `const { projectId, project } = useProject();`
(line 82) — so the prompt's "cannot determine where projectId is available" STOP does NOT fire; the
tracking effect reads `projectId` directly.

Navigation primitives already in scope: `Link` and `useLocation` are imported from `react-router-dom`
(line 1); `const location = useLocation();` exists (line 80). The existing dropdown nav uses
`location.pathname.startsWith(item.to)` for active state (line 267) — this task mirrors that.

### File: `frontend/src/components/layout/Navbar.tsx`

**Anchor 1 — add `useEffect` to the `react` import (line 2).**
- Before:
```tsx
import { useCallback, useState } from 'react';
```
- After:
```tsx
import { useCallback, useEffect, useState } from 'react';
```

**Anchor 2 — persist `lastVisitedProjectId`.** Add a `useEffect` immediately AFTER the
`const isOAuthLoggedIn = ...` line (line 134), before the `return (`.
- Before:
```tsx
  const isOAuthLoggedIn = loginStatus?.status === 'loggedin';

  return (
```
- After:
```tsx
  const isOAuthLoggedIn = loginStatus?.status === 'loggedin';

  // Persist the active project so the Board tab can route back to it.
  useEffect(() => {
    if (projectId) {
      localStorage.setItem('lastVisitedProjectId', projectId);
    }
  }, [projectId]);

  // Board tab target: last-visited project's task board, else the projects list.
  const lastVisitedProjectId =
    typeof window !== 'undefined'
      ? localStorage.getItem('lastVisitedProjectId')
      : null;
  const boardTo = lastVisitedProjectId
    ? `/projects/${lastVisitedProjectId}/tasks`
    : '/projects';

  return (
```

**Anchor 3 — insert the second nav row.** It goes between the close of the main nav row (`</div>` at
line 318) and the close of the `w-full px-3` wrapper (`</div>` at line 319), so the tab row aligns
with the main row under the same horizontal padding.
- Before:
```tsx
            </div>
          </div>
        </div>
      </div>

      {/* Mobile search dialog */}
```
- After:
```tsx
            </div>
          </div>
        </div>

        {/* Second nav row: primary section tabs */}
        <nav className="flex items-center gap-4 h-9 border-t text-sm">
          {/* TODO(i18n): vk-swarm-node-ui-localize */}
          <Link
            to={boardTo}
            className={
              location.pathname.startsWith('/projects/')
                ? 'mb-[-1px] border-b-2 border-primary py-2 text-foreground'
                : 'mb-[-1px] py-2 text-muted-foreground hover:text-foreground'
            }
          >
            {/* TODO(i18n): vk-swarm-node-ui-localize */}
            Board
          </Link>
          <Link
            to="/nodes"
            className={
              location.pathname === '/nodes'
                ? 'mb-[-1px] border-b-2 border-primary py-2 text-foreground'
                : 'mb-[-1px] py-2 text-muted-foreground hover:text-foreground'
            }
          >
            {/* TODO(i18n): vk-swarm-node-ui-localize */}
            Nodes
          </Link>
          <Link
            to="/processes"
            className={
              location.pathname === '/processes'
                ? 'mb-[-1px] border-b-2 border-primary py-2 text-foreground'
                : 'mb-[-1px] py-2 text-muted-foreground hover:text-foreground'
            }
          >
            {/* TODO(i18n): vk-swarm-node-ui-localize */}
            Processes
          </Link>
        </nav>
      </div>

      {/* Mobile search dialog */}
```
Notes:
- **Board** → `boardTo` (computed in Anchor 2): the last-visited project's task board, falling back to
  `/projects` when `localStorage` has no `'lastVisitedProjectId'`. Active when
  `location.pathname.startsWith('/projects/')`.
- **Nodes** → `/nodes` (route created in task 017 — hence `depends_on: ["017"]`). Active when
  `pathname === '/nodes'`.
- **Processes** → `/processes` (existing route, App.tsx). Active when `pathname === '/processes'`.
- The active tab carries `border-b-2 border-primary` (a 2px cyan bottom border via the `--primary`
  token) with `mb-[-1px]` so the underline overlaps the row's `border-t` rather than stacking below
  it.

## Allowed moves
- ONLY the three anchors above in `frontend/src/components/layout/Navbar.tsx`: add `useEffect` to the
  `react` import; add the persistence effect + `boardTo` computation before `return`; insert the
  `<nav>` tab row after the main row close. Do NOT alter the main nav row, the dropdown `INTERNAL_NAV`,
  the mobile search dialog, or any other file.

## STOP triggers
- 015's changes are absent — `grep '<VKSLogo' frontend/src/components/layout/Navbar.tsx` finds no match
  (halt — task 015 not applied; this task is ordered after it and must not run on stale chrome).
- `projectId` is no longer available from `useProject()` (the line
  `const { projectId, project } = useProject();` is gone) — halt; the `lastVisitedProjectId` tracking
  has no source.
- The `</div></div></div></div>` close sequence before the mobile-search-dialog comment differs
  materially from the Before text (halt — the row structure changed since decompose; re-locate the
  main-row close).

## Manual verification (record in decisions-ledger)
- `grep -i 'Nodes' frontend/src/components/layout/Navbar.tsx` → match (tab present).
- `grep 'border-b-2 border-primary' frontend/src/components/layout/Navbar.tsx` → ≥3 matches (active
  underline on each tab).
- `grep "localStorage.setItem('lastVisitedProjectId'" frontend/src/components/layout/Navbar.tsx` →
  match (tracking write).
- `cd frontend && npx tsc --noEmit` → passes.
- Manual browser check (SC11): the second row renders below the main bar; navigating to a project,
  `/nodes`, and `/processes` each lights the correct tab's cyan underline; Board returns to the last
  project after visiting another route.

## Done when
`WAI_TYPECHECK_CMD="cd frontend && npx tsc --noEmit" WAI_TEST_CMD="true" bash ~/.claude/wai/scripts/task-gate.sh ui-overhaul 016` exits 0
