import { describe, expect, it, vi } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { MemoryRouter } from 'react-router-dom';
import type { ProjectWithStats } from 'shared/types';
import { ProjectList } from './ProjectList';

// jsdom does not implement scrollIntoView; UnifiedProjectCard calls it on focus.
Element.prototype.scrollIntoView = vi.fn();

vi.mock('react-i18next', () => ({
  initReactI18next: { type: '3rdParty', init: () => {} },
  useTranslation: () => ({
    t: (key: string) => key,
  }),
}));

vi.mock('@/lib/api', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/lib/api')>();
  return {
    ...actual,
    projectsApi: {
      ...actual.projectsApi,
      getWithStats: vi.fn(),
    },
  };
});

const fixture: ProjectWithStats[] = [
  {
    id: '11111111-1111-1111-1111-111111111111',
    name: 'zeta',
    git_repo_path: '/tmp/zeta',
    created_at: new Date('2026-01-01T00:00:00Z'),
    remote_project_id: null,
    last_attempt_at: new Date('2026-02-01T00:00:00Z'),
    github_enabled: false,
    github_owner: null,
    github_repo: null,
    github_open_issues: 0,
    github_open_prs: 0,
    github_last_synced_at: null,
    task_counts: { todo: 3, in_progress: 1, in_review: 0, done: 2 },
  } as unknown as ProjectWithStats,
];

describe('ProjectList on ProjectWithStats', () => {
  it('renders a project and its task counts from /api/projects/with-stats', async () => {
    const { projectsApi } = await import('@/lib/api');
    vi.mocked(projectsApi.getWithStats).mockResolvedValue({
      projects: fixture,
    });

    const client = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });
    render(
      <MemoryRouter>
        <QueryClientProvider client={client}>
          <ProjectList />
        </QueryClientProvider>
      </MemoryRouter>
    );

    await waitFor(() => expect(screen.getByText('zeta')).toBeInTheDocument());
    // The enrichment must survive the type change — this is the a85f7d63 regression guard
    expect(screen.getByText('3')).toBeInTheDocument();
  });
});
