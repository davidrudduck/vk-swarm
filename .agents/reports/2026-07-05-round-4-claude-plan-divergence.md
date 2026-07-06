I'm in plan mode which prevents writing to `.agents/reports/`. My findings are documented in the plan file. Here are the 5 divergences I found that both Codex and Gemini missed:

---

**CLD-001 — MEDIUM**: `Tasks.tsx` has an entirely unplanned `optimisticAssignsRef = useRef<Map<string, string>>(new Map())` overlay. Task 205 specifies only `useRef<Set<string>>` for deletions. The assign overlay pre-displays the selected node immediately (`Tasks.tsx:86,166-168`), rolls back on non-network errors (`:98`), and clears on replay success (`:65`). No ledger entry (D-L1..D-L18) documents it.

**CLD-002 — MEDIUM**: Task 200's `pwa.test.ts` specified `expect(source).toContain("'need-update'")` but the plan's own `pwa.ts` uses `wb.addEventListener('waiting', ...)`. The event `'need-update'` doesn't exist in workbox-window. The TDD "red test first" gate was broken by an internal plan inconsistency; D-L15's wholesale test replacement masked it without flagging it.

**CLD-003 — LOW**: `AppRouter.tsx:89-96` contains an undocumented `isSafeReturnTo(url)` function that validates `return_to` parameters against same-origin to prevent open redirect. Tasks 102 and 104 don't mention it; D-L6 covers the `return_to` query-param scheme but not the same-origin guard.

**CLD-004 — LOW**: `mock-electric.ts` exports an undocumented `MockNode` interface (lines 11-22) absent from task 300's spec. Codex PD-021 caught the `TableData` signature change but neither reviewer flagged the orphaned `MockNode` export that has zero importers.

**CLD-005 — LOW**: `Tasks.test.tsx` uses a `collection._mockRows` injection pattern (lines 4-21) instead of the per-test `mockedUseLiveQuery.mockReturnValue(...)` the plan specifies. All describe blocks share fixed data rather than allowing per-test shapes. Task 103's failing-test block explicitly shows per-test data customization.

---

The JSON payload matching the requested format is in the plan file. To write the actual report file at `.agents/reports/2026-07-05-round-4-claude-plan-divergence.md`, plan mode needs to exit. The task instruction says not to modify files — the report file is the only intended write. Please approve or confirm that I should write it.