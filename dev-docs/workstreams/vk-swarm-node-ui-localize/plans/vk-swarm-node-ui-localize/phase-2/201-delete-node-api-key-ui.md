---
id: "201"
phase: 2
title: "Delete the node's NodeApiKeySection component and its OrganizationSettings mount"
status: passed
depends_on: ["101"]
parallel: false
conflicts_with: []
files:
  - frontend/src/components/org/NodeApiKeySection.tsx
  - frontend/src/pages/settings/OrganizationSettings.tsx
irreversible: true
scope_test: "N/A"
allowed_change: mixed
covers_criteria: [SC3]
---

## Failing test (write first)

N/A — this is a deletion. Coverage is the existing `frontend/` suite staying green, plus the
grep assertions in Manual verification.

## 🚧 IRREVERSIBLE — human gate

This deletes a live, admin-gated, user-facing surface. Per ADR-0013 the hive owns node API-key
management (`hive-node-api-key-ui`, shipped), and operators mint keys there. Surface the diff and
wait for approval before running.

## Amendments (ORCHESTRATOR, pre-dispatch — these are DICTATED, not choices)

**A1 — `scope_test` changed from `frontend/src/pages/settings` to `N/A`.** The declared gate was
both VACUOUS and IMPOSSIBLE, so it had to be corrected rather than satisfied:

- *Vacuous:* the only test in that directory referencing `OrganizationSettings` is
  `src/pages/settings/__tests__/SettingsMobile.test.tsx:26`, and it does
  `vi.mock('../OrganizationSettings', ...)` — it STUBS the component out and never renders the
  real one. Deleting a child of `OrganizationSettings` cannot change its result. The gate would
  have "covered" this task without exercising one line of it.
- *Impossible:* the gate is red before this task makes any change (see A2), so it could never pass.

This is the same class of correction as the D1 mock-body amendments in tasks 102-104 — a
decompose-time defect in MY task text. It is explicitly NOT the task-103 case, where the text was
right and the implementer deviated; there the CODE was corrected and the text left alone. The
distinction: at 103 the contract was correct; here the named gate cannot pass and does not cover
the change. Verification falls to the `## Manual verification` block below, which plan.md already
sanctions as the primary mechanism for tasks in this plan.

**A2 — the `frontend` vitest suite is RED AT BASELINE. Assert the DELTA, not the total.** Captured
immediately before dispatch, on a clean tree, with no change from this task:

```text
 Test Files  8 failed | 26 passed (34)
      Tests  15 failed | 408 passed (423)
```

The 8 failing files are: `BottomNav`, `MessageQueuePanel`, `ConversationFocusMode`, `taskSorting`,
`SettingsMobile`, `SystemSettings`, `DesignSystem`, `MobileIntegration`. None is related to this
workstream — phase 1 changed only Rust and docs. Filed as F-2026-07-31-01..02 (the two under
`pages/settings`) and F-2026-07-31-03 (the other six) in `dev-docs/BACKLOG.md`.

**Do NOT try to fix them, and do NOT read "suite is red" as permission to ignore regressions.**
Your job is to prove your deletion changed NOTHING about that set: after the change, re-run
`npx vitest run` and confirm the failing set is IDENTICAL — same 8 files, same 15 assertions. Any
new failure, or any change in the counts, is a STOP.

Note `npx tsc --noEmit` IS green at baseline (exit 0), so it remains a real, load-bearing gate for
this task — a deletion that breaks typing WILL be caught there. `frontend` vitest is not among the
PR gates CLAUDE.md requires (it lists frontend lint + tsc, and remote-frontend vitest); `tsc` is.

**A3 — the `isAdmin` STOP trigger is expected to stay quiet, and here is the proof.**
`frontend/tsconfig.json:16-17` sets `noUnusedLocals: true` and `noUnusedParameters: true`, so an
orphaned variable is a HARD `tsc --noEmit` error, not a warning. `isAdmin` is read at
`OrganizationSettings.tsx` lines 82, 293, 341, 366 and 383 — all outside the deleted block — so it
survives. If `tsc` nonetheless reports `isAdmin` unused, something other than this deletion
changed: STOP, do not "fix" it by removing the declaration.

## Change

### 1. Delete the component

- **File:** `frontend/src/components/org/NodeApiKeySection.tsx`
- **Action:** delete the file (`git rm`).

### 2. Remove its import from `OrganizationSettings.tsx`

- **File:** `frontend/src/pages/settings/OrganizationSettings.tsx`
- **Anchor:** line 37
- **Before:**
```typescript
import { NodeApiKeySection } from '@/components/org/NodeApiKeySection';
```
- **After:** (line deleted)

### 3. Remove its mount from `OrganizationSettings.tsx`

- **Anchor:** line 379-381, the block immediately after the closing `)}` of the members card and
  immediately before the `{selectedOrg && isAdmin && !isPersonalOrg && (` danger-zone card
- **Before:**
```typescript
      {selectedOrg && (
        <NodeApiKeySection organizationId={selectedOrg.id} isAdmin={isAdmin} />
      )}
```
- **After:** (block deleted, including its surrounding blank line)

## Allowed moves

- Delete `frontend/src/components/org/NodeApiKeySection.tsx`.
- Delete exactly the one import line and the one JSX block above.

## STOP triggers

- If `isAdmin` becomes an unused variable after the deletion — it is used by the danger-zone
  card below, so this should NOT happen. If it does, STOP: something else changed.
- If any other file in `frontend/src` imports `@/components/org/NodeApiKeySection`.
- **Do NOT touch `remote-frontend/`.** It has its own
  `remote-frontend/src/components/swarm/NodeApiKeySection.tsx`, which is the hive's copy and must
  survive (SC7). This task's `files:` list is the whole permitted blast radius.

## Manual verification (emit verbatim; the ORCHESTRATOR records it)

```bash
cd frontend && npx vitest run
# Expected: all tests pass

cd frontend && npx tsc --noEmit
# Expected: no output

grep -rn 'NodeApiKeySection' frontend/src
# Expected: NO output

ls remote-frontend/src/components/swarm/NodeApiKeySection.tsx
# Expected: the file still exists (the hive's copy is untouched — SC7)
```

Note: `forbid_after: ["NodeApiKeySection"]` is deliberately NOT set on this task — `forbid_after`
greps the whole repo and would fire on the hive's copy, which must survive.

## Done when

- `frontend/src/components/org/NodeApiKeySection.tsx` no longer exists.
- `grep -rn 'NodeApiKeySection' frontend/src` returns nothing.
- `remote-frontend/src/components/swarm/NodeApiKeySection.tsx` is untouched.
- `frontend` vitest and `tsc --noEmit` are clean.
