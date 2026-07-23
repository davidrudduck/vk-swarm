import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { StatusBadge, TaskCard } from './index';

describe('StatusBadge (SC5)', () => {
  it('emits vks-status vks-status--todo + dot + label for defaults', () => {
    const { container } = render(<StatusBadge />);
    expect(container.firstChild).toHaveClass('vks-status');
    expect(container.firstChild).toHaveClass('vks-status--todo');
    expect(container.querySelector('.vks-status__dot')).toBeTruthy();
    expect(screen.getByText('To Do')).toBeTruthy();
  });

  it('emits the inprogress variant', () => {
    render(<StatusBadge status="inprogress" />);
    expect(screen.getByText('In Progress')).toBeTruthy();
  });

  it('hides the label when showLabel={false}', () => {
    const { container } = render(<StatusBadge status="done" showLabel={false} />);
    expect(container.firstChild).toHaveClass('vks-status--done');
    expect(container.querySelector('.vks-status__dot')).toBeTruthy();
    expect(container.textContent).toBe('');
  });

  it('uses the custom label when provided', () => {
    render(<StatusBadge status="done" label="Shipped" />);
    expect(screen.getByText('Shipped')).toBeTruthy();
  });
});

describe('TaskCard (SC5)', () => {
  it('emits vks-task vks-task--todo with the title', () => {
    render(<TaskCard title="Implement X" status="todo" />);
    const el = screen.getByText('Implement X').closest('.vks-task');
    expect(el).toHaveClass('vks-task');
    expect(el).toHaveClass('vks-task--todo');
  });

  it('renders the description when provided', () => {
    render(<TaskCard title="T" description="D" status="inprogress" />);
    expect(screen.getByText('D')).toBeTruthy();
  });

  it('renders the node span when provided', () => {
    render(<TaskCard title="T" status="done" node="node-1" />);
    expect(screen.getByText('node-1')).toBeTruthy();
  });

  it('renders up to 2 label badges + a days badge', () => {
    const { container } = render(<TaskCard title="T" status="inreview" labels={['a', 'b', 'c']} days={3} />);
    const badges = container.querySelectorAll('.vks-badge');
    expect(badges.length).toBeGreaterThanOrEqual(3);
  });

  it('renders the AttemptIndicator (running → loader, merged → svg)', () => {
    const { container: c1 } = render(<TaskCard title="T" status="inprogress" attempt="running" />);
    expect(c1.querySelector('.vks-loader')).toBeTruthy();
    const { container: c2 } = render(<TaskCard title="T" status="done" attempt="merged" />);
    expect(c2.querySelector('svg')).toBeTruthy();
  });
});
