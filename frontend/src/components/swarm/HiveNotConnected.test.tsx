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
    expect(isHiveNotConfigured(new Error('boom'))).toBe(false);
    expect(isHiveNotConfigured(null)).toBe(false);
  });
});
