---
workstream: node-ui-localize-followups
status: shipped
shipped: 2026-08-07
created: 2026-08-05
parent_session: vk-swarm-node-ui-localize close-out
---

# node-ui-localize-followups

Low-risk leftovers from the shipped `vk-swarm-node-ui-localize` workstream. Batched because they
share one area and are individually trivial.

All tasks completed 2026-08-07 on branch `fix/frontend-cleanup-bundle`:

- `F-2026-07-31-04` — **fixed.** `LinkToLocalFolderDialog` deleted plus its API client method,
  hook mutation, and the Rust `POST /api/projects/link-local` route/handler and request type
  (verified unreferenced by remote-frontend); `shared/types.ts` regenerated
- `F-2026-07-31-05` — **fixed.** The stale `['mergedProjects']` invalidation was removed with
  the `linkLocalFolder` mutation above
- `F-2026-07-31-06` — **fixed.** Doc comment repointed to "project stats"
- `F-2026-07-31-08` — **fixed.** `settings.swarm.hiveNotConnected` added to en/ja/ko/es
- `F-2026-08-01-01` — **fixed.** `useDiffStream.test.ts` / `useRemoteConnectionStatus.test.ts`
  pin the discrimination: HiveNotConfigured 503 is quiet, plain 503 (outage) and 500 surface
- `F-2026-08-01-02` — **fixed.** `useAvailableNodes.test.ts` wrapper enables retries
  (`retry: 2, retryDelay: 0`) and asserts 1 call (suppressed) vs 3 calls (control)

Note on the last two: the 503 discrimination was materially changed during PR #467 review —
`isHiveNotConfigured` now requires the `HiveNotConfigured` message discriminator as well as status
503. Re-check these two against the new implementation before working them; they may be narrower
than filed.
