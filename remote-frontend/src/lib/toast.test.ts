// @vitest-environment node
import { describe, it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const source = readFileSync(join(__dirname, 'toast.ts'), 'utf-8');

describe('toast wrapper (SC3, SC4)', () => {
  it('exports toast from sonner as re-export', () => {
    expect(source).toMatch(/export\s+\{\s*toast[^}]*\}/);
  });

  it('has a toastError convenience that passes action-based retry', () => {
    expect(source).toContain('export function toastError');
    expect(source).toContain('action:');
    expect(source).toMatch(/label:\s*retry\.label\s*\?\?/);
  });

  it('has a toastSuccess convenience with undo action', () => {
    expect(source).toContain('export function toastSuccess');
    expect(source).toMatch(/label:\s*undo\.label\s*\?\?/);
  });

  it('re-exports Toaster from sonner', () => {
    expect(source).toMatch(/export\s+\{\s*[^}]*Toaster[^}]*\}/);
  });
});
