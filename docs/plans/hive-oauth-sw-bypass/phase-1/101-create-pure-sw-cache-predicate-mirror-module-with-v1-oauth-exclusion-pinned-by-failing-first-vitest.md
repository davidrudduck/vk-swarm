---
id: "101"
phase: 1
title: "Create pure SW cache predicate mirror module with /v1/oauth exclusion, pinned by failing-first vitest"
status: ready
depends_on: []
parallel: false
conflicts_with: []
files:
  - "remote-frontend/src/lib/swCachePredicate.ts"
  - "remote-frontend/src/lib/swCachePredicate.test.ts"
siblings: ["remote-frontend/src/lib/pwa.ts"]
irreversible: false
scope_test: "remote-frontend/src/lib/swCachePredicate.test.ts"
allowed_change: create
covers_criteria: []
covers_tests: ["TS1"]
---
## Failing test (write first)
Write remote-frontend/src/lib/swCachePredicate.test.ts FIRST and run it (module absent -> RED):

```ts
import { describe, expect, it } from 'vitest';
import { isApiCacheable } from './swCachePredicate';

describe('isApiCacheable (hive SW api-cache rule, mirror of vite.config.ts)', () => {
  it('excludes the OAuth start leg', () => {
    expect(isApiCacheable('/v1/oauth/github/start')).toBe(false);
  });
  it('excludes the OAuth callback leg', () => {
    expect(isApiCacheable('/v1/oauth/github/callback')).toBe(false);
  });
  it('excludes Electric shape traffic (adversarial review F3 precedent)', () => {
    expect(isApiCacheable('/v1/shape/tasks')).toBe(false);
  });
  it('still caches ordinary /v1/ API responses', () => {
    expect(isApiCacheable('/v1/projects')).toBe(true);
  });
  it('ignores non-/v1 paths', () => {
    expect(isApiCacheable('/assets/app.js')).toBe(false);
  });
});
```

Run: `cd remote-frontend && npx vitest run src/lib/swCachePredicate.test.ts` — must FAIL (cannot resolve ./swCachePredicate) before the module is created.


## Change
**File:** `remote-frontend/src/lib/swCachePredicate.ts` (CREATE)

**Anchor:** new file.

**After (entire file, exact):**

```ts
// MIRROR of the hive service worker's runtime api-cache rule in
// remote-frontend/vite.config.ts. It cannot be imported by the config: Workbox
// generateSW serializes the urlPattern arrow into sw.js via toString(), so the
// config's predicate must stay self-contained inline. This module exists to pin
// the exclusions under vitest; a drift-guard grep (task 102) ties the two
// copies together.
// Exclusions:
// - /v1/shape: Electric proxy long-poll/streaming traffic; caching would serve
//   stale/partial real-time data (adversarial review F3).
// - /v1/oauth: both OAuth legs (/v1/oauth/{provider}/start and
//   /v1/oauth/{provider}/callback) are GET navigations on the hive origin; a SW
//   intercepting or cache-falling-back on them breaks sign-in on the hive AND on
//   every node whose popup traverses this origin (F-2026-08-03-02).
export function isApiCacheable(pathname: string): boolean {
  return (
    pathname.startsWith('/v1/') &&
    !pathname.startsWith('/v1/shape') &&
    !pathname.startsWith('/v1/oauth')
  );
}
```

**File:** `remote-frontend/src/lib/swCachePredicate.test.ts` (CREATE) — exactly the test from 'Failing test' above. Before authoring, read sibling `remote-frontend/src/lib/pwa.ts` (plain module, no default export, single-purpose — the new files follow the same shape; colocated `*.test.ts` matches the existing `pwa.test.ts`/`toast.test.ts` convention).


## Allowed moves
Only creating the two listed files with the exact content above. Do not touch vite.config.ts (that is task 102). Do not add extra exports, options objects, or configuration parameters.


## Orchestrator amendment (2026-08-06, STOP resolution)
The RED phase of this task is ALREADY COMMITTED as `4420547d` ("test: create
swCachePredicate.test.ts (RED phase)") — the test file exists byte-identical to the
prescription above and its RED run is recorded in the executor journal. Do NOT recreate or
modify the test file. This attempt creates ONLY `remote-frontend/src/lib/swCachePredicate.ts`
(exact content from `## Change`), turning the suite GREEN. The existing-file STOP trigger
below applies to `swCachePredicate.ts` only.

## STOP triggers
remote-frontend has no vitest runner; an existing file at remote-frontend/src/lib/swCachePredicate.ts; the test file missing or differing from the prescription; the test passes before the module exists; lint/tsc reports errors that require touching any unlisted file.


## Manual verification (record in decisions-ledger)



## Done when
`WAI_ROOT="$(ls -d ~/.claude/plugins/cache/agent-plugins/wai/[0-9]*/ | sort -V | tail -1)"; WAI_TYPECHECK_CMD="cd remote-frontend && npx tsc --noEmit" WAI_TEST_CMD="cd remote-frontend && npx vitest run src/lib/swCachePredicate.test.ts" bash "$WAI_ROOT/scripts/task-gate.sh" hive-oauth-sw-bypass 101` exits 0
