---
id: "102"
phase: 1
title: "Add /v1/oauth exclusion inline to the vite.config.ts api-cache urlPattern (generateSW-safe)"
status: passed
depends_on: ["101"]
parallel: false
conflicts_with: []
files:
  - "remote-frontend/vite.config.ts"
irreversible: false
scope_test: "remote-frontend/src/lib/swCachePredicate.test.ts"
allowed_change: edit
forbid_after: ["Shape requests bypass the SW cache."]
covers_criteria: []
covers_tests: []
---
## Failing test (write first)
N/A — covered by existing tests: remote-frontend/src/lib/swCachePredicate.test.ts (task 101, the mirror pin). The behavioural pin for this wiring is the manual verification below (built sw.js contains the serialized exclusion + drift guards).


## Change
**File:** `remote-frontend/vite.config.ts`

**Anchor:** first `runtimeCaching` entry (~L17-24), the api-cache rule.

**Before:**
```ts
          {
            // Cache `/v1/` REST responses, but EXCLUDE `/v1/shape/*` (the Electric
            // proxy base). Electric shape traffic is long-poll/streaming; letting
            // Workbox's NetworkFirst cache it would serve stale/partial real-time
            // data (adversarial review F3). Shape requests bypass the SW cache.
            urlPattern: ({ url }) =>
              url.pathname.startsWith('/v1/') && !url.pathname.startsWith('/v1/shape'),
```
**After:**
```ts
          {
            // Cache `/v1/` REST responses, EXCLUDING `/v1/shape/*` (Electric
            // long-poll/streaming — adversarial review F3) and `/v1/oauth/*` (the
            // OAuth redirect chain; SW interception breaks sign-in on hive and
            // node — F-2026-08-03-02). Excluded requests bypass the SW entirely.
            // KEEP THIS ARROW SELF-CONTAINED: Workbox generateSW serializes it
            // into sw.js via toString(); an imported identifier would be
            // undefined at SW runtime. Mirrored + unit-tested in
            // src/lib/swCachePredicate.ts (drift-guarded, see task 102 evidence).
            urlPattern: ({ url }) =>
              url.pathname.startsWith('/v1/') &&
              !url.pathname.startsWith('/v1/shape') &&
              !url.pathname.startsWith('/v1/oauth'),
```

No import additions. No other lines change. The shell-cache rule's `/oauth/callback` and `/invitations/*/complete` special-cases MUST remain untouched.


## Allowed moves
Only the one comment+urlPattern replacement shown. Do not add imports, do not modify the NetworkFirst handler, cacheName, expiration, the asset-cache rule, the shell-cache rule, or the manifest.


## STOP triggers
The Before text is not found verbatim; `npx vite build` fails; the built dist/sw.js does NOT contain the string `v1/oauth`; any drift-guard grep below fails.


## Manual verification (record in decisions-ledger)
Record in decisions-ledger, all from `remote-frontend/`:
1. `npx vite build && grep -c 'v1/oauth' dist/sw.js` — count >= 1 (the serialized arrow carries the literal; this is what the spec's verify_cmd greps on the deployed hive).
2. `grep -c 'v1/shape' dist/sw.js` — count >= 1 (precedent exclusion preserved).
3. Drift guards tying config to mirror module: `grep -q "startsWith('/v1/oauth')" vite.config.ts && grep -q "startsWith('/v1/oauth')" src/lib/swCachePredicate.ts && echo DRIFT-GUARD-OK` and the same pair for `'/v1/shape'` — both print OK.


## Done when
`WAI_ROOT="$(ls -d ~/.claude/plugins/cache/agent-plugins/wai/[0-9]*/ | sort -V | tail -1)"; WAI_TYPECHECK_CMD="cd remote-frontend && npx tsc --noEmit" WAI_TEST_CMD="cd remote-frontend && npx vitest run src/lib/swCachePredicate.test.ts" bash "$WAI_ROOT/scripts/task-gate.sh" hive-oauth-sw-bypass 102` exits 0
