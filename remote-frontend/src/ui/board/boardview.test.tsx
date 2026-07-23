import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { BoardView, COLUMNS } from './index';

const columns = {
  todo: [{ id: 't1', title: 'First', description: 'd', node: 'n1', labels: ['a'], days: 1 }],
  inprogress: [
    { id: 't2', title: 'Second', node: 'n2', labels: [], attempt: 'running' as const, days: 2 },
  ],
  inreview: [],
  done: [],
  cancelled: [],
};

describe('BoardView (SC7)', () => {
  it('renders one column per COLUMNS entry with the label', () => {
    render(<BoardView columns={columns} onAdd={() => {}} onOpen={() => {}} />);
    for (const col of COLUMNS) expect(screen.getByText(col.label)).toBeTruthy();
  });

  it('renders a TaskCard per row in each column', () => {
    render(<BoardView columns={columns} onAdd={() => {}} onOpen={() => {}} />);
    expect(screen.getByText('First')).toBeTruthy();
    expect(screen.getByText('Second')).toBeTruthy();
  });

  it('renders the empty-state texture for empty columns', () => {
    const { container } = render(<BoardView columns={columns} onAdd={() => {}} onOpen={() => {}} />);
    expect(container.querySelector('.vks-ansi-dither')).toBeTruthy();
    expect(container.querySelector('.vks-scanlines')).toBeTruthy();
    expect(screen.getAllByText(/no tasks/).length).toBeGreaterThan(0);
  });

  it('calls onOpen(task, statusKey) when a TaskCard is clicked', () => {
    const onOpen = vi.fn();
    render(<BoardView columns={columns} onAdd={() => {}} onOpen={onOpen} />);
    fireEvent.click(screen.getByText('First'));
    expect(onOpen).toHaveBeenCalledWith(expect.objectContaining({ id: 't1' }), 'todo');
  });

  it('applies the selected ring when selectedId matches a task id', () => {
    render(<BoardView columns={columns} onAdd={() => {}} onOpen={() => {}} selectedId="t1" />);
    const card = screen.getByText('First').closest('.vks-task') as HTMLElement;
    expect(card.style.boxShadow).toContain('var(--primary)');
  });
});
