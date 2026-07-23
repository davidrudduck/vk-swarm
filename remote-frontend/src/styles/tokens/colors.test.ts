// @vitest-environment node
import { describe, it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const colors = readFileSync(join(__dirname, 'colors.css'), 'utf-8');

describe('color tokens (SC2)', () => {
  it('defines dark-first HSL triplets + hex aliases for the 11 vks primitives', () => {
    for (const token of [
      '--vks-void', '--vks-surface', '--vks-surface-bright',
      '--vks-cyan', '--vks-amber', '--vks-emerald', '--vks-coral',
      '--vks-violet', '--vks-text', '--vks-text-muted', '--vks-text-dim',
    ]) {
      expect(colors).toContain(`${token}:`);
    }
  });

  it('defines the -hsl triplet for each of the 11 vks primitives', () => {
    for (const token of [
      '--vks-void', '--vks-surface', '--vks-surface-bright',
      '--vks-cyan', '--vks-amber', '--vks-emerald', '--vks-coral',
      '--vks-violet', '--vks-text', '--vks-text-muted', '--vks-text-dim',
    ]) {
      expect(colors).toContain(`${token}-hsl:`);
    }
  });

  it('defines the 5 status colors', () => {
    for (const token of ['--status-todo', '--status-inprogress', '--status-inreview', '--status-done', '--status-cancelled']) {
      expect(colors).toContain(`${token}:`);
    }
  });

  it('defines semantic aliases', () => {
    for (const token of ['--background', '--foreground', '--surface-card', '--primary', '--border', '--border-strong', '--ring']) {
      expect(colors).toContain(`${token}:`);
    }
  });

  it('defines the console palette', () => {
    for (const token of ['--console-bg', '--console-fg', '--console-muted', '--console-success', '--console-error', '--console-accent']) {
      expect(colors).toContain(`${token}:`);
    }
  });

  it('declares a light-mode opt-in via [data-theme="light"], .theme-light', () => {
    expect(colors).toMatch(/\[data-theme=['"]light['"]\]\s*,\s*\.theme-light/);
  });
});
