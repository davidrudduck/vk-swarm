# Tournament Round 16 — Scoring Report

**Date:** 2026-07-08
**Challengers:** GPT-5.5, DeepSeek, Claude Opus
**Commit:** `661c9d89` (tournament R16: parseErrorMessage extracts {error} format)

## Findings

| # | Issue | Found By | Severity | Valid? | Points |
|---|-------|----------|----------|--------|--------|
| 1 | `parseErrorMessage` ignores `{error}` format | Claude Opus | LOW | Valid | 1 |
| 2 | Non-admin users shown Unblock control | Claude Opus | MEDIUM | Invalid (design decision) | 0 |

**GPT-5.5:** No findings
**DeepSeek:** No findings

## Scores

| Challenger | Findings | Points |
|---|---|---|
| **Claude Opus** | 2 findings (1+0) | **1** |
| **GPT-5.5** | 0 findings | **0** |
| **DeepSeek** | 0 findings | **0** |

**Winner: Claude Opus (1 point)**

## Remediations Applied

1. **parseErrorMessage {error} fallback:** Added `if (typeof parsed.error === 'string') return parsed.error;` after the `parsed.message` check. Now handles backend errors shaped as `{error: 'message'}` in addition to `{message: '...'}`.

## Tournament Status

- **Round 8:** 0 → clean
- **Round 9-16:** NOT clean
- **Next:** Round 17 required. Pick 3 from remaining challengers.
