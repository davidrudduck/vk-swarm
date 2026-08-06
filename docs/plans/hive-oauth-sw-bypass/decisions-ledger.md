# Decisions Ledger

## Submission
Plan accepted from submit envelope.

## Pre-execution (decompose)

### 2026-08-05 — breakdown review round 1 (hybrid external + sub-agent-fallback)
See `reviews/round1-verdict.md`. Two CONFIRMED blockers forced a deliberate spec amendment
(Workbox generateSW toString serialization → predicate must be inline; spec re-frozen via
/wai:precheck after amendment) and a forbid_after literal swap. All other validated findings
applied via plan envelope v2 resubmission.

### Done-when placeholder fill
The plan submitter renders `Done when` lines with `<dir>/<typecheck>/<test>` scaffold
placeholders (no envelope field exists for them). Filled post-render with dynamic WAI_ROOT
resolution + concrete scoped commands per task (tournament codex-F1/mechanical-F2). This
completes the scaffold; it does not alter any gate-checked artifact.

### Precheck anchor-check false positives (recorded per no-deferred-remediation)
`--no-anchor-check` used on both precheck runs: the extractor truncates
`crates/remote/src/auth/handoff.rs` / `crates/server/src/routes/oauth.rs` to `src/...`
(both verified present on main via `git cat-file -e`), and
`remote-frontend/src/lib/swCachePredicate.ts` is a to-create file (verified absent on main).

### Plan-lint advisory W: warnings (acknowledged, not blocking)
- Task 101 creates `swCachePredicate{,.test}.ts` beside unlisted same-directory sibling
  `remote-frontend/src/lib/errors.test.ts`. Justification: `errors.test.ts` tests a different
  module (error helpers) and shares no pattern dependency with a URL predicate; the declared
  sibling `pwa.ts` (and its colocated `pwa.test.ts` convention) is the pattern reference the
  task follows. The new files neither import nor mock `errors.test.ts`.

## Executor ladder

- requested_executor: default
- actual_executor(s) used: haiku
- honored: n/a (default ladder)
- run_id: 20260806T012548Z-ad8f0631-3015469

## Executor ladder

- requested_executor: default
- actual_executor(s) used: haiku
- honored: n/a (default ladder)
- run_id: 20260806T013154Z-2a7881e8-3044839

### task 201
- No undictated choices.
- Manual verification: `for l in en ja ko es; do node -e "const o=require('./frontend/src/i18n/locales/$l/common.json'); if(!o.oauth.timeoutError||!o.oauth.tryAgain)process.exit(1)" && echo "$l OK"; done` → `en OK` / `ja OK` / `ko OK` / `es OK`.

### task 102

Manual verification (run from `remote-frontend/`, 2026-08-06):

```
$ npx vite build && grep -c 'v1/oauth' dist/sw.js
vite v8.0.7 building client environment for production...
✓ 2263 modules transformed.
✓ built in 729ms
PWA v1.3.0
mode      generateSW
precache  13 entries (521.99 KiB)
files generated
  dist/sw.js
  dist/workbox-07e28819.js
1

$ grep -c 'v1/shape' dist/sw.js
1

$ grep -q "startsWith('/v1/oauth')" vite.config.ts && grep -q "startsWith('/v1/oauth')" src/lib/swCachePredicate.ts && echo DRIFT-GUARD-OK
DRIFT-GUARD-OK

$ grep -q "startsWith('/v1/shape')" vite.config.ts && grep -q "startsWith('/v1/shape')" src/lib/swCachePredicate.ts && echo DRIFT-GUARD-OK
DRIFT-GUARD-OK
```

Before→After replacement applied byte-exactly; no undictated choices. Note: the WAI task-gate scans HEAD, so the gate was run after the commit (first pre-commit run failed only because HEAD still held the old text).

### task 202

