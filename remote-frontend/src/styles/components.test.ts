// @vitest-environment node
import { describe, it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const css = readFileSync(join(__dirname, 'components.css'), 'utf-8');

describe('component CSS classes (SC1)', () => {
  it('defines the primary component classes', () => {
    for (const cls of [
      '.vks-btn', '.vks-badge', '.vks-status', '.vks-card', '.vks-input',
      '.vks-switch', '.vks-checkbox', '.vks-tabs__list', '.vks-tabs__trigger',
      '.vks-select', '.vks-loader', '.vks-task', '.vks-node', '.vks-field',
      '.vks-alert', '.vks-savebar',
    ]) {
      expect(css).toContain(cls);
    }
  });

  it('defines the button variants + sizes', () => {
    for (const cls of ['--primary', '--secondary', '--outline', '--ghost', '--destructive', '--link', '--xs', '--sm', '--md', '--lg', '--icon']) {
      expect(css).toContain(`.vks-btn${cls}`);
    }
  });

  it('defines the task status strip + node pulse keyframes', () => {
    expect(css).toContain('@keyframes vks-spin');
    expect(css).toContain('@keyframes vks-pulse');
    expect(css).toContain('.vks-task::before');
    expect(css).toContain('.vks-node__pulse');
  });

  it('defines hover/focus/active/disabled states on the button', () => {
    expect(css).toMatch(/\.vks-btn:focus-visible/);
    expect(css).toMatch(/\.vks-btn:disabled/);
    expect(css).toMatch(/\.vks-btn--primary:hover/);
  });
});
