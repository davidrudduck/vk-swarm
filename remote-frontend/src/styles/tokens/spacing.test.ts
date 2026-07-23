// @vitest-environment node
import { describe, it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const spacing = readFileSync(join(__dirname, 'spacing.css'), 'utf-8');

describe('spacing tokens (SC2)', () => {
  it('defines the 4px-grid scale (--space-0..--space-16)', () => {
    for (const token of ['--space-0', '--space-1', '--space-2', '--space-3', '--space-4', '--space-5', '--space-6', '--space-8', '--space-10', '--space-12', '--space-16']) {
      expect(spacing).toContain(`${token}:`);
    }
    expect(spacing).toContain('--space-1: 0.25rem');
    expect(spacing).toContain('--space-16: 4rem');
  });

  it('defines control heights', () => {
    for (const token of ['--control-xs', '--control-sm', '--control-md', '--control-lg']) {
      expect(spacing).toContain(`${token}:`);
    }
    expect(spacing).toContain('--control-md: 2.5rem');
  });

  it('defines radius tokens', () => {
    for (const token of ['--radius-sm', '--radius-md', '--radius-lg', '--radius-xl', '--radius-full']) {
      expect(spacing).toContain(`${token}:`);
    }
  });

  it('defines border, shadow, and glow tokens', () => {
    for (const token of ['--border-width', '--strip-width', '--shadow-sm', '--shadow-md', '--shadow-lg', '--glow-cyan', '--glow-emerald']) {
      expect(spacing).toContain(`${token}:`);
    }
  });
});
