# Tournament Round 22 — Scoring Report (FINAL)

**Date:** 2026-07-08
**Challengers:** GPT-5.5, DeepSeek, Claude Opus
**Commit:** `840339f4` (no new changes)

## Findings

No new defects found by any challenger.

## Scores

| Challenger | Findings | Points |
|---|---|---|
| **GPT-5.5** | 0 findings | **0** |
| **DeepSeek** | 0 findings | **0** |
| **Claude Opus** | 0 findings | **0** |

**Result: CLEAN ROUND — TOURNAMENT COMPLETE**

## Tournament Summary

| Round | Issues | Winner |
|-------|--------|--------|
| R1 | 10 | Kimi |
| R2 | 11 | GPT-5.5 |
| R3 | 7 | DeepSeek |
| R4 | 2 | MiniMax |
| R5 | 3 | DeepSeek |
| R6 | 0 | — |
| R7 | 0 | — |
| R8 | 0 | — |
| R9 | 10 | Kimi |
| R10 | 11 | DeepSeek & MiMo |
| R11 | 12 | MiniMax |
| R12 | 4 | DeepSeek & Claude Opus |
| R13 | 2 | Kimi |
| R14 | 1 | GPT-5.5 & Claude Opus |
| R15 | 1 | MiniMax |
| R16 | 1 | Claude Opus |
| R17 | 3 | Kimi |
| R18 | 1 | Claude Opus |
| R19 | 2 | Kimi |
| R20 | 1 | GPT-5.5 & Claude Opus |
| R21 | 0 | — |
| R22 | 0 | — |

**Total issues found and remediated:** 85
**Total rounds:** 22
**Clean rounds:** R6, R7, R8, R21, R22
**Consecutive clean rounds:** R21 + R22

## Component Final State

- 27 test cases in `NodeApiKeySection.test.tsx`
- 6 integration tests in `Nodes.test.tsx`
- All guards: `createAttemptRef` (epoch), `orgIdRef` (stale org), `isMountedRef` (unmount), `pendingKeyIds` (ref-counted per-key)
- `parseErrorMessage` handles: Error, string, null, symbol, plain object, JSON body, `{error}` format, circular refs
- Org-change resets all state (error, pending, dialog, attempt)
- Two-tier clipboard with fallback
- `uncloseable` dialog during secret reveal
