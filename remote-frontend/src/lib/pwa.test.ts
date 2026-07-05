// @vitest-environment node
import { describe, it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

describe('PWA registration module (SC6)', () => {
  it('exports registerSW function', () => {
    const source = readFileSync(join(__dirname, 'pwa.ts'), 'utf-8');
    expect(source).toContain('export function registerSW');
    expect(source).toContain('workbox-window');
  });

  it('imports Workbox and uses addEventListener', () => {
    const source = readFileSync(join(__dirname, 'pwa.ts'), 'utf-8');
    expect(source).toContain('new Workbox');
    expect(source).toContain("'waiting'");
    expect(source).toContain("'activated'");
    expect(source).toContain('window.location.reload()');
  });
});

describe('vite.config.ts PWA plugin (SC6, SC11)', () => {
  it('has VitePWA plugin config', () => {
    const source = readFileSync(join(__dirname, '../../vite.config.ts'), 'utf-8');
    expect(source).toContain('VitePWA');
    expect(source).toContain('VK Swarm Console');
    expect(source).toContain('theme_color');
    expect(source).toContain('#0f172a');
    expect(source).toContain('manifest');
    expect(source).toContain('workbox');
  });
});
