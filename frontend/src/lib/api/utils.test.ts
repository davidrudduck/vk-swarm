import { describe, expect, it } from 'vitest';

import { ApiError, isHiveNotConfigured } from './utils';

/**
 * These tests exist because HTTP status alone CANNOT identify the hive-absent
 * case. `RemoteClientError::Http` forwards the upstream status verbatim
 * (`crates/server/src/error.rs`: `StatusCode::from_u16(*status)`), so a
 * configured-but-DOWN hive also reaches the browser as 503.
 *
 * Getting this wrong is not cosmetic: callers resolve rather than throw for
 * hive-absent (`useAvailableNodes` returns `{ nodes: [] }`, `useDiffStream` and
 * `useRemoteConnectionStatus` clear their error). So a misclassified OUTAGE
 * renders "not connected to a hive" AND suppresses the retry that would let it
 * recover on its own.
 */
describe('isHiveNotConfigured', () => {
  const hiveAbsent = () =>
    new ApiError(
      'HiveNotConfigured: This node is not connected to a hive',
      503
    );

  it('is true for the server’s HiveNotConfigured 503', () => {
    expect(isHiveNotConfigured(hiveAbsent())).toBe(true);
  });

  it('is FALSE for a configured hive that is down (upstream 503 forwarded)', () => {
    // Same status, different origin — the body is the hive's, not ours.
    const outage = new ApiError('Service Unavailable', 503);
    expect(isHiveNotConfigured(outage)).toBe(false);
  });

  it('is FALSE for an upstream 503 whose body merely mentions the hive', () => {
    const outage = new ApiError('hive is restarting, try again', 503);
    expect(isHiveNotConfigured(outage)).toBe(false);
  });

  it('is false for other statuses even with a matching message', () => {
    const wrongStatus = new ApiError(
      'HiveNotConfigured: This node is not connected to a hive',
      500
    );
    expect(isHiveNotConfigured(wrongStatus)).toBe(false);
  });

  it('is false for non-ApiError values', () => {
    expect(isHiveNotConfigured(new Error('boom'))).toBe(false);
    expect(isHiveNotConfigured(null)).toBe(false);
    expect(isHiveNotConfigured(undefined)).toBe(false);
    expect(isHiveNotConfigured({ status: 503 })).toBe(false);
  });
});
