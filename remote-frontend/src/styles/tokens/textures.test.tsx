import { describe, it, expect } from 'vitest';
import { render } from '@testing-library/react';
import { readFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const base = readFileSync(join(__dirname, 'base.css'), 'utf-8');

describe('texture utilities (SC3)', () => {
  it('defines all 8 texture utility classes in base.css', () => {
    for (const cls of [
      '.vks-diagonal-lines', '.vks-ansi-dither', '.vks-ansi-weave',
      '.vks-ansi-grid', '.vks-scanlines', '.vks-dashed',
      '.vks-wordmark', '.vks-eyebrow',
    ]) {
      expect(base).toContain(cls);
    }
  });

  it('.vks-scanlines adds an ::after pseudo-element', () => {
    expect(base).toMatch(/\.vks-scanlines::after/);
  });

  it('.vks-wordmark styles .vk and .swarm spans', () => {
    expect(base).toContain('.vks-wordmark .vk');
    expect(base).toContain('.vks-wordmark .swarm');
  });

  it('renders a div with the vks-ansi-dither class', () => {
    const { container } = render(<div className="vks-ansi-dither" data-testid="tex" />);
    expect(container.firstChild).toHaveClass('vks-ansi-dither');
  });

  it('renders a vks-wordmark with .vk and .swarm children', () => {
    const { container } = render(
      <span className="vks-wordmark">
        <span className="vk">vk</span>
        <span className="swarm">swarm</span>
      </span>
    );
    expect(container.querySelector('.vk')).toBeTruthy();
    expect(container.querySelector('.swarm')).toBeTruthy();
  });
});