**STOP TRIGGER HIT — task halted before commit.** Trigger (verbatim from the task's STOP list): "eslint rejects the disable comment in every placement tried."

Evidence:
- `frontend/eslint.config.js:51` sets `'@eslint-community/eslint-comments/no-use': ['error', { allow: [] }]` — ALL eslint directive comments are banned project-wide, so NO placement of the prescribed `eslint-disable-next-line react-hooks/exhaustive-deps` can pass ("Unexpected ESLint directive comment", error).
- Removing the directive is no escape: the non-negotiable `[isPolling]` deps array produces a `react-hooks/exhaustive-deps` warning for `t`, and `npm run lint` is `eslint . --max-warnings 0` (`frontend/package.json:11`), so the warning also fails the gate.
- Lint output with the prescribed comment in place:
```
frontend/src/components/dialogs/global/OAuthDialog.tsx
   88:0  error    Unexpected ESLint directive comment                    @eslint-community/eslint-comments/no-use
   88:3  warning  Unused eslint-disable directive (no problems were reported from 'react-hooks/exhaustive-deps')
  101:6  warning  React Hook useEffect has a missing dependency: 't'.   react-hooks/exhaustive-deps
```
  (The "unused directive" warning is because a multi-line disable comment only covers the literal next line, a comment line — but even single-line placement directly above the deps line still errors on `no-use`.)

Everything else in the task completed and is left in the working tree UNCOMMITTED pending a decision:

RED (before component change) — `cd frontend && npx vitest run src/components/dialogs/global/__tests__/OAuthDialog.test.tsx`:
```
 FAIL  src/components/dialogs/global/__tests__/OAuthDialog.test.tsx
  ✗ renders the localized timeout error and stops polling past the deadline
      (oauth.timeoutError not rendered — still oauth.waitingTitle)
  ✗ returns to provider select when tryAgain is clicked after timeout
      (no oauth.tryAgain in DOM — dialog still in waiting state)
  ✗ clears the deadline timer on unmount (expected 0 to be greater than 0 — no deadline timer exists)
 Test Files  1 failed (1)
      Tests  3 failed | 2 passed (5)
```

GREEN (after Anchor 1 + Anchor 2 applied exactly as prescribed):
```
 RUN  v4.1.3 /data/Code/vk-swarm-worktrees/hive-oauth-sw-bypass/frontend
 Test Files  1 passed (1)
      Tests  5 passed (5)
```

`cd frontend && npx tsc --noEmit` — exit 0, no errors.

Undictated harness choices (test file only): mocked `@/lib/modals` as `defineModal: (C) => C` per the sibling TaskFormSheet precedent (needed so `OAuthDialog` export is the raw component under the NiceModal mock); success-before-deadline case flips the mutable status result BEFORE clicking the provider button (mutating the object mid-poll cannot trigger a React re-render under the mocked hook — flipping first means the first polling render sees logged_in:true, still "success before the deadline").

Resolution options (need decompose-level decision, both touch config/code the task does not allow):
1. Add `'react-hooks/exhaustive-deps'` to the `no-use` allow list in `frontend/eslint.config.js` (unlisted file).
2. Drop the directive and silence the warning structurally (e.g. `tRef = useRef(t)` kept current via an effect, message resolved from the ref at fire time) — deviates from the prescribed effect body.

**STOP resolution (2026-08-06, Orchestrator amendment 2):** option 2 chosen — the deadline effect ships in the structural `tRef` form dictated verbatim by the amendment (`tRef = useRef(t)`; keep-current effect on `[t]`; deadline effect deps `[isPolling]` resolving `tRef.current('oauth.timeoutError')` at fire time). No eslint directive anywhere. Semantics unchanged: deadline never resets on `t` identity change; message resolved at fire time. The RED evidence above stands (captured against the pre-change component). Final verification of the shipped form:

```
$ cd frontend && npx vitest run src/components/dialogs/global/__tests__/OAuthDialog.test.tsx
 Test Files  1 passed (1)
      Tests  5 passed (5)

$ npx tsc --noEmit
(clean, exit 0)

$ npm run lint       # eslint . --max-warnings 0
(clean, exit 0 — zero errors, zero warnings)
```

No further undictated choices — the amendment's code block was applied byte-exactly.

## Reachability gate

### (a) Call-path trace (merged code on feat/hive-oauth-sw-bypass)
Production entry point: node user clicks "Continue with GitHub" in `OAuthDialog`
(`frontend/src/components/dialogs/global/OAuthDialog.tsx` — `handleProviderSelect` →
`initHandoff.mutate`; on success the popup opens at `data.authorize_url`, OAuthDialog.tsx:51-52).
`authorize_url` is minted by the hive as `{public_origin}/v1/oauth/{provider}/start?...`
(`crates/remote/src/auth/handoff.rs:157-166`), and the provider redirects back to
`{public_origin}/v1/oauth/{provider}/callback` (`handoff.rs:198-205`). Both are GET navigations
on the hive origin, where the PWA service worker is registered. The SW's ONLY runtime route for
`/v1/` is the api-cache rule in `remote-frontend/vite.config.ts`; the changed predicate at
vite.config.ts:27-29 now returns false for both legs (`!url.pathname.startsWith('/v1/oauth')`,
line 29), so the SW registers no respondWith for the OAuth chain — the fix executes on exactly
the path the bug lives on. The compiled sw.js carries the serialized arrow (task 102 evidence:
`grep -c 'v1/oauth' dist/sw.js` = 1), so the exclusion exists in the artifact the browser runs.
The second defect's path: a dead flow leaves `isPolling` true forever; the new deadline effect
(OAuthDialog.tsx, deps `[isPolling]`) is on that exact path and fires at POLL_DEADLINE_MS.

### (b) Real-seam test
- The Workbox config→sw.js seam is driven by the REAL build: task 102's verification runs
  `npx vite build` (vite-plugin-pwa generateSW) and asserts the exclusion literal is present in
  the emitted `dist/sw.js` (count=1) — not a mock of the serializer. Drift guards tie the inline
  predicate to the vitest-pinned mirror module clause-for-clause.
- The dialog seam is driven by the component's real UI: `OAuthDialog.test.tsx` renders the real
  component, clicks the real provider button, and observes the rendered error state + polling
  `enabled` flag across the deadline (5/5 green; RED-first evidence in `### task 202`).
- The full browser-level seam (registered SW intercepting a real popup navigation) is only
  observable on the deployed system — covered by `## Deploy verification` below (SC1b/SC2/SC3
  operator evidence).

### (c) Incident-symptom assertion
Incident symptom (F-2026-08-03-01/-02): "sign-in spins forever; works only after unregistering
the SW". Mapped assertions: post-deadline the dialog STOPS spinning and shows
`oauth.timeoutError` + `oauth.tryAgain` with polling ceased (`enabled:false`) — the "silent
infinite spin" symptom is now impossible (test: OAuthDialog.test.tsx, assertions 2/3). The
root-cause symptom ("works only after unregistering") is asserted live in Deploy verification:
SC2/SC3 sign-ins complete WITH the SW registered.

### task 301 — full gate suites (2026-08-06, run in tmux gates301)
```
clippy_exit=0
test_exit=0
f_lint=0
f_tsc=0
f_vitest=0
rf_lint=0
rf_tsc=0
rf_vitest=0
DONE
```
Log tails:
```

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

[2m   Duration [22m 18.08s[2m (transform 3.13s, setup 1.72s, import 14.86s, tests 12.06s, environment 16.62s)[22m

[2m   Duration [22m 25.57s[2m (transform 1.32s, setup 2.25s, import 7.08s, tests 20.73s, environment 19.44s)[22m

```
