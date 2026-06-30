# ADR 0006 — Retain the resizable task-detail workbench instead of the 460px drawer

- **Status:** Accepted
- **Date:** 2026-06-28
- **Workstream:** `ui-overhaul`
- **Spec:** `docs/superpowers/specs/2026-06-28-ui-overhaul.md`
- **Design source:** `dev-docs/designs/2026-06-28-ui-overhaul/design-spec.md` §"Task drawer (`panels.jsx`)"

## Context

The "Midnight Terminal" design handoff specifies the task-detail surface as a
**460px slide-from-right `aside` drawer** (`min(460px, 90vw)`, overlay scrim, `shadow-lg`,
header → tabs → content → footer action bar).

The shipped product does not have a drawer. Task detail is a **resizable multi-pane
workbench** (`frontend/src/components/layout/TasksLayout.tsx`):

```text
kanban  │  attempt  │  aux
```

- `kanban | attempt` split is user-resizable with sizes persisted to
  `localStorage` (`tasksLayout.desktop.v2.kanbanAttempt`).
- The `aux` pane hosts five rich modes selected via the `?view=` search param and the
  `LayoutMode` union: `preview`, `diffs`, `files`, `terminal`, `processes`
  (`ProjectTasks.tsx:370`, `:960-983`). Diff viewer, embedded terminal, file browser, and
  per-attempt execution-process list all live here.
- The `attempt | aux` split is independently resizable and persisted
  (`tasksLayout.desktop.v2.attemptAux`).
- Mobile collapses to a single pane with swipe-to-close (`useSwipe`).

A fixed 460px drawer cannot host a side-by-side diff viewer, an interactive terminal, and a
file browser without crushing them. Adopting the drawer geometry would **delete shipped
functionality**, not just restyle it.

## Decision

**Keep the resizable workbench architecture. Do not introduce a Sheet/Drawer wrapper for task
detail.** Apply the design spec to the *internals* of the existing right-hand work area:

**Adopted from the drawer spec (internal styling — these are in scope):**
- Header: `StatusBadge` dot + task title (`text-lg weight-semibold`) + close affordance.
- Badges row beneath the header: status (outline+dot), node (secondary), labels (outline).
- Labeled `Tabs` (Diff / Logs / Attempts) replacing the current icon-only `ToggleGroup`.
  Tab → `LayoutMode` mapping: Diff = `'diffs'`, Logs = `'terminal'`, Attempts = reset `mode`
  to `null` (no new union value — attempt-history list already renders at `mode=null`).
- Footer action bar styling: Merge (`variant="default" size="sm" flex-1`) ·
  Rebase (`variant="outline" size="sm"`) · Open in IDE (`variant="ghost" size="sm"`).

**Dropped from the drawer spec (geometry/behaviour — explicitly NOT in scope):**
- 460px fixed width / `min(460px, 90vw)` sizing — the panel stays resizable.
- Overlay scrim (`--surface-overlay`, z-index 10).
- Slide-from-right transform animation and `position: absolute right-0` mounting.

## Consequences

- **Positive:** No regression of the diff/terminal/files/processes workbench. No loss of the
  resizable, persisted layout users already rely on. Visual fidelity to the design system is
  still achieved on every in-panel element.
- **Negative:** A deliberate divergence from the preserved design source. The wireframe's
  drawer silhouette is not reproduced; reviewers comparing against the mockup will see a
  resizable panel where the mockup shows a 460px overlay. This ADR is the durable answer to
  "why didn't you build the drawer the designer specced?"
- **Reversibility:** Reversible. Nothing is deleted; should a true drawer be desired later, the
  styled internals are wrapper-agnostic and can be re-parented into a Sheet. Recorded as an ADR
  for convention parity with 0001–0005 and to settle the question once, not because it is
  irreversible.

## Alternatives considered

1. **Bolt a Sheet/Drawer around the existing panel content.** Rejected — a 460px overlay cannot
   accommodate the diff + terminal + files aux pane; would force removing or hiding the
   workbench, a functional regression.
2. **Two surfaces (drawer for summary, workbench for deep work).** Rejected — duplicate
   navigation, two code paths to maintain, no user request for it; out of proportion to a
   fidelity pass.
