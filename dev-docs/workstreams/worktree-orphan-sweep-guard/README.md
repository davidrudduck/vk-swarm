---
workstream: worktree-orphan-sweep-guard
status: active
created: 2026-08-05
parent_session: vk-swarm-node-ui-localize close-out
---

# worktree-orphan-sweep-guard

Storage-safety fixes on destructive/relocating paths.

- `F-2026-07-30-03` — the orphan worktree sweep at
  `crates/local-deployment/src/container.rs:319-383` calls `remove_dir_all` with **no dirty-file
  guard**, unlike `cleanup_expired_attempt` in the same file. It can destroy uncommitted work.
  The sibling in the same file is the reference implementation — read it and justify any divergence.
- `F-2026-07-30-02` — an empty `VK_DATABASE_PATH` silently relocates the database to the process
  CWD (`crates/utils/src/assets.rs:61`). `asset_dir()` already trims-then-filters its override for
  exactly this reason (task 099); apply the same treatment.
