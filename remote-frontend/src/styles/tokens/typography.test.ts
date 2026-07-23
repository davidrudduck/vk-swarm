// @vitest-environment node
import { describe, it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const typography = readFileSync(join(__dirname, 'typography.css'), 'utf-8');

describe('typography tokens (SC2)', () => {
  it('defines the 5 font families', () => {
    for (const token of ['--font-ui', '--font-code', '--font-display', '--font-wordmark', '--font-prose']) {
      expect(typography).toContain(`${token}:`);
    }
  });

  it('defines the downshifted type scale (base=14px)', () => {
    for (const token of ['--text-xs', '--text-sm', '--text-base', '--text-lg', '--text-xl', '--text-2xl', '--text-3xl', '--text-4xl', '--text-5xl']) {
      expect(typography).toContain(`${token}:`);
    }
    expect(typography).toContain('--text-base: 0.875rem');
  });

  it('defines leading, weight, and tracking tokens', () => {
    for (const token of ['--leading-tight', '--leading-relaxed', '--weight-regular', '--weight-bold', '--tracking-tight', '--tracking-wider']) {
      expect(typography).toContain(`${token}:`);
    }
  });
});
