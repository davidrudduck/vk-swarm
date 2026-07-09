# Tournament Round 8 — Scoring Report

**Date:** 2026-07-08
**Challengers:** GPT-5.5, DeepSeek V4 Pro, MiMo v2.5 Pro

## Issues Found

| # | Issue | Finder | Severity | Valid? |
|---|-------|--------|----------|--------|
| 1 | `pendingKeyIds` leak on confirm-cancel | MiMo | Medium | **INVALID** (already fixed) |
| 2 | Stale `organizationId` in mutation closure | GPT-5.5 + DeepSeek | Medium | **PARTIAL** (createMutation only) |
| 3 | `parseErrorMessage` discards non-Error info | DeepSeek | Low | **VALID** |
| 4 | Long key names can break row layout | GPT-5.5 | Low | **VALID** |
| 5 | Doc inaccuracies (structure, line counts) | All 3 | Low | **VALID** |

## Peer Reviews

| Issue | Finder | Peer Reviewer | Finding Valid? | Remediation Valid? |
|-------|--------|---------------|----------------|-------------------|
| pendingKeyIds leak | MiMo | GPT-5.5 | **NO** (stale — code already fixed) | N/A |
| Stale orgId in closure | GPT-5.5 | DeepSeek | **PARTIAL** (createMutation only) | **NO** (mutationKey wrong, ref correct) |
| parseErrorMessage | DeepSeek | MiMo | **YES** | **YES** (peer approved) |

## Scoring

| Challenger | Findings | Remediation | Peer Review | Total |
|------------|----------|-------------|-------------|-------|
| GPT-5.5 | 1 (stale orgId) | 0 (mutationKey wrong) | N/A | **1** |
| DeepSeek | 1 (parseErrorMessage) | 1 (widened fallback) | 1 (peer approved) | **3** |
| MiMo | 0 (stale finding) | N/A | N/A | **0** |

**Winner: DeepSeek** (3 points)

## Remediations Applied

1. **parseErrorMessage** — widened to handle non-Error values (string, null, object)
2. **createMutation orgId ref** — `orgIdRef.current` used instead of captured `organizationId`
3. **Long key names** — added `truncate` class to key name span
4. **Doc structure** — corrected component tree (Alert is outside CardContent)

## Verification

- `tsc --noEmit`: clean
- `vitest run`: 14/14 pass
