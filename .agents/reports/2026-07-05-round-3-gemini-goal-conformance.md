# Goal Conformance Report: vk-swarm-hive-ui-polish

## Summary
The implementation was reviewed against the spec `2026-07-05-vk-swarm-hive-ui-polish.md` and the plan `plan.md`. While the majority of the code works as intended, there are a few significant gaps related to dead code and missing features that violate the success criteria.

## Findings

### Improvement 1: Error Resilience Layer
- **Finding 1: Missing Undo Action in Delete Toast (SC4)**
  - **Severity:** HIGH
  - **Location:** `remote-frontend/src/pages/Tasks.tsx:117`
  - **Issue:** The spec requires that "Post-delete shows an undo toast with 5s timeout (if undoable)." The current implementation calls `toastSuccess('Task deleted');` without providing the required `undo` configuration action, failing SC4.
  - **Remediation:** Implement a delayed deletion or restore mechanism, and pass the `undo` configuration to the toast:
    ```tsx
    toastSuccess('Task deleted', { undo: { label: 'Undo', onClick: handleUndoDelete } });
    ```

### Improvement 2: Offline-First PWA
- **Finding 2: Dead Code in Optimistic Mutations (SC8 & General Check)**
  - **Severity:** HIGH
  - **Location:** `remote-frontend/src/lib/electric/optimistic.ts`
  - **Issue:** The `optimisticDelete` and `optimisticUpdate` helpers were implemented but are completely unused. `Tasks.tsx` bypasses them, using local `useRef` states instead (as noted in the decisions ledger, due to a React Query vs. TanStack DB mismatch). Leaving these unused files violates the strict "No dead code" verification requirement.
  - **Remediation:** Remove `optimistic.ts` and its associated tests to eliminate the dead code, or refactor `Tasks.tsx` / `Nodes.tsx` to properly integrate and use these helpers.

### Improvement 3: Playwright E2E Test Suite
- **Finding 3: Missing Node Name Labels in Output Logs (SC14)**
  - **Severity:** HIGH
  - **Location:** `remote-frontend/src/pages/Tasks.tsx:165` & `remote-frontend/e2e/cross-node.spec.ts`
  - **Issue:** SC14 strictly requires that "output logs from different nodes render with node name labels." The `TaskDetail` component only renders the `output_type` and `content` without any node identification. Furthermore, `cross-node.spec.ts` is missing the corresponding test case for this requirement.
  - **Remediation:** Update `TaskDetail` to resolve and display the node name for each log entry:
    ```tsx
    <span className="font-mono text-xs uppercase">{l.output_type} - {nodeName}</span>
    ```
    Additionally, add the missing test case to `cross-node.spec.ts` to assert that output logs render with the correct node name labels.

## Conclusion
The implementation **does not meet the intended goals**. The blocking issues listed above must be remediated before the PR can be considered fully conformant with the specification.