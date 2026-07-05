---
id: "104"
phase: 1
title: Mount ErrorBoundary + Toaster + AuthGuard in root files
status: ready
depends_on: ["100", "101", "102"]
parallel: false
conflicts_with: []
files:
  - remote-frontend/src/main.tsx
  - remote-frontend/src/App.tsx
  - remote-frontend/src/AppRouter.tsx
irreversible: false
scope_test: "remote-frontend/src"
allowed_change: edit
covers_criteria: [SC1, SC2]
---

## Failing test (write first)

The existing test `remote-frontend/src/App.test.tsx` is the gate. It verifies that `App` renders without crashing. After this change, `App` imports and renders `Toaster` — the existing test should still pass. If it fails because the test environment doesn't have `sonner` (ESM-only module resolving), update the test to mock sonner.

**Updated `remote-frontend/src/App.test.tsx`:**

Before:
```tsx
import { render } from '@testing-library/react';
import { describe, it, expect } from 'vitest';
import App from './App';

describe('App', () => {
  it('renders without crashing', () => {
    const { container } = render(<App />);
    expect(container).toBeDefined();
  });
});
```

After:
```tsx
import { render } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import App from './App';

vi.mock('sonner', () => ({
  Toaster: () => null,
  toast: {
    error: vi.fn(),
    success: vi.fn(),
  },
}));

describe('App', () => {
  it('renders without crashing', () => {
    const { container } = render(<App />);
    expect(container).toBeDefined();
  });
});
```

## Change

### File: `remote-frontend/src/main.tsx` (EDIT — wrap App with ErrorBoundary)

**Anchor:** the `ReactDOM.createRoot(...)` block (~L5-L9)

Before:
```tsx
import React from 'react'
import ReactDOM from 'react-dom/client'
import App from './App.tsx'
import './index.css'

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
)
```

After:
```tsx
import React from 'react'
import ReactDOM from 'react-dom/client'
import App from './App.tsx'
import './index.css'
import { ErrorBoundary } from '@/components/ErrorBoundary'

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <ErrorBoundary>
      <App />
    </ErrorBoundary>
  </React.StrictMode>,
)
```

### File: `remote-frontend/src/App.tsx` (EDIT — add Toaster)

**Anchor:** the `return` block (~L11-L16)

Before:
```tsx
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { ProfileProvider } from '@/components/ProfileProvider'
import AppRouter from './AppRouter'

const queryClient = new QueryClient({
  defaultOptions: {
    queries: { staleTime: 30_000 },
  },
})

export default function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <ProfileProvider>
        <AppRouter />
      </ProfileProvider>
    </QueryClientProvider>
  )
}
```

After:
```tsx
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { ProfileProvider } from '@/components/ProfileProvider'
import AppRouter from './AppRouter'
import { Toaster } from 'sonner'

const queryClient = new QueryClient({
  defaultOptions: {
    queries: { staleTime: 30_000 },
  },
})

export default function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <ProfileProvider>
        <Toaster richColors position="bottom-right" />
        <AppRouter />
      </ProfileProvider>
    </QueryClientProvider>
  )
}
```

### File: `remote-frontend/src/AppRouter.tsx` (EDIT — wrap lazy routes with ErrorBoundary)

**Anchor 1 — imports (~L1-L10):**

Add this import to the existing imports block:
```tsx
import { ErrorBoundary } from '@/components/ErrorBoundary'
```

**Anchor 2 — the Nodes route element (~L170):**

Before:
```tsx
        { path: '/nodes', element: <Suspense fallback={<div className="p-8">Loading nodes...</div>}><Nodes /></Suspense> },
```

After:
```tsx
        { path: '/nodes', element: <ErrorBoundary><Suspense fallback={<div className="p-8">Loading nodes...</div>}><Nodes /></Suspense></ErrorBoundary> },
```

**Anchor 3 — the Tasks route element:**

Before:
```tsx
        { path: '/tasks', element: <Suspense fallback={<div className="p-8">Loading tasks...</div>}><TasksBoard /></Suspense> },
```

After:
```tsx
        { path: '/tasks', element: <ErrorBoundary><Suspense fallback={<div className="p-8">Loading tasks...</div>}><TasksBoard /></Suspense></ErrorBoundary> },
```

## Allowed moves

- Edit `remote-frontend/src/main.tsx`: add `import { ErrorBoundary }` line after the existing imports, wrap `<App />` with `<ErrorBoundary>...</ErrorBoundary>`.
- Edit `remote-frontend/src/App.tsx`: add `import { Toaster } from 'sonner'` after the existing imports, add `<Toaster richColors position="bottom-right" />` inside `<ProfileProvider>` before `<AppRouter />`.
- Update `remote-frontend/src/App.test.tsx` with the sonner mock as shown above.
- Also edit `remote-frontend/src/AppRouter.tsx`: add `import { ErrorBoundary } from '@/components/ErrorBoundary'` after the existing imports, and wrap the two lazy-loaded route elements (Nodes at `/nodes`, TasksBoard at `/tasks`) with `<ErrorBoundary>...</ErrorBoundary>`. Use the exact Before/After below.

## STOP triggers

- `ErrorBoundary` is not found at `@/components/ErrorBoundary` (task 101 must be completed first — verify with `ls remote-frontend/src/components/ErrorBoundary.tsx`).
- `sonner` is not installed (task 100 must be completed first — verify with `ls remote-frontend/node_modules/sonner/package.json`).
- The `App.test.tsx` fails even with the sonner mock — the test environment may have vitest `deps.interopDefault` issues with sonner's ESM entry. If so, add `deps: { optimizer: { ssr: { include: ['sonner'] } } }` to vite.config.ts's `test` config. Record this undictated choice in the decisions ledger.
- Any import line in the Before text differs from the actual file. The implementer MUST compare with `git diff` before making the edit.

## Done when
`WAI_TYPECHECK_CMD="cd remote-frontend && npx tsc --noEmit" WAI_TEST_CMD="cd remote-frontend && npx vitest run" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-hive-ui-polish 104` exits 0.