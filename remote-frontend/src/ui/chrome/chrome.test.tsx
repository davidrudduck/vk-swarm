import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { Navbar, Logo, ThemeToggle, NavIcon, NavTab, ICONS } from './index';

describe('Chrome (SC7)', () => {
  it('Logo renders vks-wordmark with .vk and .swarm spans', () => {
    const { container } = render(<Logo />);
    expect(container.firstChild).toHaveClass('vks-wordmark');
    expect(container.querySelector('.vk')).toBeTruthy();
    expect(container.querySelector('.swarm')).toBeTruthy();
  });

  it('ThemeToggle emits a ghost icon button that calls onToggle on click', () => {
    const onToggle = vi.fn();
    const { container } = render(<ThemeToggle theme="dark" onToggle={onToggle} />);
    const btn = container.querySelector('button')!;
    expect(btn).toHaveClass('vks-btn--ghost');
    fireEvent.click(btn);
    expect(onToggle).toHaveBeenCalled();
  });

  it('Navbar renders 3 NavTabs (Board/Nodes/Processes) and calls onView', () => {
    const onView = vi.fn();
    render(<Navbar project="proj" view="board" onView={onView} onNewTask={() => {}} theme="dark" onToggleTheme={() => {}} onOpenSettings={() => {}} />);
    expect(screen.getByText('Board')).toBeTruthy();
    expect(screen.getByText('Nodes')).toBeTruthy();
    expect(screen.getByText('Processes')).toBeTruthy();
    fireEvent.click(screen.getByText('Nodes'));
    expect(onView).toHaveBeenCalledWith('nodes');
  });

  it('Navbar renders the New Task primary button calling onNewTask', () => {
    const onNewTask = vi.fn();
    render(<Navbar project="p" view="board" onView={() => {}} onNewTask={onNewTask} theme="dark" onToggleTheme={() => {}} onOpenSettings={() => {}} />);
    fireEvent.click(screen.getByText(/Task/));
    expect(onNewTask).toHaveBeenCalled();
  });

  it('NavIcon renders a ghost icon button', () => {
    const { container } = render(<NavIcon icon={ICONS.plus} title="Add" />);
    expect(container.querySelector('button')).toHaveClass('vks-btn--ghost');
  });

  it('NavTab applies borderBottom primary when active', () => {
    const { container } = render(<NavTab active onClick={() => {}} icon={ICONS.folder} label="L" />);
    const btn = container.querySelector('button') as HTMLElement;
    expect(btn.style.borderBottom).toContain('var(--primary)');
  });
});
