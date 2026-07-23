---
id: "307"
phase: 3
title: Integrate Chrome into AppRouter (replace slim task-104 layout)
status: passed
depends_on: ["302","105"]
parallel: false
conflicts_with: []
files:
  - remote-frontend/src/AppRouter.tsx
  - remote-frontend/src/AppRouter.test.tsx
irreversible: false
scope_test: "remote-frontend/src/AppRouter.test.tsx"
allowed_change: edit
covers_criteria: [SC8]
---

## Sibling alignment

Read `remote-frontend/src/AppRouter.tsx` (task 105, 186 lines). Currently wraps `/nodes` + `/tasks` + `*` routes in `NormalLayout` (slim task-104 Navbar+BottomNav). Read `design-source/ui_kits/vk-swarm-app/chrome.jsx` (task 302). The `Navbar` from Chrome is the replacement: it has the logo, project button, search, New Task button, ThemeToggle, activity/settings/menu NavIcons, and a nav row with 3 NavTabs (board/nodes/processes). The integration replaces `NormalLayout` with a new layout component that renders `<Navbar>` + `<Outlet>` (no BottomNav — Chrome's nav row handles mobile via the Navbar's responsive collapse). Keep the pre-auth routes (`/login`, `/oauth/callback`, `/invitations/*`) WITHOUT layout. Record the layout-replacement divergence in the ledger.

## Failing test (write first)

Edit `remote-frontend/src/AppRouter.test.tsx` (task 105) — add a test asserting the Chrome Navbar renders on authed routes:

```tsx
// APPEND to the existing test file:
import { render, screen } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { ProfileProvider } from '@/components/ProfileProvider';
import { createMemoryRouter, RouterProvider } from 'react-router-dom';
import { createRoutes } from './AppRouter';

// Reuse the existing renderWithRouter helper if present; otherwise define a local one that
// wraps QueryClientProvider > ProfileProvider > RouterProvider (the authed routes need these
// providers — raw RouterProvider alone triggers hook errors because useProfile is called).
function renderWithRouter(initial: string) {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  const router = createMemoryRouter(createRoutes(), { initialEntries: [initial] });
  return render(
    <QueryClientProvider client={qc}>
      <ProfileProvider>
        <RouterProvider router={router} />
      </ProfileProvider>
    </QueryClientProvider>
  );
}

describe('Chrome integration (SC8)', () => {
  it('authed routes render the Chrome Navbar with Board/Nodes/Processes NavTabs', () => {
    localStorage.setItem('access_token', 'test-token');
    vi.spyOn(globalThis, 'fetch').mockResolvedValue({ ok: true, json: async () => ({ user_id: 'u1', username: 'david', email: 'd@e.io', providers: [] }) } as Response);
    renderWithRouter('/nodes');
    expect(screen.getByText('Board')).toBeTruthy();
    expect(screen.getByText('Nodes')).toBeTruthy();
    expect(screen.getByText('Processes')).toBeTruthy();
  });

  it('pre-auth /login does NOT render the Chrome Navbar', () => {
    const router = createMemoryRouter(createRoutes(), { initialEntries: ['/login'] });
    const { container } = render(<RouterProvider router={router} />);
    expect(container.querySelector('nav')).toBeNull();
  });
});
```

## Change

### File: `remote-frontend/src/AppRouter.tsx` (EDIT)
Replace the `NormalLayout`-wrapped route element with a new inline layout component (or anonymous function) that renders `<Navbar>` from `@/ui/chrome` + `<Outlet />`. The `Navbar` requires props: `project`, `view`, `onView`, `onNewTask`, `theme`, `onToggleTheme`, `onOpenSettings`. For now (task 307 is shell-only, data wiring is 308/309), pass static/placeholder values:
- `project="Hive"` (hardcoded — multi-project is out of scope)
- `view={deriveFromLocation(location.pathname)}` — derive from the current route (`/nodes` → 'nodes', `/tasks` or `/` → 'board', `/processes` or else → 'processes')
- `onView={(v) => navigate(v === 'board' ? '/tasks' : v === 'nodes' ? '/nodes' : '/processes')}`
- `onNewTask={() => {}}` (placeholder — task creation flow is out of scope)
- `theme="dark"` (default; theme toggle state can be local useState, but for shell-only keep dark)
- `onToggleTheme={() => {}}` (placeholder)
- `onOpenSettings={() => navigate('/settings')}` (if a /settings route is added) or `() => {}` (placeholder)

The `/nodes`, `/tasks`, `*` (NotFound) routes use this layout. `/login`, `/oauth/callback`, `/invitations/*` remain WITHOUT layout (unchanged from task 105). Import `Navbar` from `@/ui/chrome` (task 302). Remove the `NormalLayout` import (the slim task-104 layout is no longer used — but do NOT delete `NormalLayout.tsx`/`Navbar.tsx`/`BottomNav.tsx` as files; just stop importing them).

### File: `remote-frontend/src/AppRouter.test.tsx` (EDIT)
Append the new tests shown in Failing test above.

## Allowed moves

- Edit `AppRouter.tsx` to replace the authed-route layout with Chrome's Navbar + Outlet.
- Edit `AppRouter.test.tsx` to add the Chrome integration tests.
- Keep pre-auth routes unchanged.
- No other file may be touched. Do NOT edit `frontend/` (SC9). Do NOT delete the task-104 layout files (just stop importing).

## STOP triggers

- `Navbar` not exported from `@/ui/chrome` (task 302 drift → STOP).
- The existing AppRouter test file has drifted from the task-105 form (would mean a prior task drifted → STOP, escalate).

## Done when
`WAI_TYPECHECK_CMD="cd remote-frontend && npx tsc --noEmit" WAI_TEST_CMD="cd remote-frontend && npx vitest run src/AppRouter.test.tsx" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-design-system 307` exits 0.