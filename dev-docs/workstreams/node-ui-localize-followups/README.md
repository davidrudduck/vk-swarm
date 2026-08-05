---
workstream: node-ui-localize-followups
status: active
created: 2026-08-05
parent_session: vk-swarm-node-ui-localize close-out
---

# node-ui-localize-followups

Low-risk leftovers from the shipped `vk-swarm-node-ui-localize` workstream. Batched because they
share one area and are individually trivial.

- `F-2026-07-31-04` — `LinkToLocalFolderDialog` orphaned by task 302; its API client, hook, and
  server route are still live
- `F-2026-07-31-05` — stale `['mergedProjects']` query key invalidated in
  `frontend/src/hooks/useProjectMutations.ts:79` is now a no-op (verified still present 2026-08-05)
- `F-2026-07-31-06` — stale doc comment "merged projects view" at
  `crates/db/src/models/project/mod.rs:106` (verified still present)
- `F-2026-07-31-08` — i18n key `settings.swarm.hiveNotConnected` undefined in all locales;
  ja/ko/es fall back to English
- `F-2026-08-01-01`, `F-2026-08-01-02` — 503-discrimination and retry behaviour are unpinned; an
  unconditional guard would survive the suite

Note on the last two: the 503 discrimination was materially changed during PR #467 review —
`isHiveNotConfigured` now requires the `HiveNotConfigured` message discriminator as well as status
503. Re-check these two against the new implementation before working them; they may be narrower
than filed.
