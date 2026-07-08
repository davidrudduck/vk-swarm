---
id: "001"
phase: 1
title: Add safety commit block to wai-ship.sh
status: ready
depends_on: []
parallel: false
conflicts_with: []
files:
  - ~/.claude/wai/wai-ship.sh
irreversible: false
scope_test: "N/A"
allowed_change: edit
covers_criteria: []
covers_tests: []
---

## Failing test (write first)

N/A — covered by existing tests: `~/.claude/wai/scripts/test_wai_ship.py` (task 002 adds
new tests for this change).

## Change

- **File:** `~/.claude/wai/wai-ship.sh` (WAI plugin scripts directory)
- **Anchor:** Between the already-shipped skip block (line 119: `exit 0`) and the spec
  resolution block (line 121: `# --- 4. Resolve THE spec`).
- **Before:**
  ```bash
  if [ "$WS_STATUS" = "shipped" ]; then
    echo "  workstream '$TOPIC' is already shipped (README status: shipped) — nothing to flip. Run /wai:close $TOPIC directly to (re)graduate." >&2
    echo "SHIP SKIPPED (already shipped)" >&2
    exit 0
  fi

  # --- 4. Resolve THE spec (exactly one) by workstream frontmatter under docs/superpowers/specs/ ---
  ```
- **After:**
  ```bash
  if [ "$WS_STATUS" = "shipped" ]; then
    echo "  workstream '$TOPIC' is already shipped (README status: shipped) — nothing to flip. Run /wai:close $TOPIC directly to (re)graduate." >&2
    echo "SHIP SKIPPED (already shipped)" >&2
    exit 0
  fi

  # --- Safety commit: save all outstanding work before graduation ---
  # The working tree may contain uncommitted changes from the workstream (code edits, config
  # tweaks, lockfiles, test fixtures) that the executor's per-task commit didn't capture.
  # Commit them NOW so nothing is lost when the branch merges. wai-close.sh's clean-index
  # guard (line 185) will see a clean index because this commit runs first.
  #
  # Detection: `git status --porcelain` covers both tracked modifications AND untracked files
  # (git diff --quiet only checks tracked files, missing new files). Empty output = clean.
  if [ -n "$(git status --porcelain 2>/dev/null)" ]; then
    git add -A \
      || fail "safety commit: git add -A failed (see output above)."
    git commit -q -m "ship($TOPIC): save outstanding work before graduation" \
      || fail "safety commit: git commit failed (see output above). No ship frontmatter edits have been made; note that 'git add -A' already staged the working tree."
    echo "  safety commit: all outstanding working-tree changes committed" >&2
  else
    echo "  safety commit: working tree clean, nothing to commit" >&2
  fi

  # --- 4. Resolve THE spec (exactly one) by workstream frontmatter under docs/superpowers/specs/ ---
  ```

## Allowed moves

Only the insertion of the safety commit block between lines 119 and 121. Do not modify any
other lines in the file. Do not move `fail()` — it is already defined at line 41, before the
insertion point.

## STOP triggers

- `fail()` not found at line 41 (would break the safety block's error paths).
- Already-shipped skip block not found at lines 114–119 (ordering invariant violated).
- Spec resolution block not found at line 121 (insertion anchor missing).
- `git status --porcelain` or `git add -A` already present in the file (duplicate block).

## Manual verification (record in decisions-ledger)

1. Read `~/.claude/wai/wai-ship.sh` and confirm the safety commit block appears between the
   already-shipped skip (`exit 0`) and the spec resolution (`# --- 4. Resolve THE spec`).
2. Confirm `fail()` is defined at line 41 (before the safety block).
3. Confirm no other lines were modified (diff should show only the insertion).

## Done when

`diff` of `~/.claude/wai/wai-ship.sh` shows only the safety commit block insertion between
the already-shipped skip and spec resolution. No other changes.
