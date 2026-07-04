import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { TooltipProvider } from '@/components/ui/tooltip';
import {
  NodeCard,
  NodeProjectsSection,
  NodeTemplatesSection,
  SwarmHealthSection,
  SwarmLabelDialog,
  SwarmLabelsSection,
  SwarmProjectDialog,
  SwarmProjectRow,
  SwarmProjectsSection,
  SwarmTemplateDialog,
  SwarmTemplatesSection,
} from './index';
import { ProfileProvider } from '@/components/ProfileProvider';

// Mock the hooks and API clients
vi.mock('@/hooks/useSwarmHealth', () => ({
  useSwarmHealth: () => ({
    issues: [],
    isLoading: false,
  }),
}));

vi.mock('@/hooks/useSwarmHealthActions', () => ({
  useSwarmHealthActions: () => ({
    fixAllIssues: vi.fn(),
    isFixing: false,
  }),
}));

vi.mock('@/hooks/useSwarmProjects', () => ({
  useSwarmProjects: () => ({
    projects: [],
    isLoading: false,
  }),
  useSwarmProjectNodes: () => ({
    nodes: [],
    isLoading: false,
  }),
  useSwarmProjectMutations: () => ({
    createProject: { mutate: vi.fn(), isPending: false },
    updateProject: { mutate: vi.fn(), isPending: false },
    deleteProject: { mutate: vi.fn(), isPending: false },
    mergeProjects: { mutate: vi.fn(), isPending: false },
    linkNode: { mutate: vi.fn(), isPending: false },
  }),
}));

vi.mock('@/hooks/useSwarmLabels', () => ({
  useSwarmLabels: () => ({
    labels: [],
    isLoading: false,
  }),
  useSwarmLabelMutations: () => ({
    createLabel: { mutate: vi.fn(), isPending: false },
    updateLabel: { mutate: vi.fn(), isPending: false },
    deleteLabel: { mutate: vi.fn(), isPending: false },
    mergeLabels: { mutate: vi.fn(), isPending: false },
  }),
}));

vi.mock('@/hooks/useSwarmTemplates', () => ({
  useSwarmTemplates: () => ({
    templates: [],
    isLoading: false,
  }),
  useSwarmTemplateMutations: () => ({
    createTemplate: { mutate: vi.fn(), isPending: false },
    updateTemplate: { mutate: vi.fn(), isPending: false },
    deleteTemplate: { mutate: vi.fn(), isPending: false },
    mergeTemplates: { mutate: vi.fn(), isPending: false },
  }),
}));

vi.mock('@/lib/api', () => ({
  nodesApi: {
    list: vi.fn(),
  },
  swarmProjectsApi: {
    list: vi.fn(),
  },
  swarmLabelsApi: {
    list: vi.fn(),
  },
  swarmTemplatesApi: {
    list: vi.fn(),
  },
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => key,
    i18n: { language: 'en' },
  }),
}));

describe('Swarm Components', () => {
  let queryClient: QueryClient;

  beforeEach(() => {
    queryClient = new QueryClient({
      defaultOptions: {
        queries: { retry: false },
        mutations: { retry: false },
      },
    });

    // Mock fetch and localStorage for ProfileProvider
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({
        user_id: 'test-user-id',
        username: 'test-user',
        email: 'test@example.com',
        providers: {},
      }),
    });

    global.localStorage = {
      getItem: vi.fn(() => 'test-token'),
      setItem: vi.fn(),
      removeItem: vi.fn(),
      clear: vi.fn(),
      key: vi.fn(),
      length: 0,
    } as any;
  });

  const renderWithProviders = (component: React.ReactElement) => {
    return render(
      <QueryClientProvider client={queryClient}>
        <ProfileProvider>
          <TooltipProvider>
            {component}
          </TooltipProvider>
        </ProfileProvider>
      </QueryClientProvider>
    );
  };

  it('renders SwarmHealthSection without throwing', () => {
    expect(() => {
      renderWithProviders(<SwarmHealthSection />);
    }).not.toThrow();
  });

  it('renders SwarmProjectsSection without throwing', () => {
    expect(() => {
      renderWithProviders(<SwarmProjectsSection organizationId="test-org" />);
    }).not.toThrow();
  });

  it('renders SwarmLabelsSection without throwing', () => {
    expect(() => {
      renderWithProviders(<SwarmLabelsSection organizationId="test-org" />);
    }).not.toThrow();
  });

  it('renders SwarmTemplatesSection without throwing', () => {
    expect(() => {
      renderWithProviders(<SwarmTemplatesSection organizationId="test-org" />);
    }).not.toThrow();
  });

  it('renders NodeProjectsSection without throwing', () => {
    expect(() => {
      renderWithProviders(<NodeProjectsSection organizationId="test-org" />);
    }).not.toThrow();
  });

  it('renders NodeTemplatesSection without throwing', () => {
    expect(() => {
      renderWithProviders(<NodeTemplatesSection organizationId="test-org" />);
    }).not.toThrow();
  });

  it('renders NodeCard without throwing', () => {
    const mockNode = {
      id: 'node-1',
      organization_id: 'org-1',
      name: 'Test Node',
      machine_id: 'machine-1',
      status: 'online' as const,
      capabilities: {
        executors: [],
        max_concurrent_tasks: 1,
        os: 'linux',
        arch: 'x64',
        version: '1.0.0',
        git_commit: 'abc123',
        git_branch: 'main',
      },
      public_url: null,
      last_heartbeat_at: null,
      connected_at: null,
      disconnected_at: null,
      created_at: '2024-01-01T00:00:00Z',
      updated_at: '2024-01-01T00:00:00Z',
    };
    expect(() => {
      renderWithProviders(<NodeCard node={mockNode} />);
    }).not.toThrow();
  });

  it('renders SwarmProjectDialog without throwing', () => {
    expect(() => {
      renderWithProviders(
        <SwarmProjectDialog
          open={true}
          onOpenChange={() => {}}
          onSave={async () => {}}
          isSaving={false}
        />
      );
    }).not.toThrow();
  });

  it('renders SwarmLabelDialog without throwing', () => {
    expect(() => {
      renderWithProviders(
        <SwarmLabelDialog
          open={true}
          onOpenChange={() => {}}
          onSave={async () => {}}
          isSaving={false}
        />
      );
    }).not.toThrow();
  });

  it('renders SwarmTemplateDialog without throwing', () => {
    expect(() => {
      renderWithProviders(
        <SwarmTemplateDialog
          open={true}
          onOpenChange={() => {}}
          onSave={async () => {}}
          isSaving={false}
        />
      );
    }).not.toThrow();
  });

  it('renders SwarmProjectRow without throwing', () => {
    const mockProject = {
      id: 'project-1',
      organization_id: 'org-1',
      name: 'Test Project',
      description: null,
      metadata: {},
      created_at: '2024-01-01T00:00:00Z',
      updated_at: '2024-01-01T00:00:00Z',
      linked_nodes_count: 0,
      linked_node_names: [],
      hive_project_ids: [],
      task_counts: {
        todo: 0,
        in_progress: 0,
        in_review: 0,
        done: 0,
        cancelled: 0,
      },
    };
    expect(() => {
      renderWithProviders(
        <SwarmProjectRow
          project={mockProject}
          nodes={[]}
          isLoadingNodes={false}
          isExpanded={false}
          onToggleExpand={() => {}}
          onEdit={() => {}}
          onDelete={() => {}}
          onUnlinkNode={() => {}}
        />
      );
    }).not.toThrow();
  });
});
