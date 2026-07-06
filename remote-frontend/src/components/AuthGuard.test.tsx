import { render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter, Routes, Route } from 'react-router-dom';
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

  it('redirects to /login when signed out', async () => {
    mockedUseProfile.mockReturnValue({ isSignedIn: false, isLoaded: true });
    render(
      <MemoryRouter initialEntries={['/nodes']}>
        <Routes>
          <Route path="/nodes" element={<AuthGuard><div>protected content</div></AuthGuard>} />
          <Route path="/login" element={<div>login page</div>} />
        </Routes>
      </MemoryRouter>,
    );
    await waitFor(() => {
      expect(screen.getByText('login page')).toBeDefined();
    });
  });
});
