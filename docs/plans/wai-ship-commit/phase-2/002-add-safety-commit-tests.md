---
id: "002"
phase: 2
title: Add safety commit tests to test_wai_ship.py
status: ready
depends_on: ["001"]
parallel: false
conflicts_with: []
files:
  - ~/.claude/wai/scripts/test_wai_ship.py
irreversible: false
scope_test: "N/A"
allowed_change: edit
covers_criteria: [SC1, SC2, SC3, SC4, SC5]
covers_tests: []
---

## Failing test (write first)

Tests to be added to `~/.claude/wai/scripts/test_wai_ship.py` after the last existing test
function, before the `if __name__ == "__main__":` block:

```python
def test_safety_commit_on_dirty_tree():
    """SC1+SC2: dirty working tree → safety commit created before graduation commit."""
    d = _repo()
    _spec(d, "safecommit"); _unit(d, "safecommit"); _readme(d, "safecommit")
    _write(d, "src/new_feature.py", "def hello(): pass\n")
    p = _run(d, "safecommit")
    assert p.returncode == 0, p.stderr
    log = subprocess.run(["git", "log", "--oneline"], cwd=d, capture_output=True, text=True).stdout
    assert "save outstanding work before graduation" in log, f"safety commit missing from log: {log}"
    lines = [l for l in log.strip().split("\n") if l]
    safety_idx = next(i for i, l in enumerate(lines) if "save outstanding work" in l)
    grad_idx = next(i for i, l in enumerate(lines) if "graduate staged docs" in l)
    assert safety_idx < grad_idx, f"safety commit ({safety_idx}) must precede graduation ({grad_idx})"
    status = subprocess.run(["git", "status", "--porcelain"], cwd=d, capture_output=True, text=True).stdout
    assert status.strip() == "", f"working tree not clean after ship: {status}"

def test_safety_commit_noop_on_clean_tree():
    """SC4: clean working tree → no safety commit (idempotent)."""
    d = _repo()
    _spec(d, "cleantree"); _unit(d, "cleantree"); _readme(d, "cleantree")
    p = _run(d, "cleantree")
    assert p.returncode == 0, p.stderr
    log = subprocess.run(["git", "log", "--oneline"], cwd=d, capture_output=True, text=True).stdout
    assert "save outstanding work" not in log, f"safety commit should NOT appear on clean tree: {log}"

def test_safety_commit_includes_untracked_files():
    """SC5: untracked files are included in the safety commit via git add -A."""
    d = _repo()
    _spec(d, "untracked"); _unit(d, "untracked"); _readme(d, "untracked")
    _write(d, "brand_new_file.txt", "new content\n")
    p = _run(d, "untracked")
    assert p.returncode == 0, p.stderr
    safety_hash = subprocess.run(["git", "log", "--all", "--grep=save outstanding work",
                                   "--format=%H"], cwd=d, capture_output=True, text=True).stdout.strip()
    assert safety_hash, "safety commit not found"
    diff = subprocess.run(["git", "diff-tree", "--no-commit-id", "-r", "--name-only", safety_hash],
                          cwd=d, capture_output=True, text=True).stdout
    assert "brand_new_file.txt" in diff, f"untracked file missing from safety commit: {diff}"

def test_already_shipped_no_safety_commit():
    """Already-shipped re-run → no safety commit (even with dirty tree)."""
    d = _repo()
    _readme(d, "alreadyship", status="shipped"); _spec(d, "alreadyship", status="shipped")
    _write(d, "dirty_file.txt", "should not be committed\n")
    p = _run(d, "alreadyship")
    assert p.returncode == 0, p.stderr
    assert "SHIP SKIPPED" in p.stderr, f"expected SKIPPED: {p.stderr}"
    log = subprocess.run(["git", "log", "--oneline"], cwd=d, capture_output=True, text=True).stdout
    assert "save outstanding work" not in log, f"safety commit must not run on already-shipped: {log}"
```

## Change

- **File:** `~/.claude/wai/scripts/test_wai_ship.py`
- **Anchor:** After the last test function (`test_idempotent_guard_is_non_vacuous`) and before
  the `if __name__ == "__main__":` block.
- **Before:**
  ```python
  if __name__ == "__main__":
  ```
