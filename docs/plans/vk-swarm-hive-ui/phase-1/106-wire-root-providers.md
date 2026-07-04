---
id: "106"
phase: 1
title: "Hive app shell: wire root providers (ProfileProvider + QueryClientProvider + i18n if present)"
status: ready
depends_on: ["101", "102", "105"]
parallel: false
conflicts_with: []
files:
  - remote-frontend/src/App.tsx
  - remote-frontend/src/main.tsx
  - remote-frontend/src/App.test.tsx
irreversible: false
scope_test: "remote-frontend/src/App.test.tsx"
allowed_change: edit
covers_criteria: [SC3]
---
## Failing test (write first)
File: `remote-frontend/src/App.test.tsx`

Renders `<App />`, mocks `fetch` for `/profile` (200), and asserts:
1. `ProfileProvider` context is available (a consumer hook returns `isSignedIn: true`).
2. `QueryClientProvider` context is available (a `useQuery` consumer returns cached data).
3. The router renders (a link to `/nodes` is present, or the redirect to `/nodes` fires).

## Change
- **File:** `remote-frontend/src/App.tsx` (EDIT — finalize)
  - **Anchor:** the provider wrapping from task 105.
  - **Before:** task 105 wrapped `<AppRouter />` in `<ProfileProvider>` and `<QueryClientProvider>`.
  - **After:** ensure the provider order is correct: outermost `QueryClientProvider` (so `ProfileProvider` could use queries if needed), then `ProfileProvider`, then `AppRouter`. If `remote-frontend/` uses i18n (check for `i18next` in `package.json` and an `i18n.ts` setup file), wrap in `I18nextProvider` outermost. If no i18n setup exists, skip it (do NOT add i18n — it's out of scope for this workstream; the hive shell is English-only for now, record in ledger).
  - Confirm `QueryClient` is instantiated once at module scope (not per-render): `const queryClient = new QueryClient({ defaultOptions: { queries: { staleTime: 30_000 } } })` or similar — record the chosen options in the ledger.

- **File:** `remote-frontend/src/main.tsx` (EDIT — if it exists)
  - **Anchor:** the current ReactDOM render call.
  - **Before:** renders `<App />` or `<AppRouter />` directly.
  - **After:** renders `<App />` (which now owns the provider tree). If `main.tsx` currently wraps `AppRouter` directly, switch it to `App`. If `main.tsx` imports `AppRouter` directly, fix the import to go through `App`.

## Allowed moves
- Edit `remote-frontend/src/App.tsx` and `remote-frontend/src/main.tsx`.
- Add `@tanstack/react-query` to `remote-frontend/package.json` if task 105 didn't already.

## STOP triggers
- If `remote-frontend/src/main.tsx` does not exist (the entry point is elsewhere) — find the actual entry (check `remote-frontend/index.html` for the script src) and edit that file instead; record the file path in the ledger.
- If i18n is already set up in `remote-frontend/` but with a different provider than `I18nextProvider` — STOP and record the actual provider; do not silently swap.

## Manual verification (record in decisions-ledger)
- `cd remote-frontend && npx vitest run src/App.test.tsx` exits 0.
- `cd remote-frontend && npx tsc --noEmit` exits 0.
- `cd remote-frontend && npm run lint` exits 0.
- `cd remote-frontend && npm run build` exits 0 (the app builds — this is the phase 1 integration smoke).

## Done when
`WAI_TYPECHECK_CMD="cd remote-frontend && npx tsc --noEmit" WAI_TEST_CMD="cd remote-frontend && npx vitest run src/App.test.tsx" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-hive-ui 106` exits 0