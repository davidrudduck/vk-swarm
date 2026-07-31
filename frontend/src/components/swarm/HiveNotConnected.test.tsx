import { describe, expect, it } from 'vitest';
import { render, screen } from '@testing-library/react';
import { HiveNotConnected } from './HiveNotConnected';
import { isHiveNotConfigured } from '@/lib/api/utils';
import { ApiError } from '@/lib/api/utils';

describe('HiveNotConnected', () => {
  it('renders an explicit not-connected message', () => {
    render(<HiveNotConnected />);
    expect(screen.getByText(/not connected to a hive/i)).toBeInTheDocument();
  });
});

describe('isHiveNotConfigured', () => {
  it('is true for a 503 ApiError', () => {
    expect(isHiveNotConfigured(new ApiError('nope', 503))).toBe(true);
  });

  it('is false for other errors', () => {
    expect(isHiveNotConfigured(new ApiError('bad', 400))).toBe(false);
    // Pin the match to EXACTLY 503: a generic server error must not render
    // "not connected to a hive". Without these, widening the detector to
    // `status >= 500` passes silently (found by task 402's panel, mutation D).
    expect(isHiveNotConfigured(new ApiError('server', 500))).toBe(false);
    expect(isHiveNotConfigured(new ApiError('gateway', 502))).toBe(false);
    expect(isHiveNotConfigured(new ApiError('timeout', 504))).toBe(false);
    expect(isHiveNotConfigured(new Error('boom'))).toBe(false);
    expect(isHiveNotConfigured(null)).toBe(false);
  });
});
