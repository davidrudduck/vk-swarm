# Tournament round 1 — hive-node-api-key-ui (2026-07-07)

## Method
Dispatched 2 external CLI competitors in parallel for the find round:
- **Codex** (gpt-5.5) — `run_codex_panel.py --task plan-review --safety read-only`
- **Gemini** (0.49.0) — `run_gemini_panel.py --task plan-review --safety read-only`

Both panels read the frozen spec, the plan, the 8 task files, and the real repo sources
(Nodes.tsx, the swarm section siblings, the lib/api/nodes.ts API client, the types, the locale
JSON, the frontend reference impl). Both produced a Markdown findings table.

## Raw submissions
- Codex: `reviews/round-1-codex.md` (8 findings)
- Gemini: `reviews/round-1-gemini.md` (4 findings)

## Scoreboard (peer-validated findings)

| # | competitor | task | severity | issue | valid? | fix verified? |
|---|------------|------|----------|-------|--------|----------------|
| 1 | codex | 001 | BLOCKING | Button `onClick` refs `setShowCreateDialog` before the state hook is added in task 002 — typecheck fails | YES (file:line 150 of task 001 confirmed) | YES — added `useState` hook to task 001 body |
| 2 | codex | 001 | BLOCKING | `key_id` is not a real `NodeApiKey` field; should be `node_id` (per `remote-frontend/src/types/nodes.ts:56`) | YES | YES — replaced `key_id` with `node_id` in task 001 |
| 3 | codex | 002 | MAJOR | `@testing-library/user-event` not in `remote-frontend/package.json`; would fail to resolve | YES (line 41-46 of package.json confirmed) | YES — switched test to use `fireEvent` from `@testing-library/react` |
| 4 | codex | 002 | MAJOR | TS4 test doesn't exercise show/hide toggle or close-clears-secret (per spec SC2) | YES | YES — extended test to cover both; component now exposes `data-secret-wrapper` and `data-hidden` for assertions |
| 5 | codex | 003/006 | MAJOR | `TooltipProvider` missing in production mount (`App.tsx` line 1-22 has no provider) | YES | YES — task 001 now wraps the section in `<TooltipProvider>` (self-contained) |
| 6 | codex | 006 | MINOR | Task 006 imports from `@/components/swarm/NodeApiKeySection` directly, but plan says "single import from `@/components/swarm`" | YES | YES — changed to barrel import; test mock targets the barrel via `importOriginal` |
| 7 | codex | 007 | MAJOR | TS9 test lists 18 keys; spec defines 30 | YES | YES — expanded `requiredKeys` to all 30 |
| 8 | codex | 008 | MINOR | Task 008 says 6 tests in `NodeApiKeySection.test.tsx`; actually 9 | YES | YES — corrected to 9 tests |
| 9 | gemini | 001 | BLOCKING | Same as codex #2 (`key_id` → `node_id`) | YES (duplicate) | (covered by fix #2) |
| 10 | gemini | 004 | BLOCKING | TS7 expects `screen.getByText(/boom/)` but the `useTranslation` mock from task 001 ignores options — interpolation never reaches the DOM | YES (mock signature `(key) => key` confirmed in task 001 line 59) | YES — updated mock to `(key, fallback, options) => fallback.replace(/\{\{(\w+)\}\}/g, ...)` |
| 11 | gemini | 001 | BLOCKING | Task 001's ApiKeyItem omits `created_at` / `last_used_at` timestamps required by spec SC3 | YES | YES — added `t('settings.swarm.apiKeys.created', ...)` and `t('settings.swarm.apiKeys.lastUsed', ...)` to ApiKeyItem; TS3 test now asserts both |
| 12 | gemini | 007 | BLOCKING | Before-text for `frontend/src/i18n/locales/en/settings.json` insertion references non-existent key `"promoteConfirm": "Promote"`; the actual key is `"promoteDialog": { ..., "confirm": "Promote" }` | YES (verified by reading lines 738-743 of en/settings.json) | YES — corrected Before-text to match the real structure |

**Total peer-validated findings: 12** (Codex 8, Gemini 4, with 1 duplicate between panels).
All 12 are remediated. None collides with the frozen spec (no spec amendments required).

## Termination

This round closes per the tournament termination rule: every peer-validated finding is
remediated AND a focused re-check (`wai-plan-lint.sh hive-node-api-key-ui`) passes clean
(no W: warnings, no X: failures, SC/TS coverage enforced).

No second round is dispatched; the termination rule explicitly says NOT to launch another
full round just to confirm silence.