- **After:**
  ```python
  # --- safety commit tests (wai-ship-commit) ---

  def test_safety_commit_on_dirty_tree():
      """SC1+SC2: dirty working tree → safety commit created before graduation commit."""
      d = _repo()
      _spec(d, "safecommit"); _unit(d, "safecommit"); _readme(d, "safecommit")
      # Create an uncommitted file (dirty working tree)
      _write(d, "src/new_feature.py", "def hello(): pass\n")
      p = _run(d, "safecommit")
      assert p.returncode == 0, p.stderr
      log = subprocess.run(["git", "log", "--oneline"], cwd=d, capture_output=True, text=True).stdout
      assert "save outstanding work before graduation" in log, f"safety commit missing from log: {log}"
      # Verify the safety commit comes before the graduation commit
      lines = [l for l in log.strip().split("\n") if l]
      safety_idx = next(i for i, l in enumerate(lines) if "save outstanding work" in l)
      grad_idx = next(i for i, l in enumerate(lines) if "graduate staged docs" in l)
      assert safety_idx < grad_idx, f"safety commit ({safety_idx}) must precede graduation ({grad_idx})"
      # SC1: working tree is clean after ship
      status = subprocess.run(["git", "status", "--porcelain"], cwd=d, capture_output=True, text=True).stdout
      assert status.strip() == "", f"working tree not clean after ship: {status}"

  def test_safety_commit_noop_on_clean_tree():
      """SC4: clean working tree → no safety commit (idempotent)."""
      d = _repo()
      _spec(d, "cleantree"); _unit(d, "cleantree"); _readme(d, "cleantree")
      p = _run(d, "cleantree")
      assert p.returncode == 0, p.stderr
      log = subprocess.run(["git", "log", "--oneline"], cwd=d, capture_output=True, text=True).stdout
      assert "save outstanding work" not in log, f"safety commit should NOT appear on clean tree: {log}"

  def test_safety_commit_includes_untracked_files():
      """SC5: untracked files are included in the safety commit via git add -A."""
      d = _repo()
      _spec(d, "untracked"); _unit(d, "untracked"); _readme(d, "untracked")
      _write(d, "brand_new_file.txt", "new content\n")
      p = _run(d, "untracked")
      assert p.returncode == 0, p.stderr
      # Verify the safety commit contains the untracked file
      safety_hash = subprocess.run(["git", "log", "--all", "--grep=save outstanding work",
                                     "--format=%H"], cwd=d, capture_output=True, text=True).stdout.strip()
      assert safety_hash, "safety commit not found"
      diff = subprocess.run(["git", "diff-tree", "--no-commit-id", "-r", "--name-only", safety_hash],
                            cwd=d, capture_output=True, text=True).stdout
      assert "brand_new_file.txt" in diff, f"untracked file missing from safety commit: {diff}"

  def test_already_shipped_no_safety_commit():
      """Already-shipped re-run → no safety commit (even with dirty tree)."""
      d = _repo()
      _readme(d, "alreadyship", status="shipped"); _spec(d, "alreadyship", status="shipped")
      _write(d, "dirty_file.txt", "should not be committed\n")
      p = _run(d, "alreadyship")
      assert p.returncode == 0, p.stderr
      assert "SHIP SKIPPED" in p.stderr, f"expected SKIPPED: {p.stderr}"
      log = subprocess.run(["git", "log", "--oneline"], cwd=d, capture_output=True, text=True).stdout
      assert "save outstanding work" not in log, f"safety commit must not run on already-shipped: {log}"

  if __name__ == "__main__":
  ```

## Allowed moves

Only the insertion of 4 test functions before the `if __name__` block. Do not modify existing
test functions. Do not change the `_repo()`, `_sh()`, `_write()`, `_spec()`, `_unit()`,
`_readme()`, or `_run()` helpers.

## STOP triggers

- Existing test functions modified or removed.
- `_repo()`, `_sh()`, `_write()`, `_spec()`, `_unit()`, `_readme()`, `_run()` helpers changed.
- `if __name__ == "__main__"` block removed or modified.
- Tests use assertions that pass without the implementation (hollow tests).

## Manual verification (record in decisions-ledger)

1. Run `python3 ~/.claude/wai/scripts/test_wai_ship.py` and verify all tests pass.
2. Verify the 4 new tests appear in the output: `test_safety_commit_on_dirty_tree`,
   `test_safety_commit_noop_on_clean_tree`, `test_safety_commit_includes_untracked_files`,
   `test_already_shipped_no_safety_commit`.
3. Verify no existing tests regressed.

## Done when

`python3 ~/.claude/wai/scripts/test_wai_ship.py` exits 0 with all tests passing (existing +
new).
