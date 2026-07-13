# error-handling-and-dialog-a11y — Execution Plan

## Approach

Three deliverables across four phases. Phase 1 extracts and shares the error parsing utility.
Phase 2 rewrites dialog.tsx on Radix. Phase 3 adds mutation guard tests. Phase 4 updates
AGENTS.md with the missing remote-frontend gates.

Each phase is shippable independently. Phases 1-2 are independent of each other. Phase 3
depends on Phase 2 (dialog rewrite may affect test rendering). Phase 4 is independent.

## Phases

| Phase | Name | SCs | Tasks |
|-------|------|-----|-------|
| 1 | Shared parseErrorMessage | SC1, SC2 | 101-103 |
| 2 | Dialog accessibility via Radix | SC3 | 201-203 |
| 3 | Mutation guard tests | SC4, SC5, SC6 | 301 |
| 4 | AGENTS.md gates | SC7 | 401 |

## Task summary

| id | title | phase | dep: | conflicts: |
|----|-------|-------|------|------------|
| 101 | Create shared parseErrorMessage in src/lib/errors.ts | 1 | dep: - | conflicts: none |
| 102 | Create parseErrorMessage unit tests | 1 | dep: 101 | conflicts: none |
| 103 | Update all 6 dialog error call sites to use shared parseErrorMessage | 1 | dep: 101 | conflicts: none |
| 201 | Rewrite dialog.tsx to use @radix-ui/react-dialog | 2 | dep: - | conflicts: none |
| 202 | Update NodeApiKeySection for Radix dialog API | 2 | dep: 201 | conflicts: none |
| 203 | Create dialog a11y tests | 2 | dep: 201 | conflicts: none |
| 301 | Add mutation guard tests for createAttemptRef and orgIdRef | 3 | dep: 202 | conflicts: none |
| 401 | Update AGENTS.md with remote-frontend gates | 4 | dep: - | conflicts: none |
