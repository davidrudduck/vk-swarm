import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { I18nextProvider } from 'react-i18next';
import i18n from '@/i18n';
import { ProjectSettings } from './ProjectSettings';
import type { Project } from 'shared/types';

vi.mock('@/hooks/useProjects');
vi.mock('@/hooks/useProjectMutations');
vi.mock('@/hooks/useScriptPlaceholders', () => ({
  useScriptPlaceholders: () => ({
    setup: '',
    dev: '',
    cleanup: '',
  }),
}));
vi.mock('@/pages/settings/WebhooksSettings', () => ({
  WebhooksSection: () => <div data-testid="webhooks-section" />,
}));

import { useProjects } from '@/hooks/useProjects';
import { useProjectMutations } from '@/hooks/useProjectMutations';

const mockUseProjects = vi.mocked(useProjects);
const mockUseProjectMutations = vi.mocked(useProjectMutations);

const baseProject: Project = {
  id: 'project-1',
  name: 'Test Project',
  git_repo_path: '/repo',
  setup_script: null,
  dev_script: null,
  cleanup_script: null,
  copy_files: null,
  parallel_setup_script: false,
  auto_breakdown_enabled: false,
  remote_project_id: null,
  created_at: new Date(),
  updated_at: new Date(),
  is_remote: false,
  source_node_id: null,
  source_node_name: null,
  source_node_public_url: null,
  source_node_status: null,
  remote_last_synced_at: null,
} as unknown as Project;

const renderWithProviders = (ui: React.ReactElement, initialPath: string) =>
  render(
    <I18nextProvider i18n={i18n}>
      <MemoryRouter initialEntries={[initialPath]}>{ui}</MemoryRouter>
    </I18nextProvider>
  );

describe('ProjectSettings auto-breakdown toggle', () => {
  const updateMutate = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();

    mockUseProjects.mockReturnValue({
      data: [baseProject],
      isLoading: false,
      error: null,
    } as unknown as ReturnType<typeof useProjects>);

    mockUseProjectMutations.mockReturnValue({
      updateProject: { mutate: updateMutate },
      createProject: { mutate: vi.fn() },
      deleteProject: { mutate: vi.fn() },
    } as unknown as ReturnType<typeof useProjectMutations>);
  });

  it('renders the checkbox unchecked when auto_breakdown_enabled is false', async () => {
    renderWithProviders(
      <ProjectSettings />,
      '/settings/projects?projectId=project-1'
    );

    const checkbox = await screen.findByRole('checkbox', {
      name: /auto-breakdown new tasks/i,
    });
    expect(checkbox).not.toBeChecked();
  });

  it('toggles and saves auto_breakdown_enabled: true in the update payload', async () => {
    renderWithProviders(
      <ProjectSettings />,
      '/settings/projects?projectId=project-1'
    );

    const checkbox = await screen.findByRole('checkbox', {
      name: /auto-breakdown new tasks/i,
    });
    fireEvent.click(checkbox);
    expect(checkbox).toBeChecked();

    const saveButton = await screen.findByRole('button', { name: /save/i });
    fireEvent.click(saveButton);

    await waitFor(() => {
      expect(updateMutate).toHaveBeenCalledWith(
        expect.objectContaining({
          projectId: 'project-1',
          data: expect.objectContaining({ auto_breakdown_enabled: true }),
        })
      );
    });
  });
});
