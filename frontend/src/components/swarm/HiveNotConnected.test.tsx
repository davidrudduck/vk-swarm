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
  it('is true for the server’s HiveNotConfigured 503', () => {
    // Must be the REAL message shape: status alone is not sufficient, because
    // RemoteClientError::Http forwards an upstream 503 from a hive OUTAGE.
    expect(
      isHiveNotConfigured(
        new ApiError(
          'HiveNotConfigured: This node is not connected to a hive',
          503
        )
      )
    ).toBe(true);
  });

  it('is false for a configured hive that is DOWN (upstream 503 forwarded)', () => {
    expect(isHiveNotConfigured(new ApiError('Service Unavailable', 503))).toBe(
      false
    );
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
