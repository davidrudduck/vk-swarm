# Tournament Round 11 — Scoring Report

**Date:** 2026-07-08
**Challengers:** Kimi, MiniMax, GLM
**Commit:** `0345f972` (tournament R11: org guard, unmount guards, test gaps, doc accuracy)

## Findings

| # | Issue | Found By | Severity | Valid? | Points |
|---|-------|----------|----------|--------|--------|
| 1 | `execCommand` return value not checked | Kimi | MEDIUM | Valid | 1 |
| 2 | `listApiKeys` fires with empty orgId (R9 regression) | Kimi | MEDIUM | Valid | 1 |
| 3 | Stale create response leaks secret after org change | Kimi | MEDIUM | Valid | 1 |
| 4 | Doc mis-states test-case ownership | Kimi + MiniMax + GLM | LOW | Valid | 1 |
| 5 | `isListError` branch untested | GLM | MEDIUM | Valid | 1 |
| 6 | `uncloseable` behavior untested | GLM | MEDIUM | Valid | 1 |
| 7 | mutation-level `onError` doesn't guard `isMountedRef` | MiniMax + GLM | LOW | Valid | 1 |
| 8 | Symbol branch untested | MiniMax | LOW | Valid | 1 |
| 9 | org-change invalidation only tested for revoke | MiniMax | LOW | Valid | 1 |
| 10 | clipboard restore brittleness | MiniMax | LOW | Valid | 1 |
| 11 | `orgIdRef` mutated during render | MiniMax | LOW | Valid | 1 |
| 12 | 5 i18n fallback strings don't match locale | GLM | LOW | Valid | 1 |

## Scores

| Challenger | Findings | Points |
|---|---|---|
| **MiniMax** | 7 findings (1+1+1+1+1+1+1) | **7** |
| **GLM** | 5 findings (1+1+1+1+1) | **5** |
| **Kimi** | 4 findings (1+1+1+1) | **4** |

**Winner: MiniMax (7 points)**

## Remediations Applied

1. **enabled guard restored:** Added `enabled: !!organizationId` back to the list query. This was incorrectly removed in R9 — the component returns `null` for empty orgId but hooks still fire.

2. **orgIdRef moved to useEffect:** Changed from render-phase mutation (`orgIdRef.current = organizationId`) to `useEffect(() => { orgIdRef.current = organizationId; }, [organizationId])`. This follows React best practices for ref updates.

3. **createMutation orgId guard:** Added `if (orgId !== orgIdRef.current) return;` in `onSuccess` after the epoch guard. Prevents stale secrets from leaking when the user switches orgs while a create is in flight.

4. **mutation-level onError guards:** Added `if (!isMountedRef.current) return;` to `revokeMutation.onError` and `unblockMutation.onError`. Prevents `setError` on unmounted component.

5. **execCommand return value:** Added `if (!document.execCommand('copy'))` check that throws on failure, preventing false "Copied!" feedback.

6. **New tests:** TS22 (isListError), TS23 (uncloseable), TS24 (empty orgId query). 27 total tests.

7. **Doc fixes:** Corrected test-case ownership description, updated Nodes.test.tsx description.

## Tournament Status

- **Round 8:** 0 → clean
- **Round 9:** 10 → NOT clean
- **Round 10:** 11 → NOT clean
- **Round 11:** 12 → NOT clean
- **Next:** Round 12 required. Pick 3 from remaining challengers.
