# Tournament Round 20 — Scoring Report

**Date:** 2026-07-08
**Challengers:** GPT-5.5, DeepSeek, Claude Opus
**Commit:** `840339f4` (tournament R20: fix architecture doc for pendingKeyIds Map)

## Findings

| # | Issue | Found By | Severity | Valid? | Points |
|---|-------|----------|----------|--------|--------|
| 1 | Stale architecture doc for pending state | GPT-5.5 + Claude Opus | LOW | Valid | 1 each |

**DeepSeek:** No findings

## Scores

| Challenger | Findings | Points |
|---|---|---|
| **GPT-5.5** | 1 finding (1) | **1** |
| **Claude Opus** | 1 finding (1) | **1** |
| **DeepSeek** | 0 findings | **0** |

**Winners: GPT-5.5 & Claude Opus (tied at 1 point)**

## Remediations Applied

1. **Doc fix:** Updated architecture doc to describe `pendingKeyIds` as `Map<string, number>` with ref-count semantics, supporting concurrent mutations on the same key.

## Tournament Status

- **Round 8:** 0 → clean
- **Round 9-20:** NOT clean
- **Next:** Round 21 required. Need two consecutive clean rounds.
