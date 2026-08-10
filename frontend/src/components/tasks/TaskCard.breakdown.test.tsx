import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import type { TaskWithAttemptStatus, TaskStatus } from 'shared/types';

import enTasks from '@/i18n/locales/en/tasks.json';
import jaTasks from '@/i18n/locales/ja/tasks.json';
import koTasks from '@/i18n/locales/ko/tasks.json';
import esTasks from '@/i18n/locales/es/tasks.json';

// Mock react-i18next before other imports (mirrors TaskCard.swipe.test.tsx)
vi.mock('react-i18next', async () => {
  const original = await vi.importActual('react-i18next');
  return {
    ...original,
    initReactI18next: { type: '3rdParty', init: () => {} },
    useTranslation: () => ({
      t: (key: string, fallback?: string) => fallback ?? key,
      i18n: {
        changeLanguage: () => Promise.resolve(),
        language: 'en',
      },
    }),
    Trans: ({ children }: { children: React.ReactNode }) => children,
  };
});

vi.mock('@/i18n/config', () => ({
  default: {},
}));

// Mock the aggregate hooks barrel used by both TaskCard and ActionsDropdown
const mockUseIsMobile = vi.fn(() => false);
const mockUseTaskUsesSharedWorktree = vi.fn(() => ({
  usesSharedWorktree: false,
}));
vi.mock('@/hooks', () => ({
  useAuth: () => ({ userId: 'user-1' }),
  useIsOrgAdmin: () => false,
  useNavigateWithSearch: () => vi.fn(),
  useTaskUsesSharedWorktree: () => mockUseTaskUsesSharedWorktree(),
  useIsMobile: () => mockUseIsMobile(),
}));

vi.mock('@/contexts/TaskOptimisticContext', () => ({
  useTaskOptimistic: () => null,
  getArchivedCallback: () => undefined,
  getStatusCallback: () => undefined,
}));

vi.mock('@/contexts/ProjectContext', () => ({
  useProject: () => ({ project: null, projectId: 'project-1' }),
}));

vi.mock('@/hooks/useTaskLabels', () => ({
  useTaskLabels: () => ({ data: [] }),
}));

vi.mock('@/hooks/useOpenInEditor', () => ({
  useOpenInEditor: () => vi.fn(),
}));

vi.mock('@/hooks/useAttemptCleanupMutations', () => ({
  useAttemptCleanupMutations: () => ({
    cleanupWorktree: { mutateAsync: vi.fn(), isPending: false },
    purgeArtifacts: { mutate: vi.fn(), isPending: false },
  }),
}));

vi.mock('@/lib/api', () => ({
  tasksApi: {
    archive: vi.fn().mockResolvedValue({}),
    unarchive: vi.fn().mockResolvedValue({}),
    assign: vi.fn().mockResolvedValue({}),
    update: vi.fn().mockResolvedValue({}),
  },
}));

vi.mock('@/lib/openTaskForm', () => ({
  openTaskForm: vi.fn(),
}));

// Simplify KanbanCard so TaskCard can render outside a DnD context
vi.mock('@/components/ui/shadcn-io/kanban', () => ({
  KanbanCard: ({
    children,
    className,
  }: {
    children: React.ReactNode;
    className?: string;
  }) => <div className={className}>{children}</div>,
}));

// Draft breakdown proposal state — mutable so tests can toggle it
const { proposalState, triggerMutate } = vi.hoisted(() => ({
  proposalState: {
    proposal: null as { id: string; status: string } | null,
  },
  triggerMutate: vi.fn().mockResolvedValue({ id: 'proposal-1' }),
}));

vi.mock('@/hooks/useBreakdown', () => ({
  useBreakdownProposal: () => ({
    proposal: proposalState.proposal,
    items: [],
    isLoading: false,
    error: null,
  }),
  useBreakdownMutations: () => ({
    trigger: { mutateAsync: triggerMutate, isPending: false },
    putItems: { mutate: vi.fn(), isPending: false },
    discard: { mutate: vi.fn(), isPending: false },
    retry: { mutate: vi.fn(), isPending: false },
    accept: { mutate: vi.fn(), isPending: false },
  }),
}));

