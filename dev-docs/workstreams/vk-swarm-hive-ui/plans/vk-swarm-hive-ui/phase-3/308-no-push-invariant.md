---
id: "308"
phase: 3
title: No-push invariant — grep asserts zero WebSocket/SSE in the new board code
status: done
depends_on: ["305", "306", "307"]
parallel: false
conflicts_with: []
files:
  - remote-frontend/scripts/no-push-invariant.test.mjs
irreversible: false
scope_test: "remote-frontend/scripts/no-push-invariant.test.mjs"
allowed_change: edit
covers_criteria: [SC5]
---
## Failing test (write first)

Create `remote-frontend/scripts/no-push-invariant.test.mjs` (runs under node --test, auto-detected by the gate). To make this a REAL red-first test (not green-on-arrival), temporarily add `const _ws = new WebSocket('ws://x');` to `remote-frontend/src/pages/Tasks.tsx` BEFORE running the test the first time — the test fails red, proving the guard is live. Then remove the temporary line — the test passes green. Record the temporary-red-then-green verification in the decisions ledger.

```js
import { test } from 'node:test';
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join } from 'node:path';

const ROOT = join(import.meta.dirname, '..', 'src');

function listTsFiles(dir) {
  const out = [];
  for (const entry of readdirSync(dir)) {
    const full = join(dir, entry);
    if (statSync(full).isDirectory()) out.push(...listTsFiles(full));
    else if (/\.(ts|tsx)$/.test(entry)) out.push(full);
  }
  return out;
}

const PUSH_PATTERNS = /\bWebSocket\b|\bEventSource\b|\bnew\s+WebSocket\b|\bnew\s+EventSource\b|\/\.websocket|text\/event-stream|\bSSE\b/;

test('no new push channels (WebSocket/EventSource/SSE) in the hive frontend source', () => {
  const files = listTsFiles(ROOT);
  const violations = [];
  for (const f of files) {
    const src = readFileSync(f, 'utf8');
    const lines = src.split('\n');
    lines.forEach((line, i) => {
      if (PUSH_PATTERNS.test(line) && !/^\s*(\/\/|\/\*|\*)/.test(line)) {
        violations.push(`${f}:${i + 1}: ${line.trim()}`);
      }
    });
  }
  if (violations.length > 0) {
    throw new Error('Push-channel invariant violated (SC5 forbids new WS/SSE):\n' + violations.join('\n'));
  }
});
```

This is a **regression guard**, not a red-first TDD test (Codex F10). It is green-on-arrival by design — the source under `remote-frontend/src/` does not contain push-channel patterns when this task runs (tasks 305-307 use Electric shape polling, not WS/SSE). The guard's value is forward-looking: it catches a FUTURE regression if someone adds a WebSocket/EventSource to the board code. To verify the guard works, temporarily add `const _ws = new WebSocket('ws://x');` to `Tasks.tsx`, run the test (it fails red), then remove the line (it passes green). This temporary-red-then-green verification is recorded in the decisions ledger as evidence the guard is live.

## Change

### File: `remote-frontend/scripts/no-push-invariant.test.mjs` (CREATE)

The script above. It scans `remote-frontend/src/**/*.{ts,tsx}` for `WebSocket`, `EventSource`, `new WebSocket`, `new EventSource`, `/.websocket`, `text/event-stream`, `SSE` (case-sensitive for the class forms). Comments are skipped (lines starting with `//` or `*`).

**Sibling alignment:** Read `frontend/src/lib/electric/config.ts` + `collections.ts` — the existing Electric layer uses HTTP shape polling (via `createShapeUrl`), NOT WebSocket/EventSource. This invariant asserts the new board code does not regress that. Justify any divergence in the decisions ledger.

## Allowed moves
- CREATE `remote-frontend/scripts/no-push-invariant.test.mjs`.
- No source file changes (this is a guard, not a feature).

## STOP triggers
- The test finds a real `WebSocket`/`EventSource`/`SSE` use in `remote-frontend/src/**` — HALT; that's an SC5 violation. Fix the source to use Electric shape polling instead, or escalate if a push channel is genuinely required (spec says none).

## Manual verification (record in decisions-ledger)
```bash
cd remote-frontend && node --test scripts/no-push-invariant.test.mjs
```
Exits 0 (pass). To prove the guard bites: temporarily add `// WebSocket` to `src/pages/Tasks.tsx`, re-run, confirm it fails, then revert.

## Done when
`WAI_TYPECHECK_CMD="cd remote-frontend && npx tsc --noEmit" WAI_TEST_CMD="cd remote-frontend && node --test scripts/no-push-invariant.test.mjs" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-hive-ui 308` exits 0