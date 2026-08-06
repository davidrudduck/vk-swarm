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
