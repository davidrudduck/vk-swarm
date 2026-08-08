---
workstream: worktree-orphan-sweep-guard
status: shipped
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

## Decisions ledger

- 2026-08-07 — **F-2026-07-30-03 fixed** (branch `fix/backend-hygiene-bundle`). The orphan sweep in
  `crates/local-deployment/src/container.rs` now calls `orphan_worktree_must_be_preserved()` before
  deleting: dirty worktrees are skipped with a warning, and an indeterminate git status also skips
  (matching `cleanup_expired_attempt`'s safety posture). **Deliberate divergence from the sibling as
  written**: `GitService::get_dirty_files` skips untracked files (it exists for stash display), which
  is inadequate for a data-loss guard. Added `GitService::has_uncommitted_changes()` (untracked
  inclusive, `git status --porcelain`) and switched **both** the orphan guard and
  `cleanup_expired_attempt` to it. Unit tests cover clean / modified / untracked / non-repo cases.
- 2026-08-07 — **F-2026-07-30-02 fixed** (same branch). `database_path()` now trims-then-filters its
  `VK_DATABASE_PATH` override exactly like `asset_dir()` (task 099); a set-but-blank value falls back
  to the default instead of relocating the DB to the CWD. The identically-shaped hole in
  `backup_dir()` / `VK_BACKUP_DIR` was fixed in the same commit. Tests cover blank, whitespace-only,
  and padded overrides.

- 2026-08-08 — **CodeRabbit PR #472 follow-up DECLINED**: suggestion to serialize all instance-registry
  mutations (`crates/utils/src/port_file.rs` `register`/`unregister_if_owner`) behind a per-project
  cross-process file lock. The registry under `/tmp/vibe-kanban/instances/` is an advisory discovery
  index for dev tooling (`pnpm run stop`), not a correctness-critical store: the ownership check in
  `unregister_if_owner` already eliminates the realistic failure (a long-dead instance's shutdown
  deleting its successor's record), and the residual TOCTOU requires a new instance to register in the
  microseconds between the old owner's read and delete during graceful shutdown. Worst case is a
  transiently missing registry record — self-healed by the next `register()` — and the stop tooling
  has a port-based `lsof` fallback for undiscoverable instances. A cross-process lock dependency in
  `utils` is disproportionate to that exposure; rationale preserved in the PR thread.

Status: both findings in this workstream are fixed; workstream complete pending merge.
