---
workstream: local-deployment-test-orphan-cleanup-safety
doc_type: readme
status: ready
title: "Prevent local-deployment tests from sweeping real clean worktrees"
depends_on: []
adrs: []
staging_pointers:
  - docs/plans/local-node-browser-oauth/decisions-ledger.md
---

# local-deployment-test-orphan-cleanup-safety

**Origin:** discovered on 2026-08-22 during task 022's adversarial review for
`local-node-browser-oauth`. Split from that task because the remaining unsafe constructors predate
the browser-auth work and live in `crates/local-deployment/src/container.rs`, outside task 022's
declared file set.

## Finding

`LocalContainerService::new()` starts `cleanup_orphaned_worktrees`. Against a migrated test database,
real clean worktrees under the configured worktree base have no matching `task_attempts` rows and
are therefore eligible for deletion. The production dirty-worktree guard correctly preserves dirty
and untracked work, but clean developer worktrees are valid sweep targets.

Task 022 fixed the new exposure it introduced: `LocalDeployment::for_test()` and both direct
`LocalDeployment::from_parts()` tests now invoke one shared process-wide
`DISABLE_WORKTREE_ORPHAN_CLEANUP` guard before container construction. The pre-existing
`LocalContainerService::new_for_drain_test()` call sites remain exposed, so a complete
`cargo test -p local-deployment` can still sweep real clean worktrees.

## Evidence

The task-022 challenger reproduced the behavior with throwaway git worktrees under an isolated
`VK_WORKTREE_DIR`:

- the new direct-constructor test deleted a clean worktree before the task-022 guard fix;
- an equivalent run with `DISABLE_WORKTREE_ORPHAN_CLEANUP=1` preserved it;
- existing `new_for_drain_test()`-based container tests deleted clean worktrees even with the new
  task-022 test skipped;
- dirty worktrees survived, confirming this is test isolation rather than a regression in the
  shipped dirty-worktree protection.

## Required outcome

1. Make every local-deployment test constructor disable the production orphan sweep before it can
   start, preferably through one non-environment, dependency-injected test policy.
2. Add a regression test whose worktree base is an isolated temporary directory and which proves
   tests cannot remove a clean worktree outside their fixture ownership.
3. Remove or encapsulate the unsafe process-global environment mutation once all test constructors
   can receive the policy directly.
4. Verify focused container tests and the complete `cargo test -p local-deployment` suite without
   touching any pre-existing worktree.
