import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import React from 'react';
import { I18nextProvider } from 'react-i18next';
import i18n from '@/i18n';
import type {
  TaskBreakdownProposal,
  TaskBreakdownProposalItem,
} from 'shared/types';

// Mock NiceModal — mirrors the OAuthDialog.test.tsx precedent (vi.hoisted spies,
// never close a factory over a hoisted import).
const { mockRemove, modalState } = vi.hoisted(() => ({
  mockRemove: vi.fn(),
  modalState: { visible: true },
}));
vi.mock('@ebay/nice-modal-react', () => ({
  useModal: () => ({ visible: modalState.visible, remove: mockRemove }),
  create: (Component: React.ComponentType) => Component,
  default: {
    create: (Component: React.ComponentType) => Component,
  },
}));

// Mock defineModal (TaskFormSheet / OAuthDialog precedent)
vi.mock('@/lib/modals', () => ({
  defineModal: (Component: React.ComponentType) => Component,
}));

// Mock useBreakdown hooks (501's hooks — used, not modified)
const {
  proposalState,
  putItemsMutate,
  discardMutate,
  retryMutate,
  acceptMutate,
  capturedOptions,
} = vi.hoisted(() => ({
  proposalState: {
    proposal: null as TaskBreakdownProposal | null,
    items: [] as TaskBreakdownProposalItem[],
  },
  putItemsMutate: vi.fn(),
  discardMutate: vi.fn(),
  retryMutate: vi.fn(),
  acceptMutate: vi.fn(),
  capturedOptions: {
    current: undefined as
      | {
          onAcceptSuccess?: () => void;
          onDiscardSuccess?: () => void;
        }
      | undefined,
  },
}));

vi.mock('@/hooks/useBreakdown', () => ({
  useBreakdownProposal: () => ({
    proposal: proposalState.proposal,
    items: proposalState.items,
    isLoading: false,
    error: null,
  }),
  useBreakdownMutations: (
    _taskId: string,
    _projectId: string,
    options: { onAcceptSuccess?: () => void; onDiscardSuccess?: () => void }
  ) => {
    capturedOptions.current = options;
    return {
      putItems: { mutate: putItemsMutate, isPending: false },
      discard: {
        mutate: (proposalId: string) => {
          discardMutate(proposalId);
          options.onDiscardSuccess?.();
        },
        isPending: false,
      },
      retry: { mutate: retryMutate, isPending: false },
      accept: {
        mutate: (proposalId: string) => {
          acceptMutate(proposalId);
          options.onAcceptSuccess?.();
        },
        isPending: false,
      },
    };
  },
}));

import { BreakdownReviewDialog } from './BreakdownReviewDialog';

function renderDialog() {
  return render(
    <I18nextProvider i18n={i18n}>
      <BreakdownReviewDialog taskId="task-1" projectId="proj-1" />
    </I18nextProvider>
  );
}

function makeProposal(
  overrides: Partial<TaskBreakdownProposal> = {}
): TaskBreakdownProposal {
  return {
    id: 'proposal-1',
    task_id: 'task-1',
    status: 'draft',
    execution_process_id: null,
    error: null,
    created_at: new Date(),
    updated_at: new Date(),
    ...overrides,
  };
}

function makeItem(
  overrides: Partial<TaskBreakdownProposalItem>
): TaskBreakdownProposalItem {
  return {
    id: 'item-a',
    proposal_id: 'proposal-1',
    title: 'Item',
    description: null,
    sort_order: 0n,
    depends_on_item_ids: '[]',
    created_at: new Date(),
    ...overrides,
  };
}

