---
id: "201"
phase: 2
title: "Delete the node's NodeApiKeySection component and its OrganizationSettings mount"
status: ready
depends_on: ["101"]
parallel: false
conflicts_with: []
files:
  - frontend/src/components/org/NodeApiKeySection.tsx
  - frontend/src/pages/settings/OrganizationSettings.tsx
irreversible: true
scope_test: "frontend/src/pages/settings"
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

## Manual verification (record in decisions-ledger)

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
