# Code Review — Round 1

**Target:** feat/hive-oauth-sw-bypass   **Range:** `4beb483b12..974a36d0+` (origin/main...HEAD)   **Effort:** high

Method: 3 parallel finder subagents (correctness / quality / test-integrity); all candidate
findings verified against the worktree before recording. Both test suites re-run green
(frontend 16/16 OAuthDialog, remote-frontend 15/15 swCachePredicate).

## Findings

| # | File:line | Severity | Category | Finding | Confidence | Actionable? |
|---|-----------|----------|----------|---------|-----------|-------------|
| 1 | `remote-frontend/vite.config.ts:24` | medium | correctness | `navigateFallbackDenylist: [/^\/v1\//]` — the load-bearing OAuth fix per deploy-verification round 1 — has ZERO automated pin; deleting it passes every gate/test in the repo | high | yes |
| 2 | `remote-frontend/src/lib/swCachePredicate.test.ts` + `vite.config.ts:36-39` | medium | test-integrity | Tests pin a hand-copied mirror, not the shipped predicate; the "drift-guard grep" both comments cite was a one-time manual command (task 102), not CI/test-enforced — config-only edits drift silently | high | yes |
| 3 | `remote-frontend/src/lib/swCachePredicate.ts:10-13`, `vite.config.ts:30,34` | low | quality | Comments stale/overstated post-amendment-3: mirror doc implies the cache exclusion is the sign-in fix (deploy round 1 disproved); "Excluded requests bypass the SW entirely" is wrong for navigations; "drift-guard ties the copies together" overstates a manual grep | high | yes |

## Non-actionable

| # | File:line | Severity | Category | Finding | Confidence | Why non-actionable |
|---|-----------|----------|----------|---------|-----------|---------------------|
| 4 | `frontend/.../OAuthDialog.tsx:54,98-100` | low | correctness | Popup close-on-timeout (and pre-existing popup-closed detection) is dead in real browsers: `window.open` with `noopener=yes` returns null, so `popupRef.current` is always null; tests only exercise it via a mocked `window.open` | high | Root cause pre-existing (`noopener` predates diff); removing `noopener` is a deliberate security tradeoff — follow-up decision, not this diff |
| 5 | `frontend/.../OAuthDialog.tsx:96-102` | low | correctness | ~1s deadline/success boundary race: sign-in completing in the last poll gap before 120s shows the timeout error despite an established session | medium | Inherent to any bounded-deadline design; window ≤1s; retry path recovers |
| 6 | `remote-frontend/vite.config.ts:24` | low | correctness | Bare `/v1` (no trailing slash) navigation not denylisted | high | Cosmetic — no real endpoint lives at bare /v1 |
| 7 | `frontend/.../OAuthDialog.tsx:98-100` | low | quality | 5th copy of the popup-close guard block; a `closePopup()` helper warranted | high | Cosmetic; pattern pre-existing (4 copies on main); fold into the noopener follow-up |
| 8 | `frontend/package.json:106` | low | quality | `@vitest/coverage-v8` devDep added with no coverage script/config invoking it | high | Harmless; used ad hoc for recorded coverage evidence; wire a `test:coverage` script in a housekeeping pass |
| 9 | repo gates / `.github/workflows` | low | test-integrity | Frontend vitest is not part of any CI gate (`npm run check` = tsc only), so the new deadline pins can rot | high | Pre-existing repo-level gap; needs a repo-owner decision, out of this workstream's scope |
| 10 | `OAuthDialog.test.tsx:5-14` + locale JSONs | low | test-integrity | Nothing asserts `oauth.timeoutError` exists in the four locale files (mock returns raw keys; `lint:i18n` checks literals only) | high | Repo-standard i18n test pattern; key existence verified manually in all 4 locales this run |
| 11 | `frontend/src/i18n/locales/{en,es}/common.json:timeoutError` | low | quality | en phrasing predicates completion of a "window" not the flow; es "ha caducado" less idiomatic than "se agotó el tiempo" | medium | Cosmetic wording; all four locales consistent and intelligible |

## Verdict: With fixes

Actionable: [1,2,3]
