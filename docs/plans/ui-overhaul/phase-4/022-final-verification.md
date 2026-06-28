---
id: "022"
phase: 4
title: Final manual verification — static CI gates, greppable assertions, SC17 smoke-test
status: ready
depends_on: ["005", "006", "007", "008", "009", "013", "015", "016", "017", "019", "020", "021"]
parallel: false
conflicts_with: []
files: []
irreversible: false
scope_test: "N/A"
allowed_change: edit
covers_criteria: [SC1, SC2, SC3, SC4, SC17, SC19, SC20]
---
## Failing test (write first)
N/A — verification-only task. No code change (`files: []`); nothing to test-drive. The work here is
running the static gates, the greppable assertions, and the manual browser smoke-test, then
recording the results in the decisions-ledger.

## Change
None. This task commits no source code (`files: []`). It is a manual verification gate over the
work landed by Phases 1–3. The gate's "no file outside `files:`" check passes trivially because no
file is modified.

## Allowed moves
- NONE (no code change). Only running commands and recording results in the decisions-ledger.

## STOP triggers
- Any static gate fails (clippy/test/lint/tsc) — STOP; the failing change must be fixed in this
  session (per CLAUDE.md "No Deferred Remediation"), not deferred.
- Any greppable assertion does not match its expected result — STOP; the producing task regressed or
  did not land. Reconcile before signing off.
- Any SC17 checklist item fails or any console error appears — STOP; record and fix.

## Manual verification (record in decisions-ledger)
This is a **verification-only task**. Run each command from the **repo root** (worktree root) and
record pass/fail in the decisions-ledger.

### Static CI gates (SC1–SC4) — all four must be green
```bash
cargo clippy --all --all-targets --all-features -- -D warnings   # SC1
cargo test --workspace                                           # SC2
cd frontend && npm run lint                                      # SC3
cd frontend && npx tsc --noEmit                                  # SC4
```

### Greppable assertions (each must match its expected result)
```bash
# SC5a — old hardcoded status colours gone (expected: zero matches)
grep -r 'bg-green-500\|bg-red-500\|bg-amber-500\|bg-blue-500' \
  frontend/src/components/tasks/ \
  frontend/src/components/projects/TaskCountPills.tsx

# SC5b — token replacement confirmed in key files (expected: ≥1 match per file)
grep -r 'var(--status-' \
  frontend/src/components/tasks/TaskCard.tsx \
  frontend/src/components/tasks/AllProjectsTaskCard.tsx

# SC6 — status tokens defined under .dark (not .vks-theme) (expected: match)
grep -A 100 '\.dark {' frontend/src/styles/index.css | grep 'status-todo'

# SC7 — semantic tokens defined (expected: 3 matches)
grep -E '\-\-(border-strong|surface-card|surface-raised)' frontend/src/styles/index.css

# SC9 — kanban column min-width (expected: match)
grep 'minmax(264px' frontend/src/components/ui/shadcn-io/kanban/index.tsx

# SC12 — VKSLogo in Navbar (expected: match)
grep '<VKSLogo' frontend/src/components/layout/Navbar.tsx

# SC14 / SC21 — NodeCard component and /nodes route (expected: both present)
ls frontend/src/components/swarm/NodeCard.tsx
grep 'path="/nodes"' frontend/src/App.tsx

# SC15 — vks-pulse keyframe (expected: match)
grep 'vks-pulse' frontend/src/styles/index.css

# SC22 — brand palette live under .dark (expected: match)
grep -A 40 '\.dark {' frontend/src/styles/index.css | grep -- '--_primary: var(--vks-cyan)'
```

### SC17 — manual browser smoke-test
Start the dev server and open the frontend, then work through every checklist item with browser
DevTools open (zero console errors required):
```bash
pnpm run dev   # from worktree root; open http://localhost:3000 (or the port from .env)
```
- [ ] Projects page renders
- [ ] Tasks kanban renders; status strips show correct colours (not hardcoded green/red/amber)
- [ ] Task detail panel opens; header shows StatusBadge dot + task title + close button
- [ ] Task detail: Diff / Logs / Attempts tabs (labeled, not icon-only)
- [ ] Task detail footer: Merge (filled primary), Rebase (outline), Open in IDE (ghost)
- [ ] Nodes tab in Navbar navigates to `/nodes`; node cards render
- [ ] Board / Nodes / Processes tab row switches between views
- [ ] Theme toggle flips dark ↔ light; token colours update correctly
- [ ] Hard-reload after theme toggle: persists (confirms `updateAndSaveConfig` write path)

> Note on the "Open in IDE (ghost)" checklist item: per task 021, no Open-in-IDE button exists in
> the `GitOperations` footer — it lives in `ActionsDropdown`. Verify the affordance is present and
> ghost-styled wherever it actually renders, or record that it is intentionally in the dropdown.

### SC19 — WCAG AA spot-check (browser DevTools accessibility panel)
On Projects, Tasks kanban, and Nodes pages, in **both** dark and light themes, spot-check that
introduced/changed text↔background pairs meet 4.5:1 (normal text) / 3:1 (large text / UI). Record
any failures.

### SC20 — theme persistence
Toggle to light, then hard-reload `/projects`; confirm the light theme persists (confirms the
`updateAndSaveConfig` write, not just in-memory state).

### SC22 — runtime check
With the dark theme applied, confirm the `VKSLogo` "VK" (rendered with `text-primary`) appears
**cyan**, not gray.

### Cleanup between iterations
```bash
rm -rf target node_modules frontend/node_modules
```

## Done when
All static gates green, all greppable assertions match their expected results, the SC17 checklist
is complete with zero console errors, and SC19/SC20/SC22 checks pass — with results recorded in the
decisions-ledger.

Because this task changes no files (`files: []`), the task-gate runs as a manual sign-off:
```bash
echo "manual verification recorded in decisions-ledger" # all SC checks above pass
```
Fallback (real task-gate, for tooling parity — runs the inline typecheck only since there is no
scoped test and no diff to police):
```bash
WAI_TYPECHECK_CMD="cd frontend && npx tsc --noEmit" WAI_TEST_CMD="true" bash ~/.claude/wai/scripts/task-gate.sh ui-overhaul 022
```
