import { beforeEach, describe, expect, afterEach, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { Navbar } from '../Navbar';
import { SwarmSettings } from '@/pages/settings/SwarmSettings';

const { browserAuthApi } = vi.hoisted(() => ({
  browserAuthApi: {
    logout: vi.fn(),
    disconnectHive: vi.fn(),
  },
}));

vi.mock('@/lib/api', () => ({ browserAuthApi }));

vi.mock('@/components/ConfigProvider', () => ({
  useUserSystem: vi.fn(() => ({
    loginStatus: { status: 'loggedin' },
    reloadSystem: vi.fn(),
  })),
}));

vi.mock('@/contexts/ProjectContext', () => ({
  useProject: vi.fn(() => ({
    projectId: 'p1',
    project: { id: 'p1', name: 'P' },
  })),
}));

vi.mock('@/contexts/SearchContext', () => ({
  useSearch: vi.fn(() => ({
    query: '',
    setQuery: vi.fn(),
    active: false,
    clear: vi.fn(),
    registerInputRef: vi.fn(),
  })),
}));

vi.mock('@/hooks/useOpenProjectInEditor', () => ({
  useOpenProjectInEditor: vi.fn(() => vi.fn()),
}));

vi.mock('@/components/SearchBar', () => ({ SearchBar: vi.fn(() => null) }));
vi.mock('@/components/activity', () => ({ ActivityFeed: vi.fn(() => null) }));
vi.mock('../ProjectSwitcher', () => ({ ProjectSwitcher: vi.fn(() => null) }));
vi.mock('@/components/ThemeToggle', () => ({ default: vi.fn(() => null) }));
vi.mock('@/components/ide/OpenInIdeButton', () => ({
  OpenInIdeButton: vi.fn(() => null),
}));
vi.mock('@/components/dialogs/global/OAuthDialog', () => ({
  OAuthDialog: { show: vi.fn() },
}));
vi.mock('@/components/VKSLogo', () => ({ VKSLogo: vi.fn(() => null) }));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, defaultValue?: string) => defaultValue ?? key,
  }),
}));

vi.mock('@/hooks/auth/useAuth', () => ({
  useAuth: vi.fn(() => ({ isSignedIn: true, isLoaded: true })),
}));

vi.mock('@/hooks/useUserOrganizations', () => ({
  useUserOrganizations: vi.fn(() => ({
    data: { organizations: [{ id: 'o1', name: 'Org' }] },
    isLoading: false,
    error: null,
  })),
}));

vi.mock('@/hooks/useOrganizationSelection', () => ({
  useOrganizationSelection: vi.fn(() => ({
    selectedOrgId: 'o1',
    selectedOrg: { id: 'o1', name: 'Org' },
    handleOrgSelect: vi.fn(),
  })),
}));

vi.mock('@/components/swarm', () => ({
  SwarmProjectsSection: vi.fn(() => null),
  NodeProjectsSection: vi.fn(() => null),
  SwarmLabelsSection: vi.fn(() => null),
  SwarmTemplatesSection: vi.fn(() => null),
  NodeTemplatesSection: vi.fn(() => null),
}));

vi.mock('@/components/dialogs/shared/LoginRequiredPrompt', () => ({
  LoginRequiredPrompt: vi.fn(() => null),
}));

describe('browser auth actions', () => {
  const reloadSpy = vi.fn();
  const originalLocationDescriptor = Object.getOwnPropertyDescriptor(
    window,
    'location'
  );

  beforeEach(() => {
    vi.clearAllMocks();
    browserAuthApi.logout.mockResolvedValue(undefined);
    browserAuthApi.disconnectHive.mockResolvedValue(undefined);
    Object.defineProperty(window, 'location', {
      configurable: true,
      value: { ...window.location, reload: reloadSpy },
    });
  });

  afterEach(() => {
    if (originalLocationDescriptor) {
      Object.defineProperty(window, 'location', originalLocationDescriptor);
    }
  });

  it('signs out this browser without disconnecting the Hive', async () => {
    render(
      <MemoryRouter>
        <Navbar />
      </MemoryRouter>
    );

    const menuButton = screen.getByRole('button', { name: 'Main navigation' });
    fireEvent.pointerDown(menuButton);
    fireEvent.click(menuButton);
    fireEvent.click(await screen.findByTestId('navbar-sign-out'));

    await waitFor(() => expect(reloadSpy).toHaveBeenCalledTimes(1));
    expect(browserAuthApi.logout).toHaveBeenCalledTimes(1);
    expect(browserAuthApi.disconnectHive).toHaveBeenCalledTimes(0);
  });

  it('does not reload when browser sign-out fails', async () => {
    browserAuthApi.logout.mockRejectedValue(new Error('logout failed'));
    render(
      <MemoryRouter>
        <Navbar />
      </MemoryRouter>
    );

    const menuButton = screen.getByRole('button', { name: 'Main navigation' });
    fireEvent.pointerDown(menuButton);
    fireEvent.click(menuButton);
    fireEvent.click(await screen.findByTestId('navbar-sign-out'));

    await waitFor(() => expect(browserAuthApi.logout).toHaveBeenCalledTimes(1));
    expect(reloadSpy).toHaveBeenCalledTimes(0);
  });

  it('confirms before disconnecting the Hive', async () => {
    vi.spyOn(window, 'confirm').mockReturnValue(true);
    render(<SwarmSettings />);

    fireEvent.click(screen.getByTestId('hive-disconnect'));

    await waitFor(() => expect(reloadSpy).toHaveBeenCalledTimes(1));
    expect(window.confirm).toHaveBeenCalledWith(
      expect.stringContaining('EVERY browser')
    );
    expect(browserAuthApi.disconnectHive).toHaveBeenCalledTimes(1);
    expect(browserAuthApi.logout).toHaveBeenCalledTimes(0);
  });

  it('does not disconnect when confirmation is cancelled', async () => {
    vi.spyOn(window, 'confirm').mockReturnValue(false);
    render(<SwarmSettings />);

    fireEvent.click(screen.getByTestId('hive-disconnect'));

    await waitFor(() => expect(window.confirm).toHaveBeenCalledTimes(1));
    expect(browserAuthApi.disconnectHive).toHaveBeenCalledTimes(0);
    expect(reloadSpy).toHaveBeenCalledTimes(0);
  });
});
