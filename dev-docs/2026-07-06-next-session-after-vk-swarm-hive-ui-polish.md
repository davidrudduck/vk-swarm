# Next Session After vk-swarm-hive-ui-polish

**Status: IN-FLIGHT** — PR #457 needs `/wai:ship` to merge.

**Workstream:** vk-swarm-hive-ui-polish (remote-frontend error resilience + offline PWA)
**Current state:** PR #457 open, all gates green, code-review converged, `/wai:close` ran (spec still `status: active`), `/wai:ship` pending
**Branch:** `opencode/proud-panda`

---

## What Remains

The close ran but couldn't graduate the spec because it's still `status: active`. Run `/wai:ship vk-swarm-hive-ui-polish` to flip to `shipped`, re-close to graduate, and merge PR #457.

After that, three world-class improvements:

---

## Three World-Class Improvements

### 1. Real Electric Sync Integration Testing (Live Stack)

The E2E mock emits bare row JSON — D-L35/F-502 flags that the TanStack adapter may expect structured NDJSON change messages. A live integration test would:

- Spin up Docker Compose: PostgreSQL + Electric sync service + remote-frontend dev server
- Run a subset of the E2E suite against real Electric shape subscriptions (not mock)
- Verify: CRUD propagates across tabs, optimistic updates reconcile with server state, offline queue replays correctly against real API
- Catch: NDJSON format mismatches, reconnection edge cases, data type serialization bugs

**Deliverable:** `e2e/live-electric.spec.ts` + `docker-compose.e2e.yml` + documented setup in `docs/development/remote-frontend.mdx`

### 2. Accessibility Audit & Remediation (a11y)

The AlertDialog was shipped without an ARIA role until code review caught it. A systematic audit would:

- Run `@axe-core/playwright` in CI on every page (board, tasks, login, oauth callback)
- Audit focus management: does opening dialogs trap focus? Does closing them restore it?
- Verify keyboard navigation: Tab/Enter/Escape work on all interactive elements
- Check color contrast ratios against WCAG AA
- Add screen-reader landmarks: `<main>`, `<nav>`, `<aside>`, `aria-live` regions for sync status
- Add `skip-to-content` link

**Deliverable:** axe-core CI job + remediation of all violations + `docs/core-features/accessibility.mdx`

### 3. Lighthouse 100: Bundle Splitting + Performance

The PWA should feel instant even on slow connections. Current bundle likely ships as a single chunk. Optimizations:

- Route-based code splitting: `Tasks.tsx`, `Board.tsx`, `Login.tsx` in separate chunks via `React.lazy`
- Preload critical CSS: extract above-the-fold styles, inline in `index.html`
- Service worker: precache only shell routes (`/`, `/login`), runtime-cache the rest
- Add `web-vitals` instrumentation (LCP, FID, INP, CLS) with reporting
- Set up Lighthouse CI with budgets: Performance ≥ 95, PWA = 100, Accessibility ≥ 90
- Optimize Toast/Sonner bundle: tree-shake unused variants

**Deliverable:** route-level chunks, Lighthouse CI config, web-vitals reporting, `docs/development/performance.mdx`

---

## Prompt for the Next Session

```
I'm picking up the vk-swarm-hive-ui-polish workstream (remote-frontend PWA).

Current state:
- PR #457 is open at github.com/davidrudduck/vk-swarm/pull/457, all gates green, all
  CodeRabbit feedback resolved, code review converged. The spec is still status: active.
  Run /wai:ship vk-swarm-hive-ui-polish first to graduate, merge, and clean up.

After shipping, pick ONE of these three world-class improvements — whichever you can
make the most progress on in a single session — and implement it fully:

1. REAL ELECTRIC INTEGRATION: Add a live E2E test that runs against a real Electric
   sync service via Docker Compose. The current mock (mock-electric.ts) emits bare row
   JSON; a live test would catch NDJSON format mismatches, reconnection bugs, and data
   serialization issues the mock hides. Add docker-compose.e2e.yml + e2e/live-electric.spec.ts.
   See D-L35/F-502 in docs/plans/vk-swarm-hive-ui-polish/decisions-ledger.md for context.

2. ACCESSIBILITY AUDIT: Run @axe-core/playwright on every route, fix all violations,
   add focus management for dialogs, keyboard navigation, screen-reader landmarks,
   a skip-to-content link, and aria-live for sync status changes. Target WCAG AA.
   Add an axe-core CI job so a11y doesn't regress. Create docs/core-features/accessibility.mdx.

3. LIGHTHOUSE PERFORMANCE: Route-based code splitting with React.lazy, critical CSS
   inlining, optimized SW precache, web-vitals instrumentation, Lighthouse CI with
   budgets (Performance ≥ 95, PWA = 100). Create docs/development/performance.mdx.

Rules:
- Run the mandatory gates before finishing: tsc --noEmit, npm run lint, vitest run
- Update docs/docs.json sidebar if new docs are created
- Add decisions to docs/plans/vk-swarm-hive-ui-polish/decisions-ledger.md (D-L36+)
- If the session can't finish, create a dev-docs/workstreams/<name>/README.md
  follow-up workstream — never carry debt silently
```

---

## Key Files for Reference

| File | Purpose |
|------|---------|
| `docs/plans/vk-swarm-hive-ui-polish/decisions-ledger.md` | All 35 decisions + post-review known issues |
| `docs/plans/vk-swarm-hive-ui-polish/plan.md` | Original implementation plan |
| `docs/superpowers/specs/2026-07-05-vk-swarm-hive-ui-polish.md` | Spec (status: active) |
| `dev-docs/workstreams/vk-swarm-hive-ui-polish/README.md` | Workstream tracker |
| `remote-frontend/e2e/fixtures/mock-electric.ts` | Current Electric mock (target for improvement #1) |
| `remote-frontend/vite.config.ts` | PWA config, runtime caching rules |
| `remote-frontend/src/components/ui/alert-dialog.tsx` | Custom dialog (no Radix — improvement target for #2) |
| `remote-frontend/src/main.tsx` | App entry point (ErrorBoundary mount) |
| `remote-frontend/src/lib/pwa.ts` | Workbox service worker handler (redundant with autoUpdate) |
| `docs/core-features/error-resilience.mdx` | User-facing error resilience docs |
| `docs/development/remote-frontend.mdx` | Developer architecture docs |

## Mandatory Gates

```bash
cd remote-frontend && npx tsc --noEmit && npm run lint && npx vitest run
```