# Code review round 2 — vk-swarm-design-system (verification)

- **Method:** independent verification subagent (ses_fb993fbde) vs the uncommitted working tree containing A1–A7; static verification only (gates already green: lint 0, tsc 0, vitest 54 files / 413 tests).

## Findings

| ID | file:line | severity | category | finding | call |
|----|-----------|----------|----------|---------|------|
| R2-1 | `docs/plans/vk-swarm-design-system/decisions-ledger.md` (+ code comments citing it) | medium | process | Round-1 fixes cite a "2026-08-28 close review" ledger entry that did not exist yet in the ledger (A6/A7 code comments + round-1 record pointed at it). | **actionable — FIXED**: `## 2026-08-28 — pre-graduation code review (close gate)` section appended to the unit ledger in this session, recording A1–A7 dispositions + non-actionables. |
| R2-2 | `docs/development/remote-frontend.mdx:17,70,155,174` | low | docs drift | Doc still listed the deleted `e2e/fixtures/mock-electric.ts` (tree + fixture bullet) and described the six-collection contract streaming from the dead `/api/electric/v1/shape/*` path; line 174 additionally documented the deleted `mockElectricShape` helper. | **actionable — FIXED**: all four regions rewritten (single-table contract + `/v1/shape/shared_tasks`; fixture bullet replaced with a note that an Electric fixture must be captured from a real envelope stream); also corrected two adjacent stale paragraphs describing the deleted Tasks.tsx `useLiveQuery` page and sync-status behavior, same drift class. |

## Non-actionable

| ID | finding | disposition |
|----|---------|-------------|
| R2-N1 | `createShapeStreamOptions` is test-surface-only in the narrowed electric module | Pre-existing; unchanged by this diff. |
| R2-N2 | Disabled NavIcons lose keyboard focusability | Intended (honest-disabled pattern matches New Task/Settings siblings). |

## Per-fix verification summary (round 1)

A1 CONFIRMED (fixed overlay+aside, z 10/11 preserved, no containing-block hazard — only `translateY(-50%)` transform in the tree is on a non-ancestor; regression pin goes red on revert). A2 CONFIRMED (clamp pinned: `toBe(3)` + 'c' absent). A3 CONFIRMED (`role="img"`). A4 CONFIRMED (type pulled out of spread; `type="submit"` passthrough pinned by `button-badge-card.test.tsx:26-34`; all 7 `<form>` sites use the shadcn Button, unaffected). A5 CONFIRMED (matches established NOT_WIRED_TITLE pattern; no test asserted enabled). A6 CONFIRMED (zero importers of deleted symbols repo-wide incl. e2e; `createShapeUrl('nodes')` throws + asserted; single-table URL asserted in 3 files; `crates/remote/src/routes/electric_proxy.rs:28` re-verified — exactly one shape route). A7 CONFIRMED (deletion complete, zero references outside immutable dev-docs history).

## Verdict:

Approve-with-fixes — both round-2 findings fixed in-session (ledger section + mdx rewrite); round 3 to confirm convergence.

Actionable: [R2-1, R2-2]
