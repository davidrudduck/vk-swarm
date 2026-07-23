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
