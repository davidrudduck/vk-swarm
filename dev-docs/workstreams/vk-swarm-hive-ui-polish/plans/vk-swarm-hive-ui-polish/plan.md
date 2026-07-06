# Plan: vk-swarm-hive-ui-polish

Spec: `docs/superpowers/specs/2026-07-05-vk-swarm-hive-ui-polish.md` (frozen, sha `39d014445ff72b6a833abc11d1cb61d72dd4b211`).
Workstream: `dev-docs/workstreams/vk-swarm-hive-ui-polish/README.md`.
Parent workstream: `vk-swarm-hive-ui` (shipped).

## Approach

The `vk-swarm-hive-ui` workstream delivered a working hive-hosted management console but it's a prototype — no error resilience, no offline support, zero E2E tests. This plan hardens it to production-quality across three independent improvements that touch different subsystems.

The approach is strictly layered: Error Resilience first (it's cheapest and protects against user-visible breakage), then Offline-First PWA (builds on a stable error-free app), then Playwright E2E Test Suite (exercises the hardened app). Each improvement has its own independent gate — if one is completed before the others, it ships without waiting.

All work is in `remote-frontend/` only. The node frontend (`frontend/`) must stay byte-for-byte unchanged (SC4 enforced mechanically: every phase-3 task chains `cd frontend && npx tsc --noEmit` in its Done-when gate). No new backend changes required — all routes already exist (`/v1/profile`, `/v1/oauth/web/*`, `/v1/nodes`, `/v1/tasks/*`, `/v1/api/electric/v1/shape`).

The tasks are surgical — each touches exactly the files it needs. `sonner` for toasts, `vite-plugin-pwa` + `workbox-window` + `idb-keyval` for PWA, `@playwright/test` + `msw` for E2E. No other new dependencies.

SC4 guard is non-negotiable: the node frontend (`frontend/`) must continue to compile, lint, and test green. Every Done-when gate for phase-3 tasks includes `cd ../frontend && npx tsc --noEmit && npm run lint && npx vitest run`.

## Phases

### Phase 1 — Error Resilience Layer (SC1-SC5, US1-US5)

Install sonner, add a root ErrorBoundary, add AuthGuard route protection, wire toast feedback + confirmation dialogs + loading states into Tasks.tsx. This is the cheapest fix and protects against the three most user-visible breakage classes: white-screen crashes, broken signed-out pages, and silent mutation failures.

| ID  | Title                                        | SCs           | dep:         | conflicts: |
| ---:| -------------------------------------------- | ------------- | ------------ | ---------- |
| 100 | Install sonner + create toast wrapper        | SC3 SC4       | dep: -       | conflicts: - |
| 101 | Create ErrorBoundary component               | SC1           | dep: -       | conflicts: - |
| 102 | Create AuthGuard + wire into AppRouter       | SC2           | dep: -       | conflicts: - |
| 103 | Wire toasts + dialogs + loading into Tasks   | SC3 SC4 SC5   | dep: 100     | conflicts: 205 |
| 104 | Mount ErrorBoundary + Toaster in root        | SC1 SC2 SC3   | dep: 100 101 102 | conflicts: - |

### Phase 2 — Offline-First PWA (SC6-SC11, US6-US10)

Configure vite-plugin-pwa with Workbox strategies, add useOnlineStatus hook + reconnect banner, build optimistic mutation helpers for Electric collections, add sync status indicator in Navbar, implement IndexedDB-backed offline mutation queue, wire PWA features into Tasks.tsx.

| ID  | Title                                        | SCs           | dep:         | conflicts: |
| ---:| -------------------------------------------- | ------------- | ------------ | ---------- |
| 200 | Install vite-plugin-pwa + configure manifest | SC6 SC11      | dep: 104     | conflicts: - |
| 201 | Create useOnlineStatus + reconnect banner    | SC7           | dep: 104     | conflicts: - |
| 202 | Create optimistic mutation helpers           | SC8           | dep: 104     | conflicts: - |
| 203 | Create sync status indicator in Navbar       | SC9           | dep: 104     | conflicts: 204 |
| 204 | Create offline mutation queue (idb-keyval)   | SC10          | dep: 200 203 | conflicts: 203 |
| 205 | Wire PWA features into Tasks.tsx             | SC8 SC10      | dep: 103 202 204 | conflicts: 103 |

### Phase 3 — Playwright E2E Test Suite (SC12-SC16, US11-US12)

Install @playwright/test + msw, create playwright.config.ts with MSW API mocks, write 4 spec files (auth, board, cross-node, SC4-guard). Each Done-when gate chains the SC4 guard.

| ID  | Title                                        | SCs           | dep:         | conflicts: |
| ---:| -------------------------------------------- | ------------- | ------------ | ---------- |
| 300 | Install Playwright + config + MSW fixtures   | SC16          | dep: 104     | conflicts: - |
| 301 | Write auth.spec.ts (OAuth PKCE flow)         | SC12          | dep: 300     | conflicts: - |
| 302 | Write board.spec.ts (kanban board)           | SC13          | dep: 300 301 | conflicts: - |
| 303 | Write cross-node.spec.ts (multi-node data)   | SC14          | dep: 300 302 | conflicts: - |
| 304 | Write sc4-guard.spec.ts (SC4 globalSetup)    | SC15          | dep: 300     | conflicts: - |