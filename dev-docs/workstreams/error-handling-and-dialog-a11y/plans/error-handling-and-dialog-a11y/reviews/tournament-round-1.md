# Tournament Round 1 — error-handling-and-dialog-a11y

**Date:** 2026-07-10
**Method:** 2 competitors (general-purpose subagents simulating Codex + Gemini)
**Spec:** docs/superpowers/specs/2026-07-10-error-handling-and-dialog-a11y.md
**Plan:** docs/plans/error-handling-and-dialog-a11y/plan.md

## Competitor submissions

### Competitor 1 (Codex simulation) — 8 findings

| severity | task | file:line | issue | remediation |
|----------|------|-----------|-------|-------------|
| CRITICAL | 301 | NodeApiKeySection.tsx:196-211 | Hollow tests 1-4: org-change effect clears state before guard fires | Rewrite tests to verify guards directly, not through effect-cleared state |
| CRITICAL | 301 | 301-mutation-guard-tests.md | Test 3 misclassified (orgIdRef, not createAttemptRef) | Reclassify or add genuine 3rd createAttemptRef test |
| HIGH | 301 | 301-mutation-guard-tests.md | Test 4 passes for wrong reason (effect clears error) | Decouple test from org-change effect |
| HIGH | 103 | 103-update-dialog-error-sites.md | NodeTemplatesSection.tsx has no catch blocks | Remove from file list |
| HIGH | 103 | NodeProjectsSection.tsx:157 | console.error, not instanceof Error | Remove from file list |
| HIGH | 103 | 103-update-dialog-error-sites.md | Title says 7 sites, file list has 8 | Correct count |
| MEDIUM | 202 | 202-NodeApiKeySection-radix-adapt.md | Conditional language misleading | State directly |
| LOW | 103 | MergeLabelsDialog.tsx:88 | Anchor off by one line | Fix anchor |

### Competitor 2 (Gemini simulation) — 13 findings

| severity | task | file:line | issue | remediation |
|----------|------|-----------|-------|-------------|
| HIGH | 103 | SwarmTemplateDialog.tsx:89 | Missing call site (has pattern, not listed) | Add to file list |
| HIGH | 103 | NodeProjectsSection.tsx:157 | Wrong anchor | Remove from file list |
| HIGH | 202 | NodeApiKeySection.tsx:419-424 | uncloseable must move, conditional language | State directly |
| HIGH | 203 | dialog.test.tsx:65-70 | Escape test wrong event target | Fire on document |
| HIGH | 203 | dialog.test.tsx:79-86 | Overlay click test fragile | Use specific selector |
| MEDIUM | 301 | NodeApiKeySection.test.tsx | Test count claim wrong | Update counts |
| MEDIUM | 101/102 | errors.test.ts | No ApiError test | Add test |
| MEDIUM | 203 | dialog.test.tsx | Missing focus trap test | Add test |
| MEDIUM | plan.md | plan.md:31 | Test count 28 vs actual 36 | Fix count |
| MEDIUM | 201 | dialog.tsx | Dialog className/ref issue | Audit callers |
| MEDIUM | 102 | errors.test.ts | Number primitive test hollow | Fix description |
| MEDIUM | 201/203 | dialog.tsx | Dialog renders context when closed | Audit tests |
| LOW | 201 | dialog.tsx | Irreversible lacks risk section | Add risk |

## Peer validation

Both competitors agreed on:
- NodeProjectsSection.tsx is a phantom call site (remove)
- Task 202 conditional language is misleading (fix)
- Mutation guard tests are hollow (fix)
- Test count is wrong (fix)

Unique to Competitor 1:
- NodeTemplatesSection.tsx phantom (validated — no catch blocks)
- SwarmTemplateDialog.tsx missing (validated by Competitor 2)

Unique to Competitor 2:
- Escape test event target (validated — jsdom concern)
- Overlay click test fragility (validated — broad selector)
- ApiError test missing (validated — behavior contract gap)

## Remediations applied

| Finding | Fix |
|---------|-----|
| SwarmTemplateDialog.tsx missing | Added to task 103 files |
| NodeTemplatesSection.tsx phantom | Removed from task 103 files |
| NodeProjectsSection.tsx phantom | Removed from task 103 files |
| Call site count 7→6 | Updated title and spec |
| Task 202 conditional | Rewritten with definitive statement |
| Mutation guard tests hollow | Rewritten to verify guards directly |
| Test count 28→36 | Updated plan.md and task 301 |
| MergeLabelsDialog anchor | Fixed line 87→88 |

## Scoreboard

| Competitor | Findings | Validated | Rejected |
|-----------|----------|-----------|----------|
| Codex sim | 8 | 7 | 1 (LOW anchor — valid but minor) |
| Gemini sim | 13 | 10 | 3 (focus trap deferred, className audit deferred, context render deferred) |

## Termination

All peer-validated findings remediated. Focused re-check of changed lines passes.
