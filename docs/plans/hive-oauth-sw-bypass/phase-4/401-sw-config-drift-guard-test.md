---
id: "401"
phase: 4
title: "Automated drift-guard test for sw config (denylist + api-cache predicate) + correct stale comments"
status: passed
depends_on: []
parallel: false
conflicts_with: []
files:
  - "remote-frontend/src/lib/swConfigDriftGuard.test.ts"
  - "remote-frontend/vite.config.ts"
  - "remote-frontend/src/lib/swCachePredicate.ts"
irreversible: false
scope_test: "remote-frontend/src/lib/swConfigDriftGuard.test.ts"
allowed_change: mixed
forbid_after: ["drift-guard grep (task 102) ties the two copies together", "Excluded requests bypass the SW entirely."]
covers_criteria: []
covers_tests: []
---
## Context (code-review round 1, findings 1-3)

`navigateFallbackDenylist: [/^\/v1\//]` in `remote-frontend/vite.config.ts` is the load-bearing
OAuth fix (deploy verification round 1 proved the api-cache exclusion alone insufficient), yet
nothing automated pins it: deleting it passes every gate. Likewise the api-cache urlPattern is
pinned only via a hand-copied mirror (`swCachePredicate.ts`) whose sync guard was a one-time
manual grep. The frozen spec's D2 requires the predicate to stay INLINE in vite.config.ts
(generateSW toString serialization), so the fix is a SOURCE-READING drift-guard test, not an
export/import refactor.

## Failing test (write first)

Create `remote-frontend/src/lib/swConfigDriftGuard.test.ts` (vitest, node context — `fs` +
`path` imports are fine in tests). It must:

1. Read `vite.config.ts` source: `readFileSync(join(__dirname, '../../vite.config.ts'), 'utf8')`.
2. Assert the navigation denylist is present and correct:
   - source contains the literal `navigateFallbackDenylist`;
   - extract the regex literal from the line via `/navigateFallbackDenylist:\s*\[(\/.+?\/)\]/`
     and reconstruct it with `new RegExp(source-between-slashes)`;
   - assert it TESTS TRUE for `/v1/oauth/github/start`, `/v1/oauth/github/callback`,
     `/v1/oauth/google/start`, and `/v1/projects`;
   - assert it TESTS FALSE for `/`, `/login`, `/oauth/callback` (the SPA callback route must
     keep its shell fallback).
3. Assert config↔mirror predicate sync (replaces the manual grep):
   - config source contains `startsWith('/v1/')`, `!url.pathname.startsWith('/v1/shape')`,
     and `!url.pathname.startsWith('/v1/oauth')`;
   - mirror source (`readFileSync` of `swCachePredicate.ts`) contains
     `startsWith('/v1/')`, `!pathname.startsWith('/v1/shape')`,
     `!pathname.startsWith('/v1/oauth')`;
   - count of `startsWith(` occurrences inside the api-cache urlPattern arrow equals the count
     inside `isApiCacheable`'s body (clause-count sync: adding a clause to one side only fails).
4. Descriptive test names; one behaviour per test; no snapshot of the whole file.

RED first: before making any comment edits, run
`cd remote-frontend && npx vitest run src/lib/swConfigDriftGuard.test.ts`
with assertion 2's literal spelled as `navigateFallbackDenylistXX` (deliberately wrong) to
prove the test can fail, record the failure output, then correct it to the real literal and
show green. (The production config is already correct — the RED here proves the test is
discriminating, not that the config is broken.)

## Change

**File:** `remote-frontend/vite.config.ts` — two comment corrections, no code changes:

1. In the api-cache rule comment, replace the sentence
   `Excluded requests bypass the SW entirely.` with
   `Excluded requests bypass the SW caches (navigations additionally need the
   navigateFallbackDenylist above to reach the network).`
2. Replace the sentence referencing the manual drift guard
   (`Mirrored + unit-tested in src/lib/swCachePredicate.ts (drift-guarded, see task 102
   evidence).`) with
   `Mirrored + unit-tested in src/lib/swCachePredicate.ts; kept in sync by
   src/lib/swConfigDriftGuard.test.ts (source-reading drift guard).`

**File:** `remote-frontend/src/lib/swCachePredicate.ts` — doc comment corrections only:

1. In the module doc, make the oauth bullet's causal claim accurate: the PRIMARY sign-in fix
   is `navigateFallbackDenylist` in vite.config.ts (navigations); this cache exclusion is
   defense-in-depth for non-navigation `/v1/oauth` fetches.
2. Replace any reference to "a drift-guard grep (task 102)" with a pointer to
   `swConfigDriftGuard.test.ts`.
3. Do NOT change the exported function's code.

## Verification

```text
cd remote-frontend
npx vitest run src/lib/swConfigDriftGuard.test.ts   # all green
npx vitest run                                       # full suite green (>= 420 + new)
npm run lint && npx tsc --noEmit                     # clean
npm run build && grep -c 'denylist' dist/sw.js       # >= 1 (sanity)
```

## STOP triggers
- If extracting the regex literal proves brittle against Prettier formatting, STOP and report
  the exact formatted line — do not invent a different extraction strategy.
- If any existing test fails after the comment edits, STOP.
- Any file outside `files:` needed → STOP.
