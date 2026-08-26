# Code Review — Round 3

**Target:** gentle-mongoose   **Range:** `1f2caaea..3bed17c1`   **Effort:** high

Round-2 remediations in `3bed17c1` verified: prettier@3.6.1 clean on AuthBoundary tests and UI files; blocked `window.open` returns without clobbering `popupRef` (`AuthBoundary.tsx:76-80`) and the re-click test kills the null-ref mutant; `protected_api_is_denied_by_default` pins non-JSON 401 (`browser_auth_routes.rs:65-73`); `startLogin` rejection stays on the login shell. Streams/proxy slice unchanged since round 2 (kimi seat unavailable this round; gpt + round-2 kimi evidence).

## Findings

| # | File:line | Severity | Category | Finding | Confidence | Actionable? |
|---|-----------|----------|----------|---------|-----------|-------------|
| — | — | — | — | none | — | — |

## Non-actionable

| # | File:line | Severity | Category | Finding | Confidence | Why non-actionable |
|---|-----------|----------|----------|---------|-----------|---------------------|
| N1–N15 | ledger `## Post-review known issues` | — | — | Prior rounds. | high | already adjudicated |
| N16 | `AuthBoundary.tsx:65-81` + `vks_browser_binding` | medium | correctness | A second `startLogin` from the same cookie jar overwrites the binding cookie, so an earlier popup's claim fails. Two node tabs, or a re-click that still awaits `startLogin` before `window.open`, hit this. | high | SC3/SC4 single-claim + one binding cookie is the designed contract; the first popup's HTML error is "start again". Not introduced by round-2 keep-ref. Bounded by `LOGIN_DEADLINE_MS`. |
| N17 | `AuthBoundary.tsx:71-80` | medium | correctness | `window.open` runs after `await startLogin`, so a slow Hive round-trip can lose the user-gesture and block the popup, then return with no extra UI. | medium | Login-shell UX is locked (N14). Opening after init is the 016 flow (need `authorize_url` first). Silent shell is the locked one-button page. |
| N18 | `AuthBoundary.test.tsx` blocked-re-click `callsAfterClose` | low | quality | Dynamic call count does not kill a `stopPolling()`-on-blocked mutant. The `popupRef = null` mutant is still red. | high | Adjacent-mutant test-gap; same class as N8. |

## Verdict: Approve

Actionable: []
