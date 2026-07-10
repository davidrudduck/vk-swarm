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

### Pre-existing test failures

Two test failures exist on the baseline (verified via `git stash` test):
- `scripts/no-push-invariant.test.mjs` — pre-existing, not caused by this workstream
- `src/AppRouter.test.tsx > 'authenticated: hitting / redirects to /nodes'` — pre-existing

These are documented but NOT fixed in this workstream (out of scope per the plan).

## Reachability gate

### (a) Call-path trace

Entry point: 6 swarm dialog catch blocks (SwarmLabelDialog:88, MergeProjectsDialog:73,
MergeLabelsDialog:89, MergeTemplatesDialog:73, SwarmProjectDialog:78, SwarmTemplateDialog:90)
+ 3 NodeApiKeySection mutation onError callbacks (lines 207, 220, 232).

Path: `catch (err)` → `parseErrorMessage(err)` → `remote-frontend/src/lib/errors.ts`
(shared utility). Confirmed via grep: all 9 call sites use `parseErrorMessage(err)`.

The change executes on the production error path: when any dialog mutation fails, the catch
block calls `parseErrorMessage` which handles Error, string, null, symbol, JSON body, and
circular refs — returning a user-friendly string instead of raw JSON.

### (b) Real-seam test

- Test TS7: `createApiKey` rejects → `parseErrorMessage(new Error('boom'))` → Alert shows "boom"
- Tests TS16a-h: 8 test cases exercise `parseErrorMessage` through the real component integration
  (string, null, symbol, plain object, JSON body with {message}, JSON body with {error}, JSON
  string primitive, circular reference)
- Tests TS13, TS17: revoke/unblock mutation errors also flow through `parseErrorMessage`

These are NOT isolated unit tests — they exercise the full component→mutation→error→display path.

### (c) Incident-symptom assertion

Symptom: raw JSON bodies shown to users when dialog mutations fail (e.g., `{"message":"denied"}`).
Fix: `parseErrorMessage` extracts "denied" from JSON body. Evidence: TS16d asserts
`getByText(/server denied/)` passes when the mutation rejects with `new Error('{"message":"server denied"}')`.
