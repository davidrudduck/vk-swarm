# Phase 2 — Component fidelity

Switches components from hardcoded Tailwind colours to the Phase-1 tokens and corrects
typography/geometry. The A-class bugs (TaskCountPills swap, kanban add-button) ship here.

Tasks:
- 005 — TaskCard + AllProjectsTaskCard status strips → `var(--status-*)` → SC5a, SC5b (dep 002)
- 006 — TaskCard typography/meta + TaskCardHeader title (dep 005, same files)
- 007 — TaskCountPills inprogress/inreview swap → SC8 (A-class)
- 008 — DaysInColumnBadge flat + literal `{n}d` (no cap)
- 009 — LabelBadge outline variant
- 010 — Kanban card-state fixes (ring, add-button h-0→h-6 [A-class], status dot, count bg)
- 011 — VKSLogo → `font-wordmark` (dep 004)
- 012 — Create `ThemeToggle` → SC16 (create)

Shippable boundary: task cards and kanban cards match the design; theme can be toggled (once the
toggle is placed in the navbar in Phase 3).
