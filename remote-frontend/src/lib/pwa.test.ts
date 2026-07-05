import { describe, it, expect, vi, beforeEach } from 'vitest';
import { registerSW } from './pwa';

vi.mock('workbox-window', () => ({
  Workbox: class {
    addEventListener = vi.fn();
    register = vi.fn().mockResolvedValue(undefined);
  },
}));

describe('PWA registration module (SC6)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    Object.defineProperty(navigator, 'serviceWorker', {
      value: {},
      configurable: true,
    });
  });

  it('exports registerSW function', () => {
    expect(typeof registerSW).toBe('function');
  });

  it('runs without error when serviceWorker is available', () => {
    expect(() => registerSW()).not.toThrow();
  });

  it('skips registration when serviceWorker is not available', () => {
    Object.defineProperty(navigator, 'serviceWorker', { value: undefined, configurable: true });
    expect(() => registerSW()).not.toThrow();
  });
});