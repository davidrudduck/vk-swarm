# Code Review — Round 2

**Target:** feat/hive-oauth-sw-bypass   **Range:** `4beb483b12..c9711dd6`   **Effort:** high (focused delta re-review)

Scope: verify round-1 actionables [1,2,3] are remediated by task 401 (commit c9711dd6) and that
the remediation introduced nothing new. The 401 delta was adversarially mutation-tested by the
Stage-2 panel (4/4 mutations caught: denylist deletion, regex flip, config-only clause drop,
mirror-only clause drop; full suite 426/426, lint/tsc clean, built sw.js denylist grep=1).

## Findings

| # | File:line | Severity | Category | Finding | Confidence | Actionable? |
|---|-----------|----------|----------|---------|-----------|-------------|

Round-1 actionables closed:
- **#1 (denylist unpinned)** → `swConfigDriftGuard.test.ts` reconstructs the denylist regex from
  config source and asserts its match set (4 TRUE / 3 FALSE paths); mutation-verified.
- **#2 (mirror-only pin, manual grep)** → same suite enforces literal containment on BOTH files
  plus clause-count sync; one-sided edits fail (mutation-verified both directions).
- **#3 (stale comments)** → prescribed sentences applied verbatim in vite.config.ts;
  swCachePredicate.ts doc now names navigateFallbackDenylist as the primary fix and points to
  the automated guard; both forbid_after strings absent from the tree.

## Non-actionable

| # | File:line | Severity | Category | Finding | Confidence | Why non-actionable |
|---|-----------|----------|----------|---------|-----------|---------------------|
| 1 | `swConfigDriftGuard.test.ts` | low | test-integrity | Bare `toContain('navigateFallbackDenylist')` assertion is non-discriminating alone (a comment also carries the literal); regex-extraction tests are the real guard | high | Panel-noted, redundant-but-harmless assertion; guard strength proven by mutation testing |

## Verdict: Approve

Actionable: []
