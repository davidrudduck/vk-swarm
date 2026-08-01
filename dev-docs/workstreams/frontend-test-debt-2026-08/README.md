---
workstream: frontend-test-debt-2026-08
status: active
created: 2026-08-01
parent_session: vk-swarm-node-ui-localize execute run
---

# frontend-test-debt-2026-08

Clears the 8 pre-existing failing frontend test files (15 tests) discovered at task 201's gate
during the `vk-swarm-node-ui-localize` run and filed as **F-2026-07-31-01, -02, -03**.

**Why this is a separate unit, not tasks inside `vk-swarm-node-ui-localize`.** That workstream's
spec is FROZEN (ADR-0001) and covers node hive-proxy routes, the API-key surface, `ProjectWithStats`
and the hive-absent state. None of these eight files relates to any of that. Adding them to that
task tree would make the frozen spec meaningless. The user directed they be fixed *before the run
closes*, which this satisfies without corrupting the spec.

## Triage verdict — ALL 8 ARE STALE TESTS, ZERO PRODUCT BUGS

Each traces to an intentional, commit-documented behaviour change whose test was never updated.

| # | File | Root cause | Fix |
|---|---|---|---|
| 1 | `pages/settings/__tests__/SystemSettings.test.tsx` | lucide-react mock Proxy answers EVERY key including `then`; vitest awaits the factory, calls `Icon(resolve,reject)` at import time | guard `then`/`__esModule`/symbols in the trap + `React.createElement` |
| 2 | `pages/settings/__tests__/SettingsMobile.test.tsx` | sections grew 6→8 (`webhooks` in 73ac658a; `organizations`/`swarm` added; `backups` folded into `system` in e7973175) | `toBe(8)` at :85,:217,:245; search query `'backup'`→`'system'` at :180 |
| 3 | `lib/taskSorting.test.ts` | 4215189a deliberately changed `inreview` from `latest_execution_completed_at` to `activity_at` | fix the FIXTURES (see trap below), not the expectations |
| 4 | `components/layout/__tests__/BottomNav.test.tsx` | 6ede4301 changed active style `text-primary` → `text-foreground font-medium` | assert `aria-current="page"`, NOT a class regex |
| 5 | `__tests__/MobileIntegration.test.tsx:226` | same as #4 | same as #4 |
| 6 | `__tests__/DesignSystem.test.tsx:53` | 91fc6b03 (Midnight Terminal) changed `VKSLogo` `font-code` → `font-wordmark` | `toHaveClass('font-wordmark')` |
| 7 | `components/tasks/__tests__/ConversationFocusMode.test.tsx` | 267f0803: `TodosBadge` now always renders, and shows `(2)` not `2` | assert `(2)`/`(0)`; rewrite the two "renders nothing" cases |
| 8 | `components/tasks/message-queue/MessageQueuePanel.test.tsx:105` | b43ddbae removed variant display entirely (always `variant: null`) | delete the test |

## THREE TRAPS — the obvious fix gives a FALSE GREEN

These are why triage ran before any fix was attempted:

1. **#1** — making the mock factory `async` and awaiting `react` makes it WORSE: the returned
   thenable-looking Proxy gets awaited, `then` is called, never resolves, and the suite HANGS
   instead of erroring. The `then` guard is load-bearing.
2. **#3** — editing only the expected timestamps at `:279`/`:282` would assert FALLBACK behaviour
   (both fixtures have `activity_at: null`, so both collapse to `created_at`) under a test named
   "inreview sorting". The fixtures must gain distinct `activity_at` values.
3. **#4/#5** — relaxing the regex to `/text-foreground/` ALSO matches the inactive button, which
   carries `hover:text-foreground`. The test would then pass in both states and stop catching the
   regression it exists for. Use the semantic `aria-current="page"` the component already emits.

## Out-of-scope observations (filed, not fixed here)

- `i18n/locales/en/settings.json:184` still carries `backups`/`backupsDesc` nav strings with no
  matching section — searching "backup" in mobile settings returns nothing. Minor real
  discoverability gap.
- `i18n/locales/en/tasks.json:184` `messageQueue.variant` is dead since b43ddbae.
