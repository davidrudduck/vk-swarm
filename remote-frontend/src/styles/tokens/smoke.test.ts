// @vitest-environment node
import { describe, it, expect } from 'vitest';
import { execSync } from 'node:child_process';

describe('phase 1 integration smoke (SC2/SC3)', () => {
  it('npm run build exits 0 (tokens + textures compile through Vite)', () => {
    expect(() => execSync('npm run build', { stdio: 'pipe', timeout: 120_000 })).not.toThrow();
  }, 120_000);

  it('tsc --noEmit exits 0 (token test files type-check)', () => {
    expect(() => execSync('npx tsc --noEmit', { stdio: 'pipe', timeout: 120_000 })).not.toThrow();
  }, 120_000);

  it('eslint exits 0 on the tokens dir', () => {
    expect(() => execSync('npx eslint src/styles/tokens --max-warnings 0', { stdio: 'pipe', timeout: 60_000 })).not.toThrow();
  }, 60_000);
});
