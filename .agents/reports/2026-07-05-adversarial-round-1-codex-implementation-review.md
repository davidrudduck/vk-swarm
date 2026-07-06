I could not write the requested files because your prompt explicitly said “Do NOT modify any files” and the workspace is read-only. I did not change repository state. Below is the exact content for the requested markdown report and JSON findings file.

```markdown
# Adversarial Implementation Review — PR #457 vk-swarm-hive-ui-polish

Model: codex  
Scope: `git diff origin/main..HEAD`, focused on `remote-frontend/` implementation and E2E coverage.  
Write status: not written due explicit read-only/no-modification instruction.

## Findings

| ID | Severity | Issue | Evidence | Remediation |
|---|---|---|---|---|
| F001 | blocking | AuthGuard preserves `return_to` only as far as `/login`; the login flow drops it before OAuth init, so protected-route deep links return to `/nodes` instead of the original route. | `remote-frontend/src/components/AuthGuard.tsx:22` redirects to `/login?return_to=...`; `remote-frontend/src/AppRouter.tsx:42-45` always sends OAuth `return_to` as `${appBase}/oauth/callback`; `remote-frontend/src/AppRouter.tsx:107-108` only reads `return_to` from the callback URL, where this code never puts it. | In `LoginPage`, read `return_to` via `useSearchParams`, validate it with `isSafeReturnTo`, and include it in the callback URL passed to `initOAuth`, e.g. `${appBase}/oauth/callback?return_to=${encodeURIComponent(safeReturnTo)}`. Add E2E coverage for `/tasks -> /login?return_to=/tasks -> OAuth -> /tasks`. |
| F002 | blocking | Navbar sync dot is not connected to Electric updates. `TasksBoard` calls `markSynced()` on its own `useSyncStatus()` instance, while `Navbar` renders a separate hook instance, so the Navbar can go yellow/red even while live data is flowing. | `remote-frontend/src/pages/Tasks.tsx:47-52` calls `markSynced()` from a local hook instance; `remote-frontend/src/components/layout/Navbar.tsx:17-22` creates a different `useSyncStatus()` instance for the dot. | Move sync status into a shared provider/store, or have Electric collection hooks publish last-update time to a module-level subscription used by Navbar. Also mark sync completion for empty result sets, not only when `assignments.length > 0`. |
| F003 | blocking | Offline replay removes optimistic overlays immediately after API success, before Electric has delivered the new shape, so queued deletes can reappear and queued assignments can flip back to the old node during reconnect. | `remote-frontend/src/pages/Tasks.tsx:59-64` deletes optimistic entries right after `tasksApi.delete` / `tasksApi.setExecutingNode`; rendering still comes from `assignments` at `remote-frontend/src/pages/Tasks.tsx:135-164`. | Keep optimistic tombstones/assignment overlays until the live Electric data confirms the task is gone or `node_id` matches the queued node. Reconcile refs in an effect over `assignments`, not inside replay success. |
| F004 | blocking | The IndexedDB mutation queue can lose user actions. `enqueueMutation()` performs read-modify-write with separate `get` and `set`, and `replayMutations()` writes `remaining` after replay, overwriting mutations enqueued during replay. | `remote-frontend/src/lib/mutation-queue.ts:18-27` does non-atomic append; `remote-frontend/src/lib/mutation-queue.ts:34-48` snapshots queue then writes `remaining`. | Use `idb-keyval`’s `update()` or an explicit transaction/lock for atomic appends. During replay, remove only successfully replayed IDs from the current queue rather than replacing the whole queue snapshot. Add concurrent enqueue and enqueue-during-replay tests. |
| F005 | should-fix | PWA manifest references icons that are not present in the repo, which breaks installability checks and produces 404s. | `remote-frontend/vite.config.ts:41-48` references `/icons/icon-192.png` and `/icons/icon-512.png`; repository file listing contains no `remote-frontend/public/icons/...` assets. | Add `remote-frontend/public/icons/icon-192.png` and `icon-512.png`, or change the manifest to point at existing assets. Add a lightweight test or build check that manifest icon files exist. |

## Test Gaps

The current E2E auth tests only assert the default `/nodes` post-login path. `remote-frontend/e2e/auth.spec.ts:62-70` does not cover a protected deep link with `return_to`.

The Tasks unit tests acknowledge an unverified optimistic behavior. `remote-frontend/src/pages/Tasks.test.tsx:181-193` stops after asserting enqueue and leaves a TODO for whether the row reappears.

## Commands Run

Read-only inspection only:
- `git status -sb`
- `git diff --stat origin/main..HEAD`
- `git diff --name-only origin/main..HEAD`
- `nl -ba ...`
- `rg ...`
- `find ...`

No tests were run, to avoid repository state changes.
```

```json
{
  "model": "codex",
  "findings": [
    {
      "id": "F001",
      "severity": "blocking",
      "issue": "AuthGuard return_to is dropped by the OAuth login flow, so protected deep links return to /nodes instead of the original route.",
      "citation": "remote-frontend/src/components/AuthGuard.tsx:22; remote-frontend/src/AppRouter.tsx:42-45; remote-frontend/src/AppRouter.tsx:107-108 — guard writes return_to to /login, login sends a callback URL without that value, callback defaults to /nodes.",
      "remediation": "Read return_to in LoginPage, validate it, and include it in the OAuth callback URL passed to initOAuth; add E2E coverage for /tasks returning to /tasks after OAuth."
    },
    {
      "id": "F002",
      "severity": "blocking",
      "issue": "Navbar sync dot is disconnected from Electric updates because TasksBoard and Navbar use separate useSyncStatus instances.",
      "citation": "remote-frontend/src/pages/Tasks.tsx:47-52; remote-frontend/src/components/layout/Navbar.tsx:17-22 — markSynced updates a local TasksBoard hook instance, while Navbar renders another instance.",
      "remediation": "Move sync status to a shared provider/store or central Electric subscription used by Navbar; mark sync on empty result sets too."
    },
    {
      "id": "F003",
      "severity": "blocking",
      "issue": "Queued mutation replay clears optimistic overlays before Electric confirms the new data, causing deleted tasks or old assignments to reappear during reconnect.",
      "citation": "remote-frontend/src/pages/Tasks.tsx:59-64; remote-frontend/src/pages/Tasks.tsx:135-164 — replay success deletes optimistic refs while rendering still depends on stale live assignments.",
      "remediation": "Only clear optimistic deletion/assignment refs when the live Electric assignment list confirms deletion or the expected node_id."
    },
    {
      "id": "F004",
      "severity": "blocking",
      "issue": "Offline mutation queue can lose actions due non-atomic read-modify-write and replay overwriting new enqueues.",
      "citation": "remote-frontend/src/lib/mutation-queue.ts:18-27; remote-frontend/src/lib/mutation-queue.ts:34-48 — enqueue snapshots then sets the queue, replay later replaces the queue with a stale remaining array.",
      "remediation": "Use idb-keyval update() or a transaction/lock for atomic append, and remove replayed entries by ID from the current queue instead of replacing the snapshot."
    },
    {
      "id": "F005",
      "severity": "should-fix",
      "issue": "PWA manifest references missing icon files, breaking installability and causing 404s.",
      "citation": "remote-frontend/vite.config.ts:41-48 — manifest references /icons/icon-192.png and /icons/icon-512.png, but no matching remote-frontend/public/icons files are present.",
      "remediation": "Add the referenced icon files under remote-frontend/public/icons or update the manifest to existing assets; add a manifest asset existence check."
    }
  ]
}
```