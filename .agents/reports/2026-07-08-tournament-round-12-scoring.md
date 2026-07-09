# Tournament Round 12 — Scoring Report

**Date:** 2026-07-08
**Challengers:** GPT-5.5, DeepSeek, Claude Opus
**Commit:** `096deab6` (tournament R12: org-change dialog reset, double-submit guard, doc accuracy)

## Findings

| # | Issue | Found By | Severity | Valid? | Points |
|---|-------|----------|----------|--------|--------|
| 1 | Org change doesn't reset already-displayed secret | GPT-5.5 | MEDIUM | Valid | 1 |
| 2 | Doc test count stale (24 vs 27) | DeepSeek | MEDIUM | Valid | 1 |
| 3 | Clipboard mock teardown fragile | DeepSeek | MEDIUM | Valid | 1 |
| 4 | Double-submit via Enter key | Claude Opus | MEDIUM | Valid | 1 |
| 5 | Slow clipboard write leaks copied=true | Claude Opus | LOW | Valid (acknowledged) | 1 |

## Scores

| Challenger | Findings | Points |
|---|---|---|
| **DeepSeek** | 2 findings (1+1) | **2** |
| **Claude Opus** | 2 findings (1+1) | **2** |
| **GPT-5.5** | 1 finding (1) | **1** |

**Winners: DeepSeek & Claude Opus (tied at 2 points)**

## Remediations Applied

1. **Org-change dialog reset:** Added useEffect that closes the create dialog and resets all create state when `organizationId` changes. This prevents stale secrets from org A being visible in org B context.

2. **Double-submit guard:** Added `if (createMutation.isPending) return;` at the top of `handleCreateSubmit`. The button was already disabled, but Enter key in the input could still trigger form submit.

3. **Clipboard mock teardown:** Wrapped TS4 and TS14 test bodies in `try/finally` blocks to ensure `navigator.clipboard` and `document.execCommand` are always restored, even if assertions fail mid-test.

4. **Doc updates:** Updated test count from 24 to 27, added TS22-TS24 entries to the test table.

## Tournament Status

- **Round 8:** 0 → clean
- **Round 9-12:** NOT clean
- **Next:** Round 13 required. Pick 3 from remaining challengers.
