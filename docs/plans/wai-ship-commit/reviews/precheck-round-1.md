## Adversarial review: wai-ship-commit precheck round 1

**Date:** 2026-07-08
**Panelists:** gpt-5.5, kimi-k2.7-code, deepseek-v4-pro (randomly selected)
**Spec:** `docs/superpowers/specs/2026-07-08-wai-ship-commit.md`

### Findings

| # | Panelist | Finding | Classification |
|---|----------|---------|---------------|
| 1 | ALL | `fail()` called before defined at insertion point | ACTIONABLE |
| 2 | gpt-5.5 | Already-shipped rerun creates stranded safety commit | ACTIONABLE |
| 3 | kimi | Error message misleading about index state post-add-A | NON-ACTIONABLE |
| 4 | deepseek | `git status --porcelain 2>/dev/null` masks git failures | NON-ACTIONABLE |
| 5 | deepseek | `git add -A` excludes `.gitignore`d files | NON-ACTIONABLE |
| 6 | deepseek | `pipefail` irrelevant to command substitution | NON-ACTIONABLE |
| 7 | deepseek | Commit failure message misleading about index state | NON-ACTIONABLE |

### Remediation applied

**Finding 1:** Moved insertion point after `fail()` definition (line 41) instead of between
branch guard (line 39) and `fail()` (line 41). Updated spec's Approach and Design sections.

**Finding 2:** Moved already-shipped skip check (lines 114–119) before the safety commit.
If the workstream is already shipped, ship exits immediately without creating any commits.
Added Decision 6 and Test 6 to the spec.

### Non-actionable dismissals

- **Finding 3/7 (kimi/deepseek, error message):** The message "No ship frontmatter edits have
  been made" is factually correct. Added clarifying note about `git add -A` staging the index.
- **Finding 4 (deepseek, git-failure masking):** `repo_root` at line 27 would fail first if
  git is broken. Same `2>/dev/null` pattern as `wai-close.sh:185`.
- **Finding 5 (deepseek, gitignore):** By design — committing gitignored files would be a bug,
  not a feature. Spec explicitly calls this out of scope.

### Verdict

`Actionable: [1, 2]` — both fixed in spec.
