---
id: "102"
phase: 1
title: "Create parseErrorMessage unit tests with 100% line coverage"
status: ready
depends_on: ["101"]
parallel: false
conflicts_with: []
files:
  - remote-frontend/src/lib/errors.test.ts
irreversible: false
scope_test: "remote-frontend/src/lib/errors.test.ts"
allowed_change: create
covers_criteria: [SC1]
---
## Failing test (write first)
This task IS the test file. All tests fail until task 101 creates the implementation.

## Change
For each file in files::
- **File:** remote-frontend/src/lib/errors.test.ts
- **Anchor:** N/A (new file)
- **Before:** (file does not exist)
- **After:**
```typescript
import { describe, it, expect } from 'vitest';
import { parseErrorMessage } from './errors';

describe('parseErrorMessage', () => {
  it('returns error.message for Error instances', () => {
    expect(parseErrorMessage(new Error('boom'))).toBe('boom');
  });

  it('returns string as-is', () => {
    expect(parseErrorMessage('plain failure')).toBe('plain failure');
  });

  it('returns "Failed" for empty string', () => {
    expect(parseErrorMessage('')).toBe('Failed');
  });

  it('returns "Failed" for null', () => {
    expect(parseErrorMessage(null)).toBe('Failed');
  });

  it('returns "Failed" for undefined', () => {
    expect(parseErrorMessage(undefined)).toBe('Failed');
  });

  it('returns "Failed" for symbol', () => {
    expect(parseErrorMessage(Symbol('x'))).toBe('Failed');
  });

  it('returns "Failed" for plain object with no message/error keys', () => {
    expect(parseErrorMessage({ code: 'E_DENIED' })).toBe('Failed');
  });

  it('extracts message from JSON-encoded Error body with {message} key', () => {
    expect(parseErrorMessage(new Error('{"message":"server denied"}'))).toBe('server denied');
  });

  it('extracts error from JSON-encoded Error body with {error} key', () => {
    expect(parseErrorMessage(new Error('{"error":"not found"}'))).toBe('not found');
  });

  it('extracts string primitive from JSON-encoded Error body', () => {
    expect(parseErrorMessage(new Error('"just a string"'))).toBe('just a string');
  });

  it('returns raw for JSON-encoded number primitive', () => {
    expect(parseErrorMessage(new Error('42'))).toBe('42');
  });

  it('returns "Failed" for circular reference object', () => {
    const circular: Record<string, unknown> = {};
    circular.self = circular;
    expect(parseErrorMessage(circular)).toBe('Failed');
  });

  it('returns "Failed" for boolean primitive in JSON body', () => {
    expect(parseErrorMessage(new Error('true'))).toBe('true');
  });

  it('handles object with {message: ""} as no message', () => {
    expect(parseErrorMessage(new Error('{"message":""}'))).toBe('Failed');
  });

  it('handles object with {error: ""} as no error', () => {
    expect(parseErrorMessage(new Error('{"error":""}'))).toBe('Failed');
  });

  it('prefers {message} over {error} when both present', () => {
    expect(parseErrorMessage(new Error('{"message":"msg","error":"err"}'))).toBe('msg');
  });

  it('returns "Failed" for object with nested non-string message', () => {
    expect(parseErrorMessage(new Error('{"message":123}'))).toBe('Failed');
  });
});
```

## Allowed moves
Create `remote-frontend/src/lib/errors.test.ts` with the exact content above.

## STOP triggers
- If the test file already exists (unexpected)
- If any test passes without the implementation from task 101 (hollow test)

## Manual verification (record in decisions-ledger)
```bash
cd remote-frontend && npx vitest run src/lib/errors.test.ts
# Expected: all 17 tests pass
```

## Done when
- `remote-frontend/src/lib/errors.test.ts` exists
- `npx vitest run src/lib/errors.test.ts` passes (17 tests)
- 100% line coverage on `src/lib/errors.ts`
