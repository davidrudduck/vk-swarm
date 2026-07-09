# Tournament Round 9 — Scoring Report

**Date:** 2026-07-08
**Challengers:** MiniMax M3, Kimi K2.7, Claude Opus 4.8
**Commit:** `3ce94cb7` (tournament R9: form submit, parseErrorMessage tests, doc accuracy)

## Findings

| # | Issue | Found By | Severity | Peer Review | Valid? | Points |
|---|-------|----------|----------|-------------|--------|--------|
| 1 | `revokeMutation`/`unblockMutation` uses `organizationId` not `orgIdRef.current` | MiniMax | Medium | Kimi: Partial valid — code already passes orgId as mutation variables | **Already fixed** | 0 |
| 2 | No `<form>` wrapper — Enter key doesn't work in create dialog | Kimi | Medium | MiniMax: Valid — confirmed with 6 sibling dialogs as precedent | **Valid** | 3 |
| 3 | Architecture doc stale line numbers/counts | MiniMax + Kimi + Claude | Low | — | **Valid** | 1 (shared) |
| 4 | Doc says "dialog" for `confirm()` | MiniMax | Low | — | **Valid** | 1 |
| 5 | `parseErrorMessage` branches not tested | Kimi | Medium | Claude: Valid — confirmed gap, provided detailed remediation | **Valid** | 3 |
| 6 | `revokeMutation` error path not tested | MiniMax | Low | — | **Valid** | 1 |
| 7 | `fireEvent` vs `user-event` inconsistency | Kimi | Low | — | **Valid** | 1 |
| 8 | Redundant `enabled: !!organizationId` | MiniMax | Low | — | **Valid** | 1 |
| 9 | `setPendingKeyIds` after unmount | Kimi | Low | — | **Valid** | 1 |
| 10 | `invalidateQueries` before epoch guard | Kimi | Low | — | **Valid** | 1 |

## Scores

| Challenger | Findings | Points |
|---|---|---|
| **Kimi K2.7** | Enter key (3) + parseErrorMessage (3) + doc (1) + fireEvent (1) + stale guard (1) + unmount (1) | **10** |
| **MiniMax M3** | orgIdRef (0, already fixed) + doc findings (1) + confirm wording (1) + revoke test (1) + enabled guard (1) | **4** |
| **Claude Opus** | doc (1) | **1** |

**Winner: Kimi K2.7 (10 points)**

## Remediations Applied

1. **Enter key / form wrapper:** Added `<form onSubmit={handleCreateSubmit}>` wrapping the create dialog body. Create button changed to `type="submit"`, Cancel to `type="button"`. Added `autoFocus` on the name input. Follows the pattern established by 6 sibling dialogs in `remote-frontend/src/components/swarm/`. Test TS18 added.

2. **isMounted guard:** Added `isMountedRef` that sets `false` on unmount. `setPendingKeyIds` calls in `handleRevoke` and `handleUnblock` are now guarded by `if (isMountedRef.current)`.

3. **invalidateQueries reorder:** In `createMutation.onSuccess`, moved the epoch guard check (`if (attemptId !== createAttemptRef.current) return;`) before `queryClient.invalidateQueries(...)`. This prevents stale attempts from triggering unnecessary refetches.

4. **Redundant enabled guard:** Removed `enabled: !!organizationId` from the list query. The component already returns `null` at line 269 when `organizationId` is falsy, making the `enabled` guard redundant.

5. **parseErrorMessage tests:** Added 4 parameterized tests (TS16a-d) covering string, null, plain object, and JSON-message rejection payloads. These exercise the non-`Error` branches of `parseErrorMessage`.

6. **Revoke error test:** Added TS17 that mocks `revokeApiKey` to reject with an Error and verifies the Alert appears.

7. **fireEvent migration:** Migrated TS5, TS6, TS15 from `fireEvent` to `user-event`. TS4 and TS14 retain `fireEvent` for clipboard-specific interactions (browser API mocking requires lower-level event dispatch).

8. **Doc accuracy:** Removed hardcoded line counts and assertion counts from architecture doc. Softened coverage percentages to approximate bands. Fixed "dialog" → "confirmation prompt" in user-facing feature doc. Updated test case table to include all 21 tests.

## Tournament Status

- **Round 8:** 0 valid issues → clean
- **Round 9:** 10 valid issues → NOT clean
- **Next:** Round 10 required. Pick 3 from remaining challengers (GPT-5.5, DeepSeek, GLM, MiMo).
