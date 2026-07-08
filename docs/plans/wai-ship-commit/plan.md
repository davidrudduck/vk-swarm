# Plan: wai-ship-commit

## Approach

`/wai:ship` (via `wai-ship.sh`) currently delegates all git operations to `wai-close.sh`,
which only commits graduation artifacts. Outstanding working-tree changes from the workstream
are silently left behind.

The fix is a single surgical insertion in `wai-ship.sh`: a safety commit block that runs
`git add -A && git commit` when the working tree is dirty, placed **after** the `fail()`
definition and the already-shipped skip check, but **before** the spec resolution. This
ensures:

1. `fail()` is defined before the block calls it (kimi/deepseek adversarial finding).
2. Already-shipped re-runs exit immediately without creating a stranded commit (gpt-5.5 finding).
3. The safety commit precedes all frontmatter edits, so `wai-close.sh`'s clean-index guard sees
   a clean index.
4. The push is handled by `wai-close.sh`'s existing push logic — no new push code needed.

Tests are added to the existing `test_wai_ship.py` to verify the safety commit behavior
across all success criteria (SC1–SC5) and the already-shipped boundary case.

## Phases

### Phase 1: Implement safety commit in wai-ship.sh

One task: insert the safety commit block at the correct location in `wai-ship.sh`.

| ID | Title | dep: | conflicts: |
|----|-------|------|------------|
| 001 | Add safety commit block to wai-ship.sh | dep: - | conflicts: none |

### Phase 2: Add safety commit tests

One task: add tests to `test_wai_ship.py` covering dirty tree, clean tree, untracked files,
and already-shipped boundary.

| ID | Title | dep: | conflicts: |
|----|-------|------|------------|
| 002 | Add safety commit tests to test_wai_ship.py | dep: 001 | conflicts: none |
