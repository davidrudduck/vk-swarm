# Breakdown review — round 2 (re-review after round-1 remediation)

| Model | Verdict | Notes |
|---|---|---|
| Opus | REVISE | 1 real finding: SC18 status collapsed to one badge (spec wants header dot + row outline+dot). All other round-1 fixes confirmed correct. |
| Gemini | REVISE | 2 findings: 022 stale Open-in-IDE note; spec:286 residual bare `bg-[var(--surface-card)]`. All round-1 fixes confirmed. |
| Codex | REVISE (DISCARDED) | Review-type error: checked the *source files* for the not-yet-written changes and reported them "missing". 12 of 13 invalid at decompose time. Its #13 (022 Open-in-IDE note) independently confirmed by Gemini. |

## Resolution (see decisions-ledger "round 2")

- SC18: task 020 now renders BOTH the header dot and the row outline status badge; grep asserts 2.
- Task 022: Open-in-IDE note + SC17 item corrected (button is in the footer, task 021).
- Spec: `bg-[var(--surface-card)]` → `hsl(...)`; full sweep clean; re-frozen (3rd precheck).

Note for round 3: confirm the SC18 two-badge fix is consistent (020 grep = 2; StatusBadge className
outline path), and that no bare token shorthand remains anywhere.
