import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { Input, Switch, Checkbox } from './index';

describe('Input (SC4)', () => {
  it('emits vks-input for defaults, vks-input--mono when mono', () => {
    const { container: c1 } = render(<Input placeholder="x" />);
    expect(c1.firstChild).toHaveClass('vks-input');
    expect(c1.firstChild).not.toHaveClass('vks-input--mono');
    const { container: c2 } = render(<Input mono placeholder="y" />);
    expect(c2.firstChild).toHaveClass('vks-input--mono');
  });

  it('passes through native input props (type, value, onChange)', () => {
    const onChange = vi.fn();
    render(<Input type="email" value="a@b" onChange={onChange} />);
    const el = screen.getByDisplayValue('a@b') as HTMLInputElement;
    expect(el.type).toBe('email');
    fireEvent.change(el, { target: { value: 'c@d' } });
    expect(onChange).toHaveBeenCalled();
  });
});

describe('Switch (SC4)', () => {
  it('renders a button[role=switch] with vks-switch + __thumb', () => {
    const { container } = render(<Switch />);
    const btn = container.querySelector('button[role="switch"]');
    expect(btn).toHaveClass('vks-switch');
    expect(container.querySelector('.vks-switch__thumb')).toBeTruthy();
  });

  it('toggles data-checked on click (uncontrolled, defaultChecked)', () => {
    const { container } = render(<Switch defaultChecked />);
    const btn = container.querySelector('button[role="switch"]') as HTMLButtonElement;
    expect(btn.dataset.checked).toBe('true');
    fireEvent.click(btn);
    expect(btn.dataset.checked).toBe('false');
  });

  it('calls onCheckedChange on click', () => {
    const onCheckedChange = vi.fn();
    const { container } = render(<Switch onCheckedChange={onCheckedChange} />);
    fireEvent.click(container.querySelector('button[role="switch"]')!);
    expect(onCheckedChange).toHaveBeenCalledWith(true);
  });
});

describe('Checkbox (SC4)', () => {
  it('renders a button[role=checkbox] with vks-checkbox', () => {
    const { container } = render(<Checkbox />);
    expect(container.querySelector('button[role="checkbox"]')).toHaveClass('vks-checkbox');
  });

  it('toggles data-checked on click (uncontrolled, defaultChecked)', () => {
    const { container } = render(<Checkbox defaultChecked />);
    const btn = container.querySelector('button[role="checkbox"]') as HTMLButtonElement;
    expect(btn.dataset.checked).toBe('true');
    fireEvent.click(btn);
    expect(btn.dataset.checked).toBe('false');
  });

  it('calls onCheckedChange on click', () => {
    const onCheckedChange = vi.fn();
    const { container } = render(<Checkbox onCheckedChange={onCheckedChange} />);
    fireEvent.click(container.querySelector('button[role="checkbox"]')!);
    expect(onCheckedChange).toHaveBeenCalledWith(true);
  });

  it('renders the check svg (11x11 viewBox 0 0 12 12)', () => {
    const { container } = render(<Checkbox defaultChecked />);
    const svg = container.querySelector('svg');
    expect(svg).toBeTruthy();
    expect(svg?.getAttribute('viewBox')).toBe('0 0 12 12');
  });
});
