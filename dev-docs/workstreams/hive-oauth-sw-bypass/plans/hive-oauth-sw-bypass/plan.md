# hive-oauth-sw-bypass Plan

## Spec
docs/superpowers/specs/2026-08-05-hive-oauth-sw-bypass.md

## Approach
Three small, independently testable changes, sequenced as a tracer bullet. Phase 1 adds the `/v1/oauth` exclusion INLINE to the hive SW's api-cache urlPattern in `remote-frontend/vite.config.ts` (Workbox generateSW serializes the arrow via toString(), so the predicate must be self-contained — no imported identifiers), and pins it with a pure mirror module `swCachePredicate.ts` under a failing-first vitest plus drift-guard greps. Phase 2 gives the node's OAuthDialog a bounded polling deadline and a localized timeout error, reusing the error branch's existing `oauth.tryAgain` retry button. Phase 3 runs the full repo gates (exit 0 required) and records the live deploy verification: rebuilt sw.js on the running hive, empty api-cache for `/v1/oauth`, real sign-ins with the SW registered, and a stalled-flow timeout observed on the running node.


## Phases
- **Phase 1: sw-cache-exclusion** — The hive SW's runtime api-cache predicate excludes /v1/oauth inline (generateSW-safe), mirrored and pinned by a unit-tested pure module, so the built sw.js contains the exclusion.
- **Phase 2: oauth-dialog-deadline** — OAuthDialog stops polling after a bounded deadline and renders a visible, localized timeout error; the existing tryAgain button is the retry affordance.
- **Phase 3: gates-and-live-verify** — Full repo gates green (exit 0); live deploy verification evidence recorded in the decisions ledger.

## Tasks

| id | phase | title | deps | conflicts |
|---|---:|---|---|---|
| 101 | 1 | Create pure SW cache predicate mirror module with /v1/oauth exclusion, pinned by failing-first vitest | dep: none | conflicts: none |
| 102 | 1 | Add /v1/oauth exclusion inline to the vite.config.ts api-cache urlPattern (generateSW-safe) | dep: 101 | conflicts: none |
| 201 | 2 | Add oauth.timeoutError i18n key to all four locales | dep: none | conflicts: none |
| 202 | 2 | OAuthDialog: bounded polling deadline with localized timeout error (existing tryAgain = retry) | dep: 201 | conflicts: none |
| 301 | 3 | Full repo gates (exit 0) + live deploy verification evidence (SW registered sign-ins, cache inspection, stalled-flow timeout) | dep: 102 202 | conflicts: none |
