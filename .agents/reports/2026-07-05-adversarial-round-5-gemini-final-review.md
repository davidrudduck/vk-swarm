```json
{
  "model": "gemini",
  "findings": [
    {
      "id": "F-001",
      "severity": "HIGH",
      "issue": "Race condition in offline mutation queue causes data loss. `replayMutations` reads the queue, yields to async `execute()`, and then overwrites the queue with `remaining`, completely dropping any new mutations added concurrently via `enqueueMutation`.",
      "citation": "remote-frontend/src/lib/mutation-queue.ts:40 — `await set(QUEUE_KEY, remaining);` clobbers concurrent enqueues.",
      "remediation": "In `replayMutations`, re-read the queue before writing `remaining`, or use a lock/transaction that wraps the entire read-execute-write cycle."
    },
    {
      "id": "F-002",
      "severity": "HIGH",
      "issue": "The `return_to` login flow is broken. `AuthGuard` correctly redirects to `/login?return_to=...`, but `LoginPage` ignores URL parameters and hardcodes the callback return URL. Users are always redirected to `/nodes` after login, violating US2/SC2.",
      "citation": "remote-frontend/src/AppRouter.tsx:40 — `const returnTo = ${appBase}/oauth/callback` ignores `location.search`.",
      "remediation": "In `LoginPage`, read `return_to` from `useSearchParams()` and append it to the `returnTo` callback URL passed to `initOAuth`."
    },
    {
      "id": "F-003",
      "severity": "HIGH",
      "issue": "E2E test for SC14c is hollow and falsely passes. `cross-node.spec.ts` asserts `node-alpha` is visible, which passes because the list item on the board contains the text. `mockElectricShape` doesn't supply `task_output_logs`, so `TaskDetail` actually renders 'No activity yet'.",
      "citation": "remote-frontend/e2e/cross-node.spec.ts:32 — `await expect(page.locator('text=node-alpha')).toBeVisible();` matches the parent `li`.",
      "remediation": "Update `mockElectricShape` to supply mock `task_output_logs` and scope the assertion to the `TaskDetail` container."
    },
    {
      "id": "F-004",
      "severity": "MEDIUM",
      "issue": "TasksBoard uses a single shared `selectedNodeId` state across all four status columns. Changing the node dropdown in the 'Pending' column changes the dropdowns in all other columns, causing unexpected assignment targets.",
      "citation": "remote-frontend/src/pages/Tasks.tsx:125 — `<select value={selectedNodeId}...>` binds all four column dropdowns to one variable.",
      "remediation": "Change `selectedNodeId` to a `Record<string, string>` mapping column status to node ID, or use a single global dropdown above the columns."
    },
    {
      "id": "F-005",
      "severity": "MEDIUM",
      "issue": "Missing concurrency guard for mutation buttons enables double-clicks. `isAssigning` is a `string | null`. Assigning Task B overwrites `isAssigning`, immediately re-enabling the button for Task A while its API call is still pending.",
      "citation": "remote-frontend/src/pages/Tasks.tsx:84 — `setIsAssigning(taskId);` overwrites any previous pending task ID.",
      "remediation": "Change `isAssigning` and `isDeleting` to a `Set<string>`, or block all buttons while any mutation is pending."
    },
    {
      "id": "F-006",
      "severity": "MEDIUM",
      "issue": "Missing SPA shell caching strategy. SC6 explicitly mandated `stale-while-revalidate for SPA shell (/, /login, /oauth/callback)`, but `vite.config.ts` only configures `NetworkFirst` for `/v1/` and `CacheFirst` for `/assets/`.",
      "citation": "remote-frontend/vite.config.ts:16-30 — `runtimeCaching` array omits the requested SPA shell rule.",
      "remediation": "Add a `StaleWhileRevalidate` rule matching the SPA shell paths to `runtimeCaching`."
    },
    {
      "id": "F-007",
      "severity": "LOW",
      "issue": "Fake 'Undo' button on task deletion. The implementation immediately dispatches the DELETE request to the server, and the Undo button merely displays a 'not available' message, failing the intent of SC4.",
      "citation": "remote-frontend/src/pages/Tasks.tsx:112 — `await tasksApi.delete(taskId);` executes before the toast is shown.",
      "remediation": "Implement an actual delay/timeout before executing `tasksApi.delete()`, or remove the hardcoded fake Undo action."
    },
    {
      "id": "F-008",
      "severity": "LOW",
      "issue": "Navbar queue badge renders just the number, not the requested text. SC10 explicitly required 'Navbar shows queue badge (\"N pending\")'.",
      "citation": "remote-frontend/src/components/layout/Navbar.tsx:48 — `{queueLength}` renders the number alone.",
      "remediation": "Change to `{queueLength} pending`."
    }
  ]
}
```