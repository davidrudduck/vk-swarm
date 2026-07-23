# Adversarial Review — Phase 3 App-UI-Kit Integration

**Reviewer:** OpenCode (DeepSeek-V4-Pro)
**Target:** branch `feat/vk-swarm-design-system`, Phase 3 diff (2394 lines)
**Date:** 2026-07-23
**Scope:** `remote-frontend/` only

---

## F1 (CRITICAL) — TaskStatus serialization mismatch silently drops InProgress/InReview tasks

**Evidence:**

- `crates/remote/src/db/tasks.rs:22-31` — the remote crate's `TaskStatus` enum uses `#[serde(rename_all = "kebab-case")]`:
  ```rust
  #[derive(Serialize, Deserialize)]
  #[serde(rename_all = "kebab-case")]
  pub enum TaskStatus {
      Todo,        // → "todo"        ✓
      InProgress,  // → "in-progress" ✗ (hyphen)
      InReview,    // → "in-review"   ✗ (hyphen)
      Done,        // → "done"        ✓
      Cancelled,   // → "cancelled"   ✓
  }
  ```

- `crates/remote/src/db/tasks.rs:79` — `SharedTask.status: TaskStatus` uses this enum for the `/v1/tasks/bulk` response payload.

- `remote-frontend/src/components/board/StatusBadge.tsx:4` — the TypeScript `TaskStatus` type has NO hyphens:
  ```ts
  export type TaskStatus = 'todo' | 'inprogress' | 'inreview' | 'done' | 'cancelled';
  ```

- `remote-frontend/src/pages/BoardPage.tsx:78,101` — `groupByStatus` creates a record keyed by the no-hyphen variants and uses `if (status in out)` to gate insertion:
  ```ts
  const out: Record<TaskStatus, Row[]> = { todo: [], inprogress: [], inreview: [], done: [], cancelled: [] };
  // ...
  if (status in out) {  // "in-progress" in out → false!
  ```

**Failure scenario:** The hive backend returns a task with `"status": "in-progress"` (from `InProgress` via kebab-case). The frontend receives `task.status = "in-progress"` (type `string`), casts it as `TaskStatus`, then checks `"in-progress" in { todo: [], inprogress: [], ... }` which is **false**. The task is silently dropped — it never appears in any column on the kanban board.

The `"in-review"` variant has the same problem. Only `todo`, `done`, and `cancelled` match.

**Note:** The separate `db` crate (`crates/db/src/models/task/mod.rs:27`) has its own `TaskStatus` with `#[serde(rename_all = "lowercase")]` (producing `inprogress`/`inreview`), but the `/v1/tasks/bulk` endpoint uses the **remote** crate's struct, not the db crate's. The frontend's TS types match the **db** crate's serialization, not the **remote** crate's.

**Speculative element:** If there is a server-side proxy (lightning/nginx) remapping the status values between the two crates, this finding would be false. The evidence at the source level (raw Rust derives) points strongly to a real mismatch. Check `crates/lightning/src/routes/electric-proxy.rs` or similar for any status-rewrite logic.

---

## F2 (IMPORTANT) — e2e mock-electric fixture still targets old `/api/electric` path

**Evidence:**

- `remote-frontend/e2e/fixtures/mock-electric.ts:14`:
  ```ts
  await page.route('**/api/electric/v1/shape/*', async (route) => {
  ```
- `remote-frontend/src/lib/electric/config.ts:14` (post-repoint):
  ```ts
  export const ELECTRIC_PROXY_BASE = '/v1/shape';
  ```

**Failure scenario:** All existing e2e tests using `mockElectricShape` (`board.spec.ts`, `cross-node.spec.ts`) will NOT intercept the actual Electric requests going to `/v1/shape/*`. The mock route pattern `**/api/electric/v1/shape/*` matches the **OLD** path only. Real Electric shape requests will hit the network (or fail) instead of being mocked. These e2e tests are silently broken.

**Confidence:** High. No regex/glob `*` in the old pattern will match `**/v1/shape/node_task_assignments` (the path has no `/api/electric/` segment).

---

## F3 (IMPORTANT) — Workbox runtimeCaching rule covers `/v1/shape` streaming, breaking PWA

**Evidence:**

- `remote-frontend/vite.config.ts:17-23`:
  ```ts
  {
    urlPattern: ({ url }) => url.pathname.startsWith('/v1/'),
    handler: 'NetworkFirst',
    options: {
      cacheName: 'api-cache',
      expiration: { maxEntries: 100, maxAgeSeconds: 300 },
    },
  },
  ```

**Failure scenario:** The `/v1/` prefix rule now matches `/v1/shape/nodes`, `/v1/shape/node_task_assignments`, etc. (the repointed Electric proxy base). Electric shape streaming uses chunked transfer encoding (SSE-like long-lived connections). Workbox's `NetworkFirst` strategy will:
1. Attempt to cache the streaming response body (partially or incorrectly)
2. On subsequent requests, potentially serve a stale/broken cached response
3. Break real-time task update streaming in PWA/offline mode

**Mitigation:** The `/v1/shape/*` paths should be **excluded** from the API cache (e.g., add a negative-pattern check or split the `/v1/` rule).

---

## F4 (IMPORTANT) — Theme toggle button is rendered but is a dead no-op

**Evidence:**

- `remote-frontend/src/AppRouter.tsx:118`:
  ```tsx
  <Navbar ... onToggleTheme={() => {}} />
  ```
- `remote-frontend/src/ui/chrome/Chrome.tsx:1517-1526` — `ThemeToggle` renders a styled icon button that calls `onToggle()` on click, with `aria-label="Toggle theme"` and a `title` tooltip.

**Failure scenario:** The "Toggle theme" button appears in the Chrome Navbar beside the Settings icon. Clicking it does nothing — no visual feedback, no error. This is a **user-facing dead control** in every production page.

**Severity note:** Marked Important (not Critical) because it's cosmetic/non-functional rather than data-loss. But it's user-facing and represents a visibly broken feature from day one.

---

## F5 (IMPORTANT) — TaskDrawer footer buttons (Merge/Rebase/Open in IDE) are all dead clicks

**Evidence:**

- `remote-frontend/src/ui/panels/TaskDrawer.tsx:103-112`:
  ```tsx
  <Button variant="primary" size="sm" style={{ flex: 1 }}>Merge</Button>
  <Button variant="outline" size="sm">Rebase</Button>
  <Button variant="ghost" size="sm">Open in IDE</Button>
  ```
  None of these `<Button>` elements have an `onClick` handler.

**Failure scenario:** User opens a TaskDrawer from the board. The footer renders three attractive action buttons. Clicking any of them does nothing. This is a **user-facing dead control** and an expectation gap — the Merge button in particular is prominent (`variant="primary"`).

---

## F6 (MINOR) — TaskDrawer diff/logs/attempts tabs render ambiguous empty states

**Evidence:**

- `remote-frontend/src/pages/BoardPage.tsx:57-61` — TaskDrawer is mounted with no `diffLines`, `logs`, or `attempts` props:
  ```tsx
  <TaskDrawer
    task={selected?.task ?? null}
    status={selected?.status ?? 'todo'}
    onClose={() => setSelected(null)}
  />
  ```
- `remote-frontend/src/ui/panels/TaskDrawer.tsx:28` — defaults to empty arrays:
  ```ts
  export function TaskDrawer({ ..., diffLines = [], logs = [], attempts = [] })
  ```

**Failure scenario:** User opens a task from the board. The "Diff" tab shows an empty console panel (no rows, just the styled background border). The "Logs" tab shows an empty console panel. The "Attempts" tab shows an empty list. There is no visual distinction between "data has not been fetched yet" and "no diff/logs/attempts exist." The user may interpret this as "there is no diff" when in fact the data simply wasn't loaded.

---

## F7 (MINOR) — NodesPage.test.tsx mocks use wrong Node shape (os_info/hostname)

**Evidence:**

- `remote-frontend/src/pages/NodesPage.test.tsx:7-8`:
  ```ts
  { id: 'n1', name: 'justX', os_info: 'mac', status: 'online', ..., hostname: 'h', public_url: 'u' },
  ```
- `remote-frontend/src/pages/NodesPage.tsx:58` — real code reads `n.capabilities?.os`:
  ```ts
  const os = n.capabilities?.os?.toLowerCase() ?? '';
  ```
- `remote-frontend/src/types/nodes.ts:18-31` — the real `Node` interface has `capabilities: NodeCapabilities` (with `os: string`) and no `os_info` or `hostname` fields.

**Impact:** The mock uses `os_info` (non-existent field). Since `n.capabilities` is `undefined` in the mock, `mapNodeToRow` defaults OS to `'linux'` for all nodes. The test happens to pass because it only asserts node names, not OS rendering. This test would NOT catch a regression in OS detection.

**Recommended fix:** Change `os_info: 'mac'` to `capabilities: { os: 'mac' }` and remove `hostname`.

---

## F8 (NOTE) — Existing orphaned files + deriveViewFromLocation

- **Orphaned files** (`pages/Nodes.tsx`, `pages/Tasks.tsx`, `components/layout/*`): These directories/pages do **not** exist on disk. They were already removed in a prior phase (the diff declares them as forward-looking ledger entries). No stale imports into production bundles found. **Not a defect.**

- **`deriveViewFromLocation('/')='board'` latent mismatch** (`AppRouter.tsx:101`): `'/'` is mapped to `'board'`, but `'/'` is served by `RootRedirect` (which redirects to `/nodes` or `/login`) and never mounts `ChromeLayout`. The `'/'` branch of `deriveViewFromLocation` is unreachable at runtime. **Harmless.**

---

## F9 (NOTE) — `NodeApiKeySection` imports from `@/lib/api` barrel vs NodesPage imports from submodule

`remote-frontend/src/components/swarm/NodeApiKeySection.tsx:27` imports `nodesApi` from `@/lib/api` (the barrel export). `remote-frontend/src/pages/NodesPage.tsx:3` imports from `@/lib/api/nodes` (the submodule). Both resolve the same module (`nodesApi`) but through different paths. This is a maintainability concern (not a bug) — see `AppRouter.test.tsx` having to mock both import paths separately. If the barrel re-exports change, one path could break without the other.

---

## F10 (NOTE) — `Node` mock in NodesPage.test.tsx missing required fields

The mock nodes at `NodesPage.test.tsx:7-8` are missing `machine_id` and `organization_id` (both non-optional in the `Node` interface). The fetch mock bypasses TypeScript checking (returns `Response`), so this doesn't cause a compile error. These fields aren't consumed by `mapNodeToRow`, so the test passes. However, if future code accesses these fields, the test mock will produce `undefined` rather than a realistic value.

---

## F11 (NOTE) — `NodeApiKeySection` `TooltipProvider` renders as `<div>` wrapper

`NodeApiKeySection.tsx:316` has a root-level `<TooltipProvider>` wrapping the `<Card>`. Radix UI `TooltipProvider` renders as its direct child without wrapping in a `<div>`, but in React, the return statement for `NodeApiKeySection` is:

```tsx
  if (!organizationId) return null;
  return (
    <TooltipProvider>
      <Card>...</Card>
    </TooltipProvider>
  );
```

This is fine — `TooltipProvider` is a context provider, not a DOM-wrapper. No issue. This was investigated for the "error boundary swallowing" concern; the `ErrorBoundary` with `fallback={null}` at `NodesPage.tsx:41` will catch render errors from `NodeApiKeySection` including its child components. The `componentDidCatch` logs to `console.error` (`ErrorBoundary.tsx:26`), which is standard practice. **No swallowing defect.**

---

## Verdict: FIX-FIRST

**Blockers (must fix before merge):**

| ID | Severity | Summary |
|----|----------|---------|
| F1 | CRITICAL | Tasks with status `InProgress` or `InReview` silently dropped from board due to kebab-case serialization mismatch |

**Should fix (strongly recommended):**

| ID | Severity | Summary |
|----|----------|---------|
| F2 | IMPORTANT | e2e mock-electric fixture targets old `/api/electric/v1/shape/*` path; e2e tests using it are broken |
| F3 | IMPORTANT | Workbox `NetworkFirst` caches `/v1/shape/*` streaming responses; breaks real-time updates in PWA |
| F4 | IMPORTANT | Theme toggle button is dead no-op; user-facing broken control |
| F5 | IMPORTANT | TaskDrawer Merge/Rebase/Open in IDE buttons are dead clicks |

**Non-blocking (acceptable to defer):**

| ID | Severity | Summary |
|----|----------|---------|
| F6 | MINOR | TaskDrawer empty tabs don't distinguish "not loaded" from "no data" |
| F7 | MINOR | NodesPage.test.tsx mock uses wrong Node shape |