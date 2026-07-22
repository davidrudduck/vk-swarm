# ADVERSARIAL TOURNAMENT — FIND + REMEDIATE round (hive-node-api-key-ui)

You are ONE competitor in a multi-model adversarial tournament. You review a
decomposition (a frozen spec + a plan of 8 surgical TDD task files) BEFORE any code is written.

**Scoring (you want the high score):**
- +1 for each REAL, cited problem in the breakdown.
- +1 more for a remediation that is concrete and correct.
- **BUT** every finding+remediation you submit will be adversarially judged by a *peer model*
  (not you). A finding the peer rules NOT-real scores 0. A remediation the peer rules
  insufficient/incorrect scores 0 for that half. So do NOT pad with pedantic/vacuous nits — a weak
  finding or a hand-wavy fix loses you points. Quality and correctness beat quantity.

**Ground every claim in the real repo** (read the files, run grep) — an uncited or unverifiable
finding will be rejected by the peer judge. Trace control-flow claims to actual `file:line`.

## What to read (paths relative to your --cwd = repo root)
- SPEC (frozen): `docs/superpowers/specs/2026-07-07-hive-node-api-key-ui.md`
- PLAN: `docs/plans/hive-node-api-key-ui/plan.md`
- TASK FILES: every `docs/plans/hive-node-api-key-ui/phase-*/*.md` (8 tasks)
- The real sources each task edits:
  - `remote-frontend/src/pages/Nodes.tsx` (composed in by task 006)
  - `remote-frontend/src/pages/Nodes.test.tsx` (extended in task 006)
  - `remote-frontend/src/components/swarm/SwarmHealthSection.tsx` (sibling pattern)
  - `remote-frontend/src/components/swarm/NodeProjectsSection.tsx` (sibling pattern)
  - `remote-frontend/src/components/swarm/index.ts` (barrel, extended in task 005)
  - `remote-frontend/src/components/swarm/index.test.tsx` (extended in task 005)
  - `remote-frontend/src/lib/api/nodes.ts` (the API client; `listApiKeys`/`createApiKey`/`revokeApiKey`/`unblockApiKey`)
  - `remote-frontend/src/types/nodes.ts` (the `NodeApiKey`, `CreateNodeApiKeyRequest`, `CreateNodeApiKeyResponse` types)
  - `frontend/src/i18n/locales/{en,es,ja,ko}/settings.json` (extended in task 007)
  - `frontend/src/components/org/NodeApiKeySection.tsx` (the reference impl — 451 lines, the behavioral source)
  - The WAI plan-lint and the `docs/plans/vk-swarm-design-system/decisions-ledger.md` (prior decomposition patterns)

## Attack axes (cite task id + the contradicting repo file:line)

1. **Not bite-sized / two concerns in one task.** E.g. task 002 covers both the create Dialog AND the secret reveal — is that one concern or two? Task 007 edits 4 JSON files AND a TS file.
2. **Wrong or non-existent anchor/symbol/Before-text.** Every Change section gives exact file:line + Before text. Verify each one is real (the file exists, the line/anchor matches, the Before text is the actual current content).
3. **Ambiguous instruction.** A step that leaves the implementer to choose between two reasonable interpretations.
4. **allowed_change mismatch.** E.g. task 002 says `edit` but creates a brand-new `useMutation` block — is `edit` right when the file already exists and we're modifying? (yes, this is correct — `create` is for new files, `edit` is for changes to existing). But verify there are no `move`/`delete` tasks that the rubric says should be split.
5. **Dependency/conflict error or cycle.** `006` depends on `002, 005`; `007` depends on `004`; `008` depends on `006, 007`. Trace the graph. Also: tasks 002, 003, 004 all edit the same file (`NodeApiKeySection.tsx`) AND the same test file — does `conflicts_with` need to be set? (check: the lint accepts sequential edits to the same file because `parallel: false`.)
6. **Unmarked irreversible.** No task should delete code, remove a dep, or change a public contract. The spec says "no Rust changes" and "frontend/ unchanged" — verify no task violates that.
7. **Untestable or HOLLOW test (would pass without the implementation).** Each task's failing test must really fail without the implementation. For example, does task 002's test actually exercise the Dialog open/close + secret reveal + copy? Or does it just check that the button click does nothing?
8. **CONTROL-FLOW GROUNDING: open the real code.** `nodesApi.createApiKey` takes `data: CreateNodeApiKeyRequest` where `organization_id` is a string (see `remote-frontend/src/lib/api/nodes.ts:67-79` and `remote-frontend/src/types/nodes.ts:67-75`). Verify the task 002 mutation's argument shape matches. Same for revoke (line 81-89) and unblock (line 95-104). The test for TS4 expects `nodesApi.createApiKey` to be called with `{ organization_id: 'org-1', name: 'Test Key' }` — verify this matches the actual call in the change.
9. **Fidelity: an SC/TS clause no task truly delivers (covered-but-hollow).** Each spec SC and TS is claimed by at least one task's `covers_criteria`/`covers_tests` (the lint enforces this). But the lint doesn't check that the task ACTUALLY delivers — e.g. SC6 says "all user-facing strings go through `useTranslation(['settings', 'common'])`" — does task 001 actually wire this up in the component skeleton, or does it only wire the list/empty/loading states? Walk each clause sub-id.

## OUTPUT FORMAT (mandatory — the judge + scorer parse this)

Emit a Markdown table, ONE row per finding, then a TOTAL line. Use `~` only inside cells if needed:

| # | severity | task | file:line (evidence) | issue (1 sentence) | remediation (concrete, applicable) |
|---|----------|------|----------------------|--------------------|------------------------------------|
| 1 | BLOCKING/MAJOR/MINOR | 0NN | path:line | … | … |

End with: `FINDINGS: <n>` and `SELF-ASSESSMENT: <one line on why these survive peer review>`.
If you genuinely find NOTHING real after a thorough pass, output `FINDINGS: 0` and one line on what
you checked — an honest zero is better than a rejected nit.
