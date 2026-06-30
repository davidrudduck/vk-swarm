# Breakdown review — round 1

Three challengers in fresh contexts (cross-model), attacking the breakdown against the pushed repo.

| Model | Verdict | Headline findings |
|---|---|---|
| Opus (claude-opus) | REVISE | SC18 badges-row/title unimplemented but claimed; task 003 ANSI dither uses bare triplets as raw CSS colours (won't render); SC7 grep count wrong; 001 Before omits `--_neutral` lines |
| Codex (codex-rescue) | REVISE | SC18 footer Open-in-IDE + node/labels deferred; task-detail header has two components (TaskPanelHeaderActions vs AttemptHeaderActions); 020 i18n keys mismatched (Logs→"Terminal"); 022 `files:[]` vs ledger-record |
| Gemini (gemini-agent) | REVISE | SC7 `--border-strong` defined but never consumed; SC18 clauses deferred without named follow-up; SC10 `text-muted` vs `text-muted-foreground`; residual stale-spec var()/hsl() shorthand |

## Resolution (all REVISE findings addressed — see decisions-ledger "round 1")

- **SC18 — user chose "implement fully":** 020 expanded (literal tab labels + StatusBadge dot +
  status/node/labels badges via `useTaskLabels`); 021 expanded (ghost-sm Open-in-IDE via
  `useOpenInEditor`). Control-flow verified: `AttemptHeaderActions` (attempt view) is the correct
  component for the mode switcher; title already in the breadcrumb (no duplicate). Badges render as an
  inline header cluster (NewCardHeader's actions slot is top-right); literal below-header band noted as
  cosmetic follow-up.
- **Task 003 dither:** wrapped `--background`/`--border` in `hsl(...)`.
- **SC7 consumed clause:** 006 adds `hover:border-[hsl(var(--border-strong))]`; 022 greps consumption.
- **Spec re-frozen (2nd precheck):** text-muted→text-muted-foreground; var()→hsl() shorthand; SC7 grep
  count; NodeCard offline dot token.
- **Minor:** 001 range relabel + `--_neutral` note; 020 i18n literal labels; 022 `files:` += ledger.

Round 2 (re-review) dispatched after remediation + push.
