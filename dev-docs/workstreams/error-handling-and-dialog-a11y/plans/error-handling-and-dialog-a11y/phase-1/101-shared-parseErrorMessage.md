---
id: "101"
phase: 1
title: "Create shared parseErrorMessage in src/lib/errors.ts"
status: ready
depends_on: []
parallel: false
conflicts_with: []
files:
  - remote-frontend/src/lib/errors.ts
irreversible: false
scope_test: "remote-frontend/src/lib/errors.test.ts"
allowed_change: create
covers_criteria: [SC1]
---
## Failing test (write first)
Covered by: `remote-frontend/src/lib/errors.test.ts` (task 102). The 17 tests in that file
exercise every code path in parseErrorMessage. Task 101 creates the implementation; task 102
creates the tests. They are split because the test file (17 tests, 100% coverage) is substantial
enough to warrant its own task.

## Change
For each file in files::
- **File:** remote-frontend/src/lib/errors.ts
- **Anchor:** N/A (new file)
- **Before:** (file does not exist)
- **After:**
```typescript
/**
 * Parse an unknown error into a user-friendly string.
 *
 * Handles: Error (including ApiError with error_data), string, null, symbol,
 * objects with {error} or {message} keys, JSON-encoded bodies, circular refs,
 * primitive JSON values. Returns 'Failed' as the generic fallback.
 */
export function parseErrorMessage(err: unknown): string {
  let raw: string;
  if (err instanceof Error) {
    raw = err.message;
  } else if (typeof err === 'string') {
    raw = err || 'Failed';
  } else if (err == null) {
    return 'Failed';
  } else if (typeof err === 'symbol') {
    return 'Failed';
  } else {
    try {
      raw = JSON.stringify(err) ?? 'Failed';
    } catch {
      return 'Failed';
    }
  }
  if (!raw) return 'Failed';
  try {
    const parsed = JSON.parse(raw);
    if (typeof parsed === 'string' && parsed) return parsed;
    if (parsed !== null && typeof parsed === 'object') {
      if (typeof parsed.message === 'string' && parsed.message) return parsed.message;
      if (typeof parsed.error === 'string' && parsed.error) return parsed.error;
      return 'Failed';
    }
    return raw || 'Failed';
  } catch {
    return raw || 'Failed';
  }
}
```

## Allowed moves
Create `remote-frontend/src/lib/errors.ts` with the exact content above.

## STOP triggers
- If `remote-frontend/src/lib/errors.ts` already exists (unexpected)
- If the implementation differs from NodeApiKeySection.tsx:34-64 (must be identical logic)

## Manual verification (record in decisions-ledger)
N/A — covered by scope_test (task 102).

## Done when
- `remote-frontend/src/lib/errors.ts` exists with the `parseErrorMessage` export
- Logic is identical to NodeApiKeySection.tsx:34-64
