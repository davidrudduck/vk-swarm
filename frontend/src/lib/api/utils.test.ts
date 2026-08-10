import { describe, expect, it } from 'vitest';

import { ApiError, isHiveNotConfigured, jsonBody } from './utils';

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

/**
 * ts-rs maps Rust `i64` to `bigint`, so generated request types carry BigInts
 * that plain `JSON.stringify` refuses to serialise. `jsonBody` exists because
 * that threw at runtime for every edit in the breakdown review dialog while the
 * whole test suite stayed green (consumers mocked the api module out).
 */
describe('jsonBody', () => {
  it('serialises bigint as a JSON number', () => {
    expect(jsonBody({ sort_order: BigInt(3) })).toBe('{"sort_order":3}');
  });

  it('serialises nested bigints inside arrays', () => {
    expect(jsonBody({ deps: [BigInt(0), BigInt(2)] })).toBe('{"deps":[0,2]}');
  });

  it('round-trips to numbers, not strings, within the safe range', () => {
    const parsed = JSON.parse(jsonBody({ n: BigInt(Number.MAX_SAFE_INTEGER) }));
    expect(typeof parsed.n).toBe('number');
    expect(parsed.n).toBe(Number.MAX_SAFE_INTEGER);
  });

  it('throws rather than silently losing precision beyond the safe range', () => {
    // 2^53 + 2 cannot be represented exactly as a JS number. Coercing it would
    // corrupt the value; quoting it would be rejected by serde on an i64 field.
    const huge = BigInt(Number.MAX_SAFE_INTEGER) + BigInt(2);
    expect(() => jsonBody({ n: huge })).toThrow(RangeError);
  });

  it('throws for a large negative bigint too', () => {
    const tiny = BigInt(Number.MIN_SAFE_INTEGER) - BigInt(2);
    expect(() => jsonBody({ n: tiny })).toThrow(RangeError);
  });

  it('leaves non-bigint values untouched', () => {
    expect(jsonBody({ a: 1, b: 'two', c: null, d: [true] })).toBe(
      '{"a":1,"b":"two","c":null,"d":[true]}'
    );
  });
});
