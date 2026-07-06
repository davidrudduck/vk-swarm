My plan is complete. Here's a summary of the 10 findings across 4 severities:

**BLOCKING (4):**
- **F001** — `useSyncStatus()` is instantiated independently in both `Navbar` and `Tasks.tsx`. The `markSynced()` call in Tasks writes to a private `useRef` that has no connection to the Navbar's sync dot. The Navbar's green/yellow/red indicator never reflects actual data arrival.
- **F002** — `setupTaskApiMocks` is imported but never called in `board.spec.ts`. With `eslint --max-warnings 0` and the `unused-imports` plugin, this is a CI-breaking lint error.
- **F003** — The E2E logout test asserts `localStorage.access_token === null`, but `handleLogout` in `Navbar.tsx` only calls `oauthApi.logout()` then `window.location.reload()` — it never clears the token. Test will fail.
- **F004** — `replayMutations` has no concurrency guard. If `isOnline` transitions true twice rapidly (or component remounts during replay), two concurrent calls read the same queue and each writes back an independent `remaining` array, corrupting queue state.

**SHOULD-FIX (4):**
- **F005** — `AuthGuard` redirects to `/login?return_to=…` but `LoginPage` never reads `return_to` when building the OAuth redirect. Users always land at `/nodes` after login, ignoring their original destination.
- **F006** — `idb-keyval` mock returns `0` instead of `undefined` for missing keys. Tests pass accidentally because `0` is falsy, but the mock type is wrong.
- **F007** — `optimistic.ts` (`optimisticDelete`/`optimisticUpdate`) is entirely unused — no importer. Tasks.tsx uses `useRef<Set>/<Map>` directly. Dead scaffolding that misleads future maintainers.
- **F008** — `cross-node.spec.ts` "TaskDetail shows correct node_id label" is hollow: `TaskDetail` shows logs/events (both empty in mock), so the test passes only because "node-alpha" is visible in the board `<li>`.

**INFO (2):**
- **F009** — `wasOffline` exported from `useOnlineStatus` but never consumed by any component.
- **F010** — `handleDelete` is declared `async` despite containing no `await`.

Ready to write the report files. Approve the plan to proceed.