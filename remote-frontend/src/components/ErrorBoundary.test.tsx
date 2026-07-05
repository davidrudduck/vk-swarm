import { render, screen } from '@testing-library/react';
import { describe, it, expect, vi, afterEach } from 'vitest';
import { ErrorBoundary } from './ErrorBoundary';

function ThrowingComponent(): JSX.Element {
  throw new Error('test crash');
}
function SafeComponent(): JSX.Element {
  return <div>all good</div>;
}

describe('ErrorBoundary (SC1)', () => {
  // Suppress the noisy React error event that fires when a child throws.
  // componentDidCatch still receives the error; this just silences the
  // unhandled exception in jsdom so the vitest runner can complete.
  const errorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

  afterEach(() => {
    errorSpy.mockClear();
  });

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
