import { describe, it, expect } from 'vitest';
import { render } from '@testing-library/react';
import { SettingsSection, SettingsRow } from './index';
import { Switch, Checkbox } from '../core';

describe('SettingsSection (SC6)', () => {
  it('emits vks-card with header/title/desc/content', () => {
    const { container } = render(
      <SettingsSection title="General" description="Basics">
        <div data-testid="body" />
      </SettingsSection>
    );
    expect(container.firstChild).toHaveClass('vks-card');
    expect(container.querySelector('.vks-card__header')).toBeTruthy();
    expect(container.querySelector('.vks-card__title')!.textContent).toBe('General');
    expect(container.querySelector('.vks-card__desc')!.textContent).toBe('Basics');
    expect(container.querySelector('.vks-card__content')).toBeTruthy();
    expect(container.querySelector('[data-testid="body"]')).toBeTruthy();
  });

  it('renders the icon header when icon is provided', () => {
    const { container } = render(<SettingsSection title="T" icon={<span data-testid="i" />} />);
    expect(container.querySelector('.vks-settings__header')).toBeTruthy();
    expect(container.querySelector('.vks-settings__header-icon')).toBeTruthy();
  });

  it('renders the footer when provided', () => {
    const { container } = render(<SettingsSection title="T" footer={<div data-testid="f" />} />);
    expect(container.querySelector('.vks-card__footer')).toBeTruthy();
    expect(container.querySelector('[data-testid="f"]')).toBeTruthy();
  });
});

describe('SettingsRow (SC6)', () => {
  it('stacked layout: vks-field with label/body/error/helper', () => {
    const { container } = render(
      <SettingsRow label="Name" htmlFor="n" helper="hint" error="bad" control={<input id="n" />} />
    );
    expect(container.firstChild).toHaveClass('vks-field');
    expect(container.querySelector('.vks-field__label')).toBeTruthy();
    expect(container.querySelector('.vks-field__error')!.textContent).toBe('bad');
  });

  it('inline layout: vks-field--inline with __body', () => {
    const { container } = render(<SettingsRow label="X" inline control={<input />} />);
    expect(container.firstChild).toHaveClass('vks-field--inline');
    expect(container.querySelector('.vks-field__body')).toBeTruthy();
  });

  it('nested layout adds vks-field--nested', () => {
    const { container } = render(<SettingsRow label="X" nested control={<input />} />);
    expect(container.firstChild).toHaveClass('vks-field--nested');
  });

  it('renders helper when no error', () => {
    const { container } = render(<SettingsRow label="X" helper="hint" control={<input />} />);
    expect(container.querySelector('.vks-field__helper')!.textContent).toBe('hint');
  });

  it('error replaces helper in the stacked layout', () => {
    const { container } = render(<SettingsRow label="X" helper="hint" error="bad" control={<input />} />);
    expect(container.querySelector('.vks-field__error')!.textContent).toBe('bad');
    expect(container.querySelector('.vks-field__helper')).toBeNull();
  });

  it('error replaces helper in the inline layout', () => {
    const { container } = render(<SettingsRow label="X" inline helper="hint" error="bad" control={<input />} />);
    expect(container.querySelector('.vks-field__error')!.textContent).toBe('bad');
    expect(container.querySelector('.vks-field__helper')).toBeNull();
  });

  it('composes htmlFor label with a Switch id (typed passthrough)', () => {
    const { container } = render(
      <SettingsRow label="Notify" htmlFor="notify" inline control={<Switch id="notify" aria-label="Notify" />} />
    );
    const label = container.querySelector('.vks-field__label') as HTMLLabelElement;
    const control = container.querySelector('button[role="switch"]') as HTMLButtonElement;
    expect(label.getAttribute('for')).toBe('notify');
    expect(control.id).toBe('notify');
    expect(control.getAttribute('aria-label')).toBe('Notify');
  });

  it('composes htmlFor label with a Checkbox id (typed passthrough)', () => {
    const { container } = render(
      <SettingsRow label="Agree" htmlFor="agree" inline control={<Checkbox id="agree" aria-labelledby="lbl" />} />
    );
    const label = container.querySelector('.vks-field__label') as HTMLLabelElement;
    const control = container.querySelector('button[role="checkbox"]') as HTMLButtonElement;
    expect(label.getAttribute('for')).toBe('agree');
    expect(control.id).toBe('agree');
    expect(control.getAttribute('aria-labelledby')).toBe('lbl');
  });
});
