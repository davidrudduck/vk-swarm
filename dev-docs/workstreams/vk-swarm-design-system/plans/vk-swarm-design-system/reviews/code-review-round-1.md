# Code review round 1 — vk-swarm-design-system (pre-graduation close gate)

- **Target:** `3dcdc24c` (squash of PR #466, merged to main) + post-merge CodeRabbit remediation relationship check (`cb87543f` commit object absent from this clone, but its fixes verified present in tree: NodeCard pulse `role="status"` + online/offline labels, Tabs `aria-controls` + deterministic ids).
- **Verified against:** HEAD `108b3c41` (post Unit-A graduation; remote-frontend tree).
- **Method:** 2 parallel finder subagents (ses_fb9a8265d styles/components lens, ses_fb9a7ff09 app-shell integration lens), HIGH effort, every finding cited to real file:line and empirically verified (A1 via headless Chromium against the dev server with 40 mock tasks).

## Findings

| ID | file:line | severity | category | finding | call |
|----|-----------|----------|----------|---------|------|
| A1 | `remote-frontend/src/ui/panels/TaskDrawer.tsx:44-54` | high | correctness | Overlay + aside `position:absolute` with no positioned ancestor → containing block is the ICB; on a scrolled board the opened drawer lands at `top:-3665px` (empirically proven at scrollY 3665, `visible:false`). jsdom + 5-task e2e blind. | **actionable — FIXED**: both set `position:'fixed'` (viewport-anchored); regression pin added in `drawer.test.tsx` asserting `style.position === 'fixed'` + viewport edges. |
| A2 | `remote-frontend/src/components/board/statusbadge-taskcard.test.tsx:53` | medium | test-integrity | "up to 2 labels + days badge" test asserted `>=3` badges — vacuous if the `labels.slice(0,2)` clamp regressed; never asserted label 'c' absent. | **actionable — FIXED**: `toBe(3)` + `queryByText('c')` null + a/b visible. |
| A3 | `remote-frontend/src/components/board/TaskCard.tsx:46` | medium | a11y | `AttemptIndicator` merged/failed `<svg aria-label>` had no `role` → not exposed to a11y tree, inconsistent with running-state `Loader role="status"`. | **actionable — FIXED**: `role="img"` added. |
| A4 | `remote-frontend/src/components/core/Button.tsx:38` | low | correctness | `Button` spread props without defaulting `type` → implicit `type="submit"` inside forms; sibling primitives (Switch/Checkbox) hard-code `type="button"`. | **actionable — FIXED**: destructured `type = 'button'`. |
| A5 | `remote-frontend/src/ui/chrome/Chrome.tsx:175-220` | low | quality/a11y | Desktop search input `placeholder`-only label, no `type="search"`; mobile Search / Activity / Menu NavIcons enabled with no `onClick` (dead clicks) — violating the honest-disabled policy applied to siblings. | **actionable — FIXED**: input `type="search"` + `aria-label="Search tasks"`; three NavIcons `disabled` + `title` per `NOT_WIRED_TITLE` pattern. |
| A6 | `remote-frontend/src/lib/electric/config.ts:41-92` (+ collections.ts, tests) | medium | quality | Contract drift: module advertised 6 shape tables + 6 collection factories but the hive proxy serves exactly one route (`/v1/shape/shared_tasks`, `crates/remote/src/routes/electric_proxy.rs:28`); `projects`/`node_projects` dropped by migrations; `ElectricNode.hostname/os_info` don't exist (capabilities JSONB). Sole consumer (Tasks.tsx) was deleted by this commit's own F8 sweep — latent 404 trap for the "wire later" plan. | **actionable — FIXED**: single-table contract (`shared_tasks` only; `createShapeUrl('nodes')` now throws — asserted in tests); `ElectricSharedTask = ElectricRow & {id, organization_id}` + `createSharedTasksCollection()`; barrel + 4 test files rewritten; stale `bridge.test.ts` rewritten to pin the barrel surface. |
| A7 | `remote-frontend/e2e/fixtures/mock-electric.ts:13-24` | low | quality | Zero importers; protocol-wrong (raw ndjson rows vs Electric shape envelopes) — a future e2e built on it would pass against a mock the real parser rejects. | **actionable — FIXED (deleted)**: recreate from a captured real stream when e2e needs it (ledgered). |

## Non-actionable

| ID | file:line | severity | category | finding | disposition |
|----|-----------|----------|----------|---------|-------------|
| N1 | `remote-frontend/src/styles/components.css:26` (+ colors.css:64,134) | low | correctness | `--ring-hsl` fallback dead token; `--ring` defined in both themes but consumed by nothing → button focus rings ignore the theme-aware ring token in light mode. | Byte-identity design-source lock — upstream design-source item; ledgered. |
| N2 | `remote-frontend/tailwind.config.js:8` | medium | correctness (pre-existing) | `theme.extend` empty → shadcn named-color utilities (`text-muted-foreground` etc.) generate no CSS. Byte-identical at `3dcdc24c^` — NOT a regression of this commit. | Promoted to `dev-docs/BACKLOG.md` as its own remediation unit. |
| N3 | `remote-frontend/src/styles/tokens/textures.test.tsx:26-54` | low | test-integrity | Render-based texture tests tautological (className echo); real guard is the CSS-string asserts in the same file. | Cosmetic; real guard exists. |
| N4 | `remote-frontend/src/components/render-parity.test.tsx:21-44` | low | test-integrity | Mount-smoke only (`not.toThrow`); per-component suites carry the actual parity contract. | Harmless additive smoke. |
| N5 | `remote-frontend/src/AppRouter.tsx:106` | low | correctness | Unknown-path fallback highlights Processes NavTab while rendering NotFoundPage inside ChromeLayout. | Cosmetic; all tabs navigate correctly. |
| N6 | round-2 carryover (theoretical) | low | quality | With A6's narrowed module, any future second table needs deliberate re-introduction — that friction is intentional. | By design. |

## Verdict:

Approve-with-fixes — 7 actionable findings, all fixed in-session (A1–A7); gates re-run green (lint 0, tsc 0, vitest 54 files / 413 tests). Round 2 verification pending.

Actionable: [A1, A2, A3, A4, A5, A6, A7]