const mockBreakdownDialogShow = vi.fn();
vi.mock('@/components/dialogs/tasks/BreakdownReviewDialog', () => ({
  BreakdownReviewDialog: {
    show: (props: unknown) => mockBreakdownDialogShow(props),
  },
}));

// Other dialogs referenced by ActionsDropdown — not under test here
vi.mock('@/components/dialogs/tasks/ArchiveTaskConfirmationDialog', () => ({
  ArchiveTaskConfirmationDialog: { show: vi.fn() },
}));
vi.mock('@/components/dialogs/tasks/CleanupWorktreeConfirmationDialog', () => ({
  CleanupWorktreeConfirmationDialog: { show: vi.fn() },
}));
vi.mock('@/components/dialogs/tasks/DeleteTaskConfirmationDialog', () => ({
  DeleteTaskConfirmationDialog: { show: vi.fn() },
}));
vi.mock('@/components/dialogs/tasks/ViewProcessesDialog', () => ({
  ViewProcessesDialog: { show: vi.fn() },
}));
vi.mock('@/components/dialogs/tasks/ViewRelatedTasksDialog', () => ({
  ViewRelatedTasksDialog: { show: vi.fn() },
}));
vi.mock('@/components/dialogs/tasks/CreateAttemptDialog', () => ({
  CreateAttemptDialog: { show: vi.fn() },
}));
vi.mock('@/components/dialogs/tasks/GitActionsDialog', () => ({
  GitActionsDialog: { show: vi.fn() },
}));
vi.mock('@/components/dialogs/tasks/EditBranchNameDialog', () => ({
  EditBranchNameDialog: { show: vi.fn() },
}));

// Import after mocks
import { TaskCard } from './TaskCard';

function renderTaskCard(ui: React.ReactElement) {
  return render(<MemoryRouter>{ui}</MemoryRouter>);
}

function createMockTask(
  overrides: Partial<TaskWithAttemptStatus> = {}
): TaskWithAttemptStatus {
  return {
    id: 'task-1',
    title: 'Test Task',
    description: null,
    status: 'todo' as TaskStatus,
    project_id: 'project-1',
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
    archived_at: null,
    parent_task_id: null,
    shared_task_id: null,
    executor: 'CLAUDE_CODE',
    has_in_progress_attempt: false,
    has_merged_attempt: false,
    last_attempt_failed: false,
    remote_assignee_user_id: null,
    remote_assignee_name: null,
    remote_assignee_username: null,
    remote_version: BigInt(0),
    remote_last_synced_at: null,
    remote_stream_node_id: null,
    remote_stream_url: null,
    activity_at: null,
    latest_execution_started_at: null,
    latest_execution_completed_at: null,
    source_node_name: null,
    ...overrides,
  } as TaskWithAttemptStatus;
}

