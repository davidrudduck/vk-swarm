---
id: "100"
phase: 1
title: Install sonner + create toast wrapper
status: ready
depends_on: []
parallel: false
conflicts_with: []
files:
  - remote-frontend/package.json
  - remote-frontend/src/lib/toast.ts
irreversible: false
scope_test: "remote-frontend/src/lib/toast"
allowed_change: mixed
covers_criteria: [SC3, SC4]
---

## Failing test (write first)

Create `remote-frontend/src/lib/toast.test.ts`:

```ts
// @vitest-environment node
import { describe, it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const source = readFileSync(join(__dirname, 'toast.ts'), 'utf-8');

describe('toast wrapper (SC3, SC4)', () => {
  it('exports toast from sonner as re-export', () => {
    expect(source).toMatch(/export\s+\{\s*toast\s*\}/);
  });

  it('has a toastError convenience that passes action-based retry', () => {
    expect(source).toContain('export function toastError');
    expect(source).toContain('action:');
    expect(source).toContain("label: 'Retry'");
  });

  it('has a toastSuccess convenience with undo action', () => {
    expect(source).toContain('export function toastSuccess');
    expect(source).toContain("label: 'Undo'");
  });

  it('re-exports Toaster from sonner', () => {
    expect(source).toMatch(/export\s+\{\s*Toaster\s*\}/);
  });
});
```

## Change

### File: `remote-frontend/package.json` (EDIT — add sonner dependency)

- **Anchor:** `"dependencies"` block, the line `"react-router-dom": "^7.9.5"`
- **Before:**
  ```
  "react-router-dom": "^7.9.5",
  ```
- **After:**
  ```
  "react-router-dom": "^7.9.5",
  "sonner": "^2.0.7",
  ```
  Then run `cd remote-frontend && npm install` so lockfile is updated.

### File: `remote-frontend/src/lib/toast.ts` (CREATE)

```ts
import { toast, Toaster } from 'sonner';

export function toastError(
  message: string,
  retry?: { label?: string; onClick: () => void },
) {
  toast.error(message, {
    action: retry
      ? { label: retry.label ?? 'Retry', onClick: retry.onClick }
      : undefined,
  });
}

export function toastSuccess(
  message: string,
  undo?: { label?: string; onClick: () => void },
) {
  toast.success(message, {
    action: undo
      ? { label: undo.label ?? 'Undo', onClick: undo.onClick }
      : undefined,
  });
}

export { toast, Toaster };
```

## Allowed moves

- Edit `remote-frontend/package.json` to add `"sonner": "^2.0.7"` in dependencies (insert one line after `react-router-dom`).
- Run `cd remote-frontend && npm install` to update lockfile.
- Create `remote-frontend/src/lib/toast.ts` with the exact code above.
- Create `remote-frontend/src/lib/toast.test.ts` with the exact code above.
- Do NOT touch any other file.

## STOP triggers

- `npm install sonner` fails (check npm registry availability, network).
- `sonner` version `^2.0.7` fails to resolve (use the latest 2.x; if only 3.x is available, STOP and escalate — breaking API changes possible).
- The vitest environment cannot resolve `sonner` exports after install (verify with `cd remote-frontend && npx vitest run src/lib/toast` — the test reads the source file, not the module, so this should be fine; if vitest loader chokes on sonner module, STOP).
- The `src/lib/` directory already contains a `toast.ts` file (check with `ls remote-frontend/src/lib/toast.ts 2>/dev/null` — if it exists, this is a stale artifact from a prior aborted run; delete it or rename before proceeding).

## Done when
`WAI_TYPECHECK_CMD="cd remote-frontend && npx tsc --noEmit" WAI_TEST_CMD="cd remote-frontend && npx vitest run src/lib/toast" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-hive-ui-polish 100` exits 0.