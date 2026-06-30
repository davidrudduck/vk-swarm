# Code Review — Round 2

**Target:** worktree-bridge-cse_01SHcfL9XSiArcLSCnKmJ49p   **Range:** `main..HEAD`   **Effort:** high

Round-1 remediated 5 actionable findings. Three parallel finder subagents reviewed the fixed diff.

## Findings

| # | File:line | Severity | Category | Finding | Confidence | Actionable? |
|---|-----------|----------|----------|---------|-----------|-------------|
| 1 | `frontend/src/components/tasks/AllProjectsTaskCard.tsx:120` + `frontend/src/components/tasks/TaskCard.tsx:240` | low | quality | `Loader2` spin icon still uses hardcoded `text-blue-500` while its sibling status icons (`CheckCircle`, `XCircle`) were migrated to semantic tokens in round-1. `text-status-inprogress` (defined in tailwind.config.js as `hsl(var(--status-inprogress))`) is the theme-aware equivalent; both files are in the diff scope. | high | yes |

## Non-actionable

| # | File:line | Severity | Category | Finding | Confidence | Why non-actionable |
|---|-----------|----------|----------|---------|-----------|---------------------|
| 2 | `frontend/src/components/labels/LabelBadge.tsx:92` | low | correctness | Nested `<button>` (onRemove) inside `role="button"` span (onClick) violates ARIA. Latent only — no caller currently passes both `onClick` and `onRemove` simultaneously. Callers verified. | medium | No current manifestation; purely theoretical. Log as a warning if both props are ever wired together. |
| 3 | `frontend/src/styles/index.css:117` | low | quality | `--shadow-sm`, `--shadow-md`, `--shadow-lg` added by Task 003 have no consumers (`hover:shadow-md` on TaskCard uses Tailwind's built-in, not these tokens). | high | Intentional design-system vocabulary; forward scaffolding added per spec. Not dead in the "should be deleted" sense. |
| 4 | `frontend/src/styles/index.css:121` | low | quality | `--glow-emerald` added by Task 003 has no consumers (only `--glow-cyan` is consumed by `button.tsx`). | high | Same as #3 — intentional token vocabulary scaffolding. |
| 5 | `frontend/src/styles/index.css:76` | low | quality | `--strip-width: 4px` defined in `:root` has no consumers. | high | Pre-existing; not introduced by this diff. |
| 6 | `frontend/tailwind.config.js:102` | low | quality | `status.*` Tailwind color group has no consumers; all components use `bg-[hsl(var(--status-*))]` arbitrary values directly. | high | Pre-existing pattern. The group provides a cleaner forward-compatible API and is harmless. |
| 7 | `frontend/src/styles/index.css:110` | low | correctness | `--status-*`, `--surface-*`, `--border-strong` are defined only in `.dark`/`.light`, not in `:root`. Brief pre-hydration window where these tokens are undefined. | medium | Pre-existing architectural pattern. Transient FOUC only; ThemeProvider resolves on mount. |
| 8 | `frontend/src/components/ThemeToggle.tsx:23` | low | quality | When OS/system theme is dark and the stored preference is SYSTEM, `isDark` evaluates to false; the toggle shows the Sun icon rather than reflecting the actual rendered appearance. | medium | Documented limitation ("Toggles binary DARK↔LIGHT only, never SYSTEM"). Pre-existing and intentional. |
| 9 | `frontend/src/components/panels/AttemptHeaderActions.tsx:84-104` | low | quality | Keyboard view-cycle in `ProjectTasks.tsx:558` (`[null,'preview','diffs','files']`) does not include `terminal`/`logs`; the Tabs switcher only accepts diffs/logs/attempts — inputs from other sources fall through to `_none`. | low | Intentional SC18 scope reduction; documented in decisions-ledger (task 020 processes-mode note). Content panels still render correctly. |

## Verdict: With fixes

Actionable: [1]
