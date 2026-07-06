import { describe, it, expect } from 'vitest';
import { getSyncStatus } from './sync-status';

describe('getSyncStatus function', () => {
  it('returns synced when last update < 30s ago', () => {
    expect(getSyncStatus(Date.now() - 10_000)).toBe('synced');
  });

  it('returns reconnecting when 30-60s since last update', () => {
    expect(getSyncStatus(Date.now() - 45_000)).toBe('reconnecting');
  });

  it('returns disconnected when > 60s since last update', () => {
    expect(getSyncStatus(Date.now() - 90_000)).toBe('disconnected');
  });

  it('returns synced when lastUpdateAt is null (initial state)', () => {
    expect(getSyncStatus(null)).toBe('synced');
  });
});