describe('BreakdownReviewDialog', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    modalState.visible = true;
    proposalState.proposal = makeProposal();
    proposalState.items = [];
  });

  it('renders items with titles and dependency chips', () => {
    proposalState.items = [
      makeItem({ id: 'a', title: 'Item A', sort_order: 0n }),
      makeItem({
        id: 'b',
        title: 'Item B',
        sort_order: 1n,
        depends_on_item_ids: '["a"]',
      }),
    ];

    renderDialog();

    expect(screen.getByDisplayValue('Item A')).toBeInTheDocument();
    expect(screen.getByDisplayValue('Item B')).toBeInTheDocument();
    expect(screen.getByTestId('breakdown-item-b-deps')).toHaveTextContent(
      'Item A'
    );
  });

  it('persists an edited title via putItems on save (blur)', () => {
    proposalState.items = [
      makeItem({ id: 'a', title: 'Item A', sort_order: 0n }),
    ];

    renderDialog();

    const titleInput = screen.getByDisplayValue('Item A');
    fireEvent.change(titleInput, { target: { value: 'Renamed Item' } });
    fireEvent.blur(titleInput);

    expect(putItemsMutate).toHaveBeenCalledWith({
      proposalId: 'proposal-1',
      payload: {
        items: [
          {
            title: 'Renamed Item',
            description: null,
            sort_order: 0n,
            depends_on_indices: [],
          },
        ],
      },
    });
  });

  it('remaps surviving dependency indices in the putItems payload on delete', () => {
    proposalState.items = [
      makeItem({ id: 'a', title: 'Item A', sort_order: 0n }),
      makeItem({
        id: 'b',
        title: 'Item B',
        sort_order: 1n,
        depends_on_item_ids: '["a"]',
      }),
      makeItem({
        id: 'c',
        title: 'Item C',
        sort_order: 2n,
        depends_on_item_ids: '["a","b"]',
      }),
    ];

    renderDialog();

    const deleteButtons = screen.getAllByLabelText('Delete item');
    // Delete Item B (index 1)
    fireEvent.click(deleteButtons[1]);

    expect(putItemsMutate).toHaveBeenCalledWith({
      proposalId: 'proposal-1',
      payload: {
        items: [
          {
            title: 'Item A',
            description: null,
            sort_order: 0n,
            depends_on_indices: [],
          },
          {
            title: 'Item C',
            description: null,
            sort_order: 1n,
            depends_on_indices: [0n],
          },
        ],
      },
    });

    expect(screen.queryByDisplayValue('Item B')).not.toBeInTheDocument();
  });

  it('produces an updated sort_order + remapped depends_on_indices on reorder', () => {
    proposalState.items = [
      makeItem({ id: 'a', title: 'Item A', sort_order: 0n }),
      makeItem({
        id: 'b',
        title: 'Item B',
        sort_order: 1n,
        depends_on_item_ids: '["a"]',
      }),
    ];

    renderDialog();

    // Move Item A (index 0) down, swapping with Item B
    const moveDownButtons = screen.getAllByLabelText('Move down');
    fireEvent.click(moveDownButtons[0]);

    expect(putItemsMutate).toHaveBeenCalledWith({
      proposalId: 'proposal-1',
      payload: {
        items: [
          {
            title: 'Item B',
            description: null,
            sort_order: 0n,
            depends_on_indices: [1n],
          },
          {
            title: 'Item A',
            description: null,
            sort_order: 1n,
            depends_on_indices: [],
          },
        ],
      },
    });
  });

  it('calls accept via the hook then closes', () => {
    proposalState.items = [
      makeItem({ id: 'a', title: 'Item A', sort_order: 0n }),
    ];

    renderDialog();

    fireEvent.click(screen.getByText('Accept'));

    expect(acceptMutate).toHaveBeenCalledWith('proposal-1');
    expect(mockRemove).toHaveBeenCalled();
  });

  it('calls discard via the hook then closes', () => {
    proposalState.items = [
      makeItem({ id: 'a', title: 'Item A', sort_order: 0n }),
    ];

    renderDialog();

    fireEvent.click(screen.getByText('Discard'));

    expect(discardMutate).toHaveBeenCalledWith('proposal-1');
    expect(mockRemove).toHaveBeenCalled();
  });

  it("renders the localized error and a wired Retry button when status is 'failed'", () => {
    proposalState.proposal = makeProposal({
      status: 'failed',
      error: 'Something went wrong generating the breakdown',
    });
    proposalState.items = [];

    renderDialog();

    expect(
      screen.getByText('Something went wrong generating the breakdown')
    ).toBeInTheDocument();

    fireEvent.click(screen.getByText('Retry'));
    expect(retryMutate).toHaveBeenCalledWith('proposal-1');
  });

  it("renders the running state when status is 'draft' with zero items and a live run", () => {
    proposalState.proposal = makeProposal({
      status: 'draft',
      execution_process_id: 'ep-1',
    });
    proposalState.items = [];

    renderDialog();

    expect(screen.getByTestId('breakdown-running-state')).toBeInTheDocument();
  });
});
