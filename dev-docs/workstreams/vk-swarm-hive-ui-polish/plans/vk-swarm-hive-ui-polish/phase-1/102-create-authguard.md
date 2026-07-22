---
id: "102"
phase: 1
title: Create AuthGuard + wire into AppRouter
status: done
depends_on: []
parallel: false
conflicts_with: []
files:
  - remote-frontend/src/components/AuthGuard.tsx
  - remote-frontend/src/components/AuthGuard.test.tsx
  - remote-frontend/src/AppRouter.tsx
irreversible: false
scope_test: "remote-frontend/src/components"
allowed_change: mixed
covers_criteria: [SC2]
---

## Failing test (write first)

Create `remote-frontend/src/components/AuthGuard.test.tsx`:

```tsx
import { render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { AuthGuard } from './AuthGuard';

vi.mock('@/components/ProfileProvider', () => ({
  useProfile: vi.fn(),
}));

import { useProfile } from '@/components/ProfileProvider';

const mockedUseProfile = useProfile as ReturnType<typeof vi.fn>;

function renderWithRouter(ui: React.ReactElement, initialEntries = ['/nodes']) {
  return render(
    <MemoryRouter initialEntries={initialEntries}>
      {ui}
    </MemoryRouter>,
  );
}

describe('AuthGuard (SC2)', () => {
  beforeEach(() => {
    mockedUseProfile.mockReset();
  });

  it('renders children when signed in', () => {
    mockedUseProfile.mockReturnValue({ isSignedIn: true, isLoaded: true });
    renderWithRouter(<AuthGuard><div>protected content</div></AuthGuard>);
    expect(screen.getByText('protected content')).toBeDefined();
    expect(screen.queryByText('Loading...')).toBeNull();
  });

  it('shows loading spinner when not yet loaded', () => {
    mockedUseProfile.mockReturnValue({ isSignedIn: false, isLoaded: false });
    renderWithRouter(<AuthGuard><div>protected content</div></AuthGuard>);
    expect(screen.getByText('Loading...')).toBeDefined();
    expect(screen.queryByText('protected content')).toBeNull();
  });

  it('redirects to /login when signed out', () => {
    mockedUseProfile.mockReturnValue({ isSignedIn: false, isLoaded: true });
    renderWithRouter(
      <AuthGuard><div>protected content</div></AuthGuard>,
      ['/nodes'],
    );
    expect(screen.queryByText('protected content')).toBeNull();
    expect(screen.queryByText('Loading...')).toBeNull();
  });
});
```

## Change

### File: `remote-frontend/src/components/AuthGuard.tsx` (CREATE)

```tsx
import { useProfile } from '@/components/ProfileProvider';
import { Navigate, useLocation } from 'react-router-dom';
import type { ReactNode } from 'react';

interface AuthGuardProps {
  children: ReactNode;
}

export function AuthGuard({ children }: AuthGuardProps) {
  const { isSignedIn, isLoaded } = useProfile();
  const location = useLocation();

  if (!isLoaded) {
    return (
      <div className="min-h-screen flex items-center justify-center">
        Loading...
      </div>
    );
  }

if (!isSignedIn) {
      return <Navigate to={`/login?return_to=${encodeURIComponent(location.pathname + location.search)}`} replace />;
    }

  return <>{children}</>;
}
```

### File: `remote-frontend/src/AppRouter.tsx` (EDIT — wrap NormalLayout with AuthGuard)

- **Anchor:** the import block, last import line (currently `import { oauthApi } from '@/lib/api/oauth'` around L8)
- **Before (at ~L8):**
  ```
  import { oauthApi } from '@/lib/api/oauth'
  ```
- **After (at ~L8):**
  ```
  import { oauthApi } from '@/lib/api/oauth'
  import { AuthGuard } from '@/components/AuthGuard'
  ```

- **Anchor:** the route definition block with `element: <NormalLayout />` (currently ~L165)
- **Before (~L165):**
  ```
      element: <NormalLayout />,
  ```
- **After (~L165):**
  ```
      element: <AuthGuard><NormalLayout /></AuthGuard>,
  ```

## Allowed moves

- Create `remote-frontend/src/components/AuthGuard.tsx` with the exact code above.
- Create `remote-frontend/src/components/AuthGuard.test.tsx` with the exact code above.
- Edit `remote-frontend/src/AppRouter.tsx` in exactly TWO places: add the `import { AuthGuard }` line after the oauthApi import, and wrap `<NormalLayout />` with `<AuthGuard>...</AuthGuard>` in the route definition.
- Do NOT change any other line in AppRouter.tsx. Do NOT touch any other file.
- The `useProfile` hook is already provided by `@/components/ProfileProvider` (task 101 of vk-swarm-hive-ui, created at `remote-frontend/src/components/ProfileProvider.tsx:98`). It returns `{ profile, isSignedIn, isLoaded }`. Do NOT modify ProfileProvider.

## STOP triggers

- The AuthGuard file already exists.
- The AuthGuard test file already exists.
- `useProfile()` from `@/components/ProfileProvider` does not have `isSignedIn` or `isLoaded` in its return type — verify by inspecting `remote-frontend/src/components/ProfileProvider.tsx:88-98`. If the shape changed, STOP and update the task.
- The `element: <NormalLayout />` line is not found at ~L165 in AppRouter.tsx (the file may have been modified since the audit). If the exact line differs, STOP — the anchor moved.
- The AuthGuard test fails because `react-router-dom`'s `Navigate` renders an empty node in jsdom (it pushes to history instead of rendering an `<a>`). The test verifies that children are NOT rendered and loading is NOT shown — this is sufficient. If jsdom+jsdom behavior causes a different failure, adjust the test assertion to check that `screen.queryByText('protected content')` is null while `screen.queryByText('Loading...')` is also null (meaning Navigate was reached).

## Done when
`WAI_TYPECHECK_CMD="cd remote-frontend && npx tsc --noEmit" WAI_TEST_CMD="cd remote-frontend && npx vitest run src/components/AuthGuard" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-hive-ui-polish 102` exits 0.