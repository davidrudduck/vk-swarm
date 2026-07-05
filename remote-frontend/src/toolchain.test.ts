import { describe, it, expect } from 'vitest';

describe('toolchain', () => {
  it('vitest is importable', () => {
    expect(typeof describe).toBe('function');
    expect(typeof it).toBe('function');
    expect(typeof expect).toBe('function');
  });
});
