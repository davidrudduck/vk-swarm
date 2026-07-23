import { describe, it, expect, vi } from 'vitest';
import { render, fireEvent } from '@testing-library/react';
import { Tabs, Select, Loader } from './index';

describe('Tabs (SC4)', () => {
  it('renders a vks-tabs__list[role=tablist] with one vks-tabs__trigger[role=tab] per item', () => {
    const { container } = render(<Tabs tabs={[{ value: 'a', label: 'A' }, { value: 'b', label: 'B' }]} />);
    expect(container.querySelector('[role="tablist"]')).toHaveClass('vks-tabs__list');
    expect(container.querySelectorAll('[role="tab"]')).toHaveLength(2);
    expect(container.querySelectorAll('.vks-tabs__trigger')).toHaveLength(2);
  });

  it('derives deterministic id + aria-controls per trigger', () => {
    const { container } = render(<Tabs tabs={[{ value: 'diff', label: 'Diff' }, { value: 'logs', label: 'Logs' }]} />);
    const diff = container.querySelector('#vks-tab-diff')!;
    expect(diff.getAttribute('role')).toBe('tab');
    expect(diff.getAttribute('aria-controls')).toBe('vks-tabpanel-diff');
    expect(container.querySelector('#vks-tab-logs')!.getAttribute('aria-controls')).toBe('vks-tabpanel-logs');
  });

  it('sets data-active=true on the selected tab', () => {
    const { container } = render(<Tabs tabs={[{ value: 'a', label: 'A' }, { value: 'b', label: 'B' }]} value="b" />);
    const active = container.querySelector('[data-active="true"]') as HTMLButtonElement;
    expect(active).toBeTruthy();
    expect(active.textContent).toBe('B');
  });

  it('calls onValueChange on tab click', () => {
    const onValueChange = vi.fn();
    const { container } = render(<Tabs tabs={[{ value: 'a', label: 'A' }]} onValueChange={onValueChange} />);
    fireEvent.click(container.querySelector('[role="tab"]')!);
    expect(onValueChange).toHaveBeenCalledWith('a');
  });

  it('applies roving tabindex (active=0, others=-1)', () => {
    const { container } = render(
      <Tabs tabs={[{ value: 'a', label: 'A' }, { value: 'b', label: 'B' }, { value: 'c', label: 'C' }]} value="b" />
    );
    const tabs = container.querySelectorAll('[role="tab"]');
    expect(tabs[0].getAttribute('tabindex')).toBe('-1');
    expect(tabs[1].getAttribute('tabindex')).toBe('0');
    expect(tabs[2].getAttribute('tabindex')).toBe('-1');
  });

  it('ArrowRight moves focus/selection to the next tab and wraps', () => {
    const onValueChange = vi.fn();
    const { container } = render(
      <Tabs
        tabs={[{ value: 'a', label: 'A' }, { value: 'b', label: 'B' }, { value: 'c', label: 'C' }]}
        defaultValue="c"
        onValueChange={onValueChange}
      />
    );
    const tabs = container.querySelectorAll('[role="tab"]');
    fireEvent.keyDown(tabs[2], { key: 'ArrowRight' });
    expect(onValueChange).toHaveBeenCalledWith('a');
    expect(document.activeElement).toBe(tabs[0]);
  });

  it('ArrowLeft moves focus/selection to the previous tab and wraps', () => {
    const onValueChange = vi.fn();
    const { container } = render(
      <Tabs
        tabs={[{ value: 'a', label: 'A' }, { value: 'b', label: 'B' }, { value: 'c', label: 'C' }]}
        defaultValue="a"
        onValueChange={onValueChange}
      />
    );
    const tabs = container.querySelectorAll('[role="tab"]');
    fireEvent.keyDown(tabs[0], { key: 'ArrowLeft' });
    expect(onValueChange).toHaveBeenCalledWith('c');
    expect(document.activeElement).toBe(tabs[2]);
  });

  it('Home/End jump to first/last tab', () => {
    const onValueChange = vi.fn();
    const { container } = render(
      <Tabs
        tabs={[{ value: 'a', label: 'A' }, { value: 'b', label: 'B' }, { value: 'c', label: 'C' }]}
        defaultValue="b"
        onValueChange={onValueChange}
      />
    );
    const tabs = container.querySelectorAll('[role="tab"]');
    fireEvent.keyDown(tabs[1], { key: 'End' });
    expect(onValueChange).toHaveBeenLastCalledWith('c');
    expect(document.activeElement).toBe(tabs[2]);
    fireEvent.keyDown(tabs[2], { key: 'Home' });
    expect(onValueChange).toHaveBeenLastCalledWith('a');
    expect(document.activeElement).toBe(tabs[0]);
  });
});

describe('Select (SC4)', () => {
  it('renders a vks-select wrapper with a native select + __chevron', () => {
    const { container } = render(<Select options={[{ value: 'x', label: 'X' }]} />);
    expect(container.firstChild).toHaveClass('vks-select');
    expect(container.querySelector('select')).toBeTruthy();
    expect(container.querySelector('.vks-select__chevron')).toBeTruthy();
  });

  it('renders one option per entry', () => {
    const { container } = render(<Select options={[{ value: 'a', label: 'A' }, { value: 'b', label: 'B' }]} />);
    expect(container.querySelectorAll('option')).toHaveLength(2);
  });

  it('calls onValueChange on change', () => {
    const onValueChange = vi.fn();
    const { container } = render(<Select options={[{ value: 'a', label: 'A' }]} onValueChange={onValueChange} />);
    fireEvent.change(container.querySelector('select')!, { target: { value: 'a' } });
    expect(onValueChange).toHaveBeenCalledWith('a');
  });
});

describe('Loader (SC4)', () => {
  it('emits vks-loader with role=status', () => {
    const { container } = render(<Loader />);
    const el = container.firstChild as HTMLElement;
    expect(el).toHaveClass('vks-loader');
    expect(el.getAttribute('role')).toBe('status');
  });

  it('applies width/height for the size token', () => {
    const { container } = render(<Loader size="lg" />);
    const el = container.firstChild as HTMLElement;
    expect(el.style.width).toBe('28px');
    expect(el.style.height).toBe('28px');
  });
});
