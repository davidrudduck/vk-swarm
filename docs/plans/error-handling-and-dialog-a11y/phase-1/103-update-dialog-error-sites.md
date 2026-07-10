---
id: "103"
phase: 1
title: "Update all 6 dialog error call sites to use shared parseErrorMessage"
status: ready
depends_on: ["101"]
parallel: false
conflicts_with: []
files:
  - remote-frontend/src/components/swarm/NodeApiKeySection.tsx
  - remote-frontend/src/components/swarm/SwarmLabelDialog.tsx
  - remote-frontend/src/components/swarm/MergeProjectsDialog.tsx
  - remote-frontend/src/components/swarm/MergeLabelsDialog.tsx
  - remote-frontend/src/components/swarm/MergeTemplatesDialog.tsx
  - remote-frontend/src/components/swarm/SwarmProjectDialog.tsx
  - remote-frontend/src/components/swarm/SwarmTemplateDialog.tsx
irreversible: false
scope_test: "remote-frontend/src/components/swarm/NodeApiKeySection.test.tsx"
allowed_change: edit
covers_criteria: [SC2]
---
## Failing test (write first)
Covered by: `remote-frontend/src/components/swarm/NodeApiKeySection.test.tsx` (36 existing tests).
Tests TS7, TS13, TS16a-h, TS17 exercise error paths through parseErrorMessage. After this task,
all 36 tests must still pass — the shared utility produces identical output to the local one.

## Change

### NodeApiKeySection.tsx
- **File:** remote-frontend/src/components/swarm/NodeApiKeySection.tsx
- **Anchor:** lines 34-64 (local `parseErrorMessage` function)
- **Before:**
```typescript
function parseErrorMessage(err: unknown): string {
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
- **After:** (function deleted — import added at top of file)

Add import after existing imports:
```typescript
import { parseErrorMessage } from '@/lib/errors';
```

### SwarmLabelDialog.tsx
- **File:** remote-frontend/src/components/swarm/SwarmLabelDialog.tsx
- **Anchor:** line 87
- **Before:** `const message = err instanceof Error ? err.message : 'An error occurred';`
- **After:** `const message = parseErrorMessage(err);`
- Add import: `import { parseErrorMessage } from '@/lib/errors';`

### MergeProjectsDialog.tsx
- **File:** remote-frontend/src/components/swarm/MergeProjectsDialog.tsx
- **Anchor:** line 72
- **Before:** `const message = err instanceof Error ? err.message : 'An error occurred';`
- **After:** `const message = parseErrorMessage(err);`
- Add import: `import { parseErrorMessage } from '@/lib/errors';`

### MergeLabelsDialog.tsx
- **File:** remote-frontend/src/components/swarm/MergeLabelsDialog.tsx
- **Anchor:** line 88
- **Before:** `const message = err instanceof Error ? err.message : 'An error occurred';`
- **After:** `const message = parseErrorMessage(err);`
- Add import: `import { parseErrorMessage } from '@/lib/errors';`

### MergeTemplatesDialog.tsx
- **File:** remote-frontend/src/components/swarm/MergeTemplatesDialog.tsx
- **Anchor:** line 72
- **Before:** `const message = err instanceof Error ? err.message : 'An error occurred';`
- **After:** `const message = parseErrorMessage(err);`
- Add import: `import { parseErrorMessage } from '@/lib/errors';`

### SwarmProjectDialog.tsx
- **File:** remote-frontend/src/components/swarm/SwarmProjectDialog.tsx
- **Anchor:** line 77
- **Before:** `const message = err instanceof Error ? err.message : 'An error occurred';`
- **After:** `const message = parseErrorMessage(err);`
- Add import: `import { parseErrorMessage } from '@/lib/errors';`

### SwarmTemplateDialog.tsx
- **File:** remote-frontend/src/components/swarm/SwarmTemplateDialog.tsx
- **Anchor:** line 89
- **Before:** `const message = err instanceof Error ? err.message : 'An error occurred';`
- **After:** `const message = parseErrorMessage(err);`
- Add import: `import { parseErrorMessage } from '@/lib/errors';`

## Allowed moves
- Delete the local `parseErrorMessage` function from NodeApiKeySection.tsx
- Add `import { parseErrorMessage } from '@/lib/errors';` to each file
- Replace `err instanceof Error ? err.message : 'An error occurred'` with `parseErrorMessage(err)` in each file

## STOP triggers
- If any dialog file uses a DIFFERENT error pattern than `err instanceof Error ? err.message : 'An error occurred'`
- If removing the local function from NodeApiKeySection breaks existing tests

## Manual verification (record in decisions-ledger)
```bash
cd remote-frontend && npx vitest run
# Expected: all existing tests pass (36 NodeApiKeySection + any others)
cd remote-frontend && npx tsc --noEmit
# Expected: no type errors
```

## Done when
- All 6 files import from `@/lib/errors` instead of inline checks
- NodeApiKeySection no longer has a local `parseErrorMessage`
- All existing tests pass