describe('TaskCard - breakdown action & badge', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    proposalState.proposal = null;
    mockUseIsMobile.mockReturnValue(false);
    mockUseTaskUsesSharedWorktree.mockReturnValue({
      usesSharedWorktree: false,
    });
  });

  const defaultProps = {
    index: 0,
    status: 'todo',
    onViewDetails: vi.fn(),
    isOpen: false,
    projectId: 'project-1',
  };

  it('shows the proposed-subtasks badge when a draft proposal exists', () => {
    proposalState.proposal = { id: 'proposal-1', status: 'draft' };
    const task = createMockTask();

    renderTaskCard(<TaskCard {...defaultProps} task={task} />);

    expect(screen.getByText('Proposed subtasks')).toBeInTheDocument();
  });

  it('does not show the badge when there is no draft proposal', () => {
    proposalState.proposal = null;
    const task = createMockTask();

    renderTaskCard(<TaskCard {...defaultProps} task={task} />);

    expect(screen.queryByText('Proposed subtasks')).not.toBeInTheDocument();
  });

  it('does not show the badge when the latest proposal is accepted', () => {
    proposalState.proposal = { id: 'proposal-1', status: 'accepted' };
    const task = createMockTask();

    renderTaskCard(<TaskCard {...defaultProps} task={task} />);

    expect(screen.queryByText('Proposed subtasks')).not.toBeInTheDocument();
  });

  it('does not show the badge when the latest proposal is discarded', () => {
    proposalState.proposal = { id: 'proposal-1', status: 'discarded' };
    const task = createMockTask();

    renderTaskCard(<TaskCard {...defaultProps} task={task} />);

    expect(screen.queryByText('Proposed subtasks')).not.toBeInTheDocument();
  });

  it('desktop dropdown contains the Break down item and opens the dialog on click (existing draft)', async () => {
    mockUseIsMobile.mockReturnValue(false);
    proposalState.proposal = { id: 'proposal-1', status: 'draft' };
    const task = createMockTask();

    renderTaskCard(<TaskCard {...defaultProps} task={task} />);

    const trigger = screen.getByRole('button', { name: 'Actions' });
    fireEvent.pointerDown(trigger, { button: 0, ctrlKey: false });
    fireEvent.click(trigger);

    const breakdownItem = await screen.findByText('Break down');
    fireEvent.click(breakdownItem);

    // A draft already exists, so trigger is skipped and the dialog opens directly.
    expect(triggerMutate).not.toHaveBeenCalled();
    expect(mockBreakdownDialogShow).toHaveBeenCalledWith({
      taskId: task.id,
      projectId: 'project-1',
    });
  });

  it('desktop dropdown shows the dialog without triggering when latest proposal failed', async () => {
    mockUseIsMobile.mockReturnValue(false);
    proposalState.proposal = { id: 'proposal-1', status: 'failed' };
    const task = createMockTask();

    renderTaskCard(<TaskCard {...defaultProps} task={task} />);

    const trigger = screen.getByRole('button', { name: 'Actions' });
    fireEvent.pointerDown(trigger, { button: 0, ctrlKey: false });
    fireEvent.click(trigger);

    const breakdownItem = await screen.findByText('Break down');
    fireEvent.click(breakdownItem);

    // Failed proposal opens directly — the dialog offers Retry.
    expect(triggerMutate).not.toHaveBeenCalled();
    expect(mockBreakdownDialogShow).toHaveBeenCalledWith({
      taskId: task.id,
      projectId: 'project-1',
    });
  });

  it('desktop dropdown re-triggers generation when the latest proposal is discarded', async () => {
    mockUseIsMobile.mockReturnValue(false);
    proposalState.proposal = { id: 'proposal-1', status: 'discarded' };
    const task = createMockTask();

    renderTaskCard(<TaskCard {...defaultProps} task={task} />);

    const trigger = screen.getByRole('button', { name: 'Actions' });
    fireEvent.pointerDown(trigger, { button: 0, ctrlKey: false });
    fireEvent.click(trigger);

    const breakdownItem = await screen.findByText('Break down');
    fireEvent.click(breakdownItem);

    // Terminal (discarded) proposal re-triggers a fresh draft before opening.
    expect(triggerMutate).toHaveBeenCalled();
    await Promise.resolve();
    expect(mockBreakdownDialogShow).toHaveBeenCalledWith({
      taskId: task.id,
      projectId: 'project-1',
    });
  });

  it('desktop dropdown re-triggers generation when the latest proposal is accepted', async () => {
    mockUseIsMobile.mockReturnValue(false);
    proposalState.proposal = { id: 'proposal-1', status: 'accepted' };
    const task = createMockTask();

    renderTaskCard(<TaskCard {...defaultProps} task={task} />);

    const trigger = screen.getByRole('button', { name: 'Actions' });
    fireEvent.pointerDown(trigger, { button: 0, ctrlKey: false });
    fireEvent.click(trigger);

    const breakdownItem = await screen.findByText('Break down');
    fireEvent.click(breakdownItem);

    // Terminal (accepted) proposal re-triggers a fresh draft before opening.
    expect(triggerMutate).toHaveBeenCalled();
    await Promise.resolve();
    expect(mockBreakdownDialogShow).toHaveBeenCalledWith({
      taskId: task.id,
      projectId: 'project-1',
    });
  });

  it('desktop dropdown triggers generation before opening when no draft exists', async () => {
    mockUseIsMobile.mockReturnValue(false);
    proposalState.proposal = null;
    const task = createMockTask();

    renderTaskCard(<TaskCard {...defaultProps} task={task} />);

    const trigger = screen.getByRole('button', { name: 'Actions' });
    fireEvent.pointerDown(trigger, { button: 0, ctrlKey: false });
    fireEvent.click(trigger);

    const breakdownItem = await screen.findByText('Break down');
    fireEvent.click(breakdownItem);

    expect(triggerMutate).toHaveBeenCalled();
    await Promise.resolve();
    expect(mockBreakdownDialogShow).toHaveBeenCalledWith({
      taskId: task.id,
      projectId: 'project-1',
    });
  });

  it('mobile bottom-sheet branch also renders the Break down action', async () => {
    mockUseIsMobile.mockReturnValue(true);
    proposalState.proposal = { id: 'proposal-1', status: 'draft' };
    const task = createMockTask();

    renderTaskCard(<TaskCard {...defaultProps} task={task} />);

    // Mobile: tapping the trigger button opens the bottom sheet
    const trigger = screen.getByRole('button', { name: 'Actions' });
    fireEvent.pointerDown(trigger, { button: 0, ctrlKey: false });
    fireEvent.click(trigger);

    const breakdownItem = await screen.findByText('Break down');
    fireEvent.click(breakdownItem);

    expect(mockBreakdownDialogShow).toHaveBeenCalledWith({
      taskId: task.id,
      projectId: 'project-1',
    });
  });

  it('desktop dropdown disables Break down for shared-worktree tasks', async () => {
    mockUseIsMobile.mockReturnValue(false);
    mockUseTaskUsesSharedWorktree.mockReturnValue({
      usesSharedWorktree: true,
    });
    proposalState.proposal = null;
    const task = createMockTask();

    renderTaskCard(<TaskCard {...defaultProps} task={task} />);

    const trigger = screen.getByRole('button', { name: 'Actions' });
    fireEvent.pointerDown(trigger, { button: 0, ctrlKey: false });
    fireEvent.click(trigger);

    const breakdownItem = await screen.findByText('Break down');
    const breakdownMenuItem = breakdownItem.closest(
      '[role="menuitem"]'
    ) as HTMLElement;

    // Radix marks disabled menu items via aria/data attributes rather than
    // the native `disabled` attribute. Note: jsdom's fireEvent.click does
    // not honor pointer-events:none, so we assert on these attributes
    // rather than on click-handler suppression (see decisions-ledger F3
    // for the documented coverage boundary).
    expect(breakdownMenuItem).toHaveAttribute('data-disabled');
    expect(breakdownMenuItem).toHaveAttribute('aria-disabled', 'true');
  });
});

describe('breakdown i18n key parity (en/ja/ko/es)', () => {
  const EXPECTED_KEYS = [
    'action',
    'proposedBadge',
    'title',
    'failedGeneric',
    'retry',
    'running',
    'itemTitle',
    'moveUp',
    'moveDown',
    'deleteItem',
    'itemDescription',
    'dependencies',
    'discard',
    'accept',
  ];

  it.each([
    ['en', enTasks],
    ['ja', jaTasks],
    ['ko', koTasks],
    ['es', esTasks],
  ])('%s tasks.json has the full breakdown.* key set', (_locale, dict) => {
    const breakdown = (dict as Record<string, unknown>).breakdown as
      | Record<string, unknown>
      | undefined;
    expect(breakdown).toBeDefined();
    for (const key of EXPECTED_KEYS) {
      expect(breakdown).toHaveProperty(key);
      expect(typeof breakdown![key]).toBe('string');
    }
  });
});
