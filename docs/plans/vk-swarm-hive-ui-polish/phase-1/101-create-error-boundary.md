---
id: "101"
phase: 1
title: Create ErrorBoundary component
status: ready
depends_on: []
parallel: false
conflicts_with: []
files:
  - remote-frontend/src/components/ErrorBoundary.tsx
  - remote-frontend/src/components/ErrorBoundary.test.tsx
irreversible: false
scope_test: "remote-frontend/src/components"
allowed_change: create
covers_criteria: [SC1]
---

## Failing test (write first)

Create `remote-frontend/src/components/ErrorBoundary.test.tsx`:

```tsx
import { render, screen } from '@testing-library/react';
import { describe, it, expect } from 'vitest';
import { ErrorBoundary } from './ErrorBoundary';

function ThrowingComponent() {
  throw new Error('test crash');
}
function SafeComponent() {
  return <div>all good</div>;
}

describe('ErrorBoundary (SC1)', () => {
  it('renders children when no error', () => {
    render(
      <ErrorBoundary>
        <SafeComponent />
      </ErrorBoundary>,
    );
    expect(screen.getByText('all good')).toBeDefined();
  });

  it('renders fallback UI when child throws', () => {
    render(
      <ErrorBoundary>
        <ThrowingComponent />
      </ErrorBoundary>,
    );
    expect(screen.getByText('Something went wrong')).toBeDefined();
  });

  it('renders a Reload button in the fallback', () => {
    render(
      <ErrorBoundary>
        <ThrowingComponent />
      </ErrorBoundary>,
    );
    const button = screen.getByRole('button', { name: 'Reload' });
    expect(button).toBeDefined();
  });
});
```

## Change

### File: `remote-frontend/src/components/ErrorBoundary.tsx` (CREATE)

```tsx
import { Component, type ReactNode } from 'react';

interface ErrorBoundaryProps {
  children: ReactNode;
  fallback?: ReactNode;
}

interface ErrorBoundaryState {
  hasError: boolean;
}

export class ErrorBoundary extends Component<
  ErrorBoundaryProps,
  ErrorBoundaryState
> {
  constructor(props: ErrorBoundaryProps) {
    super(props);
    this.state = { hasError: false };
  }

  static getDerivedStateFromError(): ErrorBoundaryState {
    return { hasError: true };
  }

  componentDidCatch(error: Error, info: { componentStack: string }) {
    console.error('ErrorBoundary caught:', error, info.componentStack);
  }

  render() {
    if (this.state.hasError) {
      return (
        this.props.fallback ?? (
          <div className="min-h-screen flex items-center justify-center bg-[#0a0a0f] p-4">
            <div className="text-center space-y-4">
              <p className="text-lg text-[#d1d5db]">Something went wrong</p>
              <button
                onClick={() => window.location.reload()}
                className="px-4 py-2 bg-[#0091b5] text-white rounded-lg hover:bg-[#007a99] transition-colors"
              >
                Reload
              </button>
            </div>
          </div>
        )
      );
    }

    return this.props.children;
  }
}
```

## Allowed moves

- Create `remote-frontend/src/components/ErrorBoundary.tsx` with the exact code above.
- Create `remote-frontend/src/components/ErrorBoundary.test.tsx` with the exact code above.
- Do NOT touch any other file. The ErrorBoundary is NOT yet mounted in `main.tsx` — that's task 104.

## STOP triggers

- The component file already exists (a stale artifact from a prior aborted run — delete it or rename before proceeding).
- The test file already exists (same).
- TypeScript compile fails on the class component (React class components are fully supported; if `tsc` reports errors, check that `@types/react` is installed — it is, v18.2.17 in package.json).

## Done when
`WAI_TYPECHECK_CMD="cd remote-frontend && npx tsc --noEmit" WAI_TEST_CMD="cd remote-frontend && npx vitest run src/components/ErrorBoundary" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-hive-ui-polish 101` exits 0.