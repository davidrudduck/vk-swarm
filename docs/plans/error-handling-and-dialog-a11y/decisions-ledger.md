# Decisions Ledger — error-handling-and-dialog-a11y

> Implementer appends here for ANY choice the task didn't dictate. Empty section = perfect.

## Pre-existing decisions (from spec)

| Decision | Source | Reversible? |
|----------|--------|-------------|
| D1: Replace dialog.tsx with Radix | spec/ADR-0012 | Irreversible |
| D2: parseErrorMessage uses 'Failed' fallback | spec | Reversible |
| D3: uncloseable via Radix event prevention | spec | Reversible |
| D4: Update AGENTS.md with remote-frontend gates | spec | Reversible |

## Implementer decisions

### Advisory sibling warnings (plan-lint W: lines)

**W: task 101/102** — `mutation-queue.test.ts` is a test file in the same `src/lib/` directory.
It is NOT a sibling pattern to `errors.ts` (one is a test file, the other is a utility module).
No divergence to justify — different concerns entirely.

**W: task 203** — `alert-dialog.tsx` is in the same `src/components/ui/` directory. It is a
separate component (Radix AlertDialog), not a sibling pattern to `dialog.test.tsx`. The test
file tests `dialog.tsx`, not `alert-dialog.tsx`. No divergence to justify.
