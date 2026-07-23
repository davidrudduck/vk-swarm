import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { TaskDrawer } from './TaskDrawer';

const task = { id: 't1', title: 'Wire up OAuth callback', node: 'justX', labels: ['auth', 'backend'] };

describe('TaskDrawer (SC7)', () => {
  it('renders nothing when task is null', () => {
    const { container } = render(<TaskDrawer task={null} status="inprogress" onClose={() => {}} />);
    expect(container.firstChild).toBeNull();
  });

  it('renders the task title + status badge + tabs', () => {
    render(<TaskDrawer task={task} status="inprogress" onClose={() => {}} />);
    expect(screen.getByText('Wire up OAuth callback')).toBeTruthy();
    expect(screen.getByText('Diff')).toBeTruthy();
    expect(screen.getByText('Logs')).toBeTruthy();
    expect(screen.getByText('Attempts')).toBeTruthy();
  });

  it('calls onClose when the close button is clicked', () => {
    const onClose = vi.fn();
    const { container } = render(<TaskDrawer task={task} status="inprogress" onClose={onClose} />);
    fireEvent.click(container.querySelector('[aria-label="Close"], .vks-btn--ghost')!);
    // The overlay also calls onClose; click the first ghost button (close)
    expect(onClose).toHaveBeenCalled();
  });

  it('renders footer Merge / Rebase / Open in IDE buttons', () => {
    render(<TaskDrawer task={task} status="inprogress" onClose={() => {}} />);
    expect(screen.getByText('Merge')).toBeTruthy();
    expect(screen.getByText('Rebase')).toBeTruthy();
    expect(screen.getByText('Open in IDE')).toBeTruthy();
  });
});
