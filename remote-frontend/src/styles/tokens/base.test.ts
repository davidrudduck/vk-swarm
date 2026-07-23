// @vitest-environment node
import { describe, it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const fonts = readFileSync(join(__dirname, 'fonts.css'), 'utf-8');
const base = readFileSync(join(__dirname, 'base.css'), 'utf-8');

describe('fonts (SC2)', () => {
  it('imports the 5 font families from Google Fonts', () => {
    expect(fonts).toContain('fonts.googleapis.com');
    for (const family of ['Inter', 'JetBrains+Mono', 'Chivo+Mono', 'Noto+Emoji', 'Source+Serif+4']) {
      expect(fonts).toContain(family);
    }
  });
});

describe('base element CSS (SC2)', () => {
  it('sets color-scheme dark by default', () => {
    expect(base).toMatch(/html\s*\{[^}]*color-scheme:\s*dark/);
  });

  it('resets body margin and applies background/foreground/font', () => {
    expect(base).toContain('body');
    expect(base).toContain('margin: 0');
    expect(base).toContain('var(--background)');
    expect(base).toContain('var(--text-body)');
  });

  it('styles anchor, code, selection', () => {
    expect(base).toContain('::selection');
    expect(base).toContain('code');
    expect(base).toMatch(/\ba\s*\{/);
  });
});
