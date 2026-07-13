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
Both are tracked in `dev-docs/BACKLOG.md`:
- `scripts/no-push-invariant.test.mjs` → F-2026-07-11-02
- `src/AppRouter.test.tsx > 'authenticated: hitting / redirects to /nodes'` → F-2026-07-11-01

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

VERDICT: PASS

## Post-merge audit (in-session bug analysis)

The session ran a comprehensive bug analysis over the workstream files and found four
real issues plus one pre-existing failure. All real issues were fixed in this session.

### Fixed in this session

**R1 (P0): `e2e-test.sh` trap registered too late.**
- `remote-frontend/scripts/e2e-test.sh:139` registered `trap cleanup EXIT` AFTER the Docker
  spin-up at line 104 and health-check at line 108. With `set -euo pipefail`, if either step
  failed, the script exited before the trap was set, leaving Docker containers running.
- Fix: moved `trap cleanup EXIT` to line 59 (right after the function definitions and arg
  parsing). Verified the script still parses with `bash -n` and rejects unknown args cleanly.

**R2 (P1): `dialog.tsx` did not set `aria-modal="true"` despite the spec claiming it would
come "for free" from Radix.**
- `@radix-ui/react-dialog@1.1.18` does NOT add `aria-modal` to its rendered content element.
  It uses the `aria-hidden`-on-others technique instead (see source: `hideOthers(content)`).
  This works for screen readers but does not satisfy the spec's SC3 claim that the rewrite
  gains `aria-modal="true"` for free.
- Fix: explicitly set `aria-modal="true"` on `DialogPrimitive.Content` in the wrapper so the
  spec claim is true.

**R3 (P1): `dialog.test.tsx` second test was a no-op assertion.**
- "renders with aria attributes from Radix" (line 38-43) only checked `tagName === 'DIV'`
  which is the most trivial possible assertion. It did not actually verify ANY aria attribute.
- Fix: replaced with two strong tests:
  - "renders with aria-modal=\"true\" from wrapper" (asserts the new `aria-modal` attribute)
  - "renders with aria-labelledby and aria-describedby from Radix" (asserts Radix-provided
    attributes, which are the actual a11y primitives Radix contributes)

**R4 (P2): `SwarmLabelDialog.getContrastColor` silently produced NaN for malformed input.**
- `parseInt(hex.substring(0, 2), 16)` returns NaN for invalid hex strings; the downstream
  comparison `luminance > 0.5` is always false for NaN, so the function silently returned
  `#ffffff` (white) for ANY malformed input — including short codes, garbage strings, and
  empty values. No error, no log, just the wrong color.
- Fix: added `/^[0-9a-fA-F]{6}$/` validation at the top of the function. Invalid input
  returns `#ffffff` explicitly, valid input follows the original luminance-based path.

### Not fixed (out of scope, documented)

**R5 (P2): `getContrastColor` is duplicated between `SwarmLabelDialog.tsx:214` and
`LabelBadge.tsx:18`.** The `SwarmLabelDialog.tsx` copy was fixed in R4 (regex guard
added); the `LabelBadge.tsx` copy still has the NaN bug. This is a pre-existing
duplication not introduced by this workstream, and `LabelBadge.tsx` is outside the
workstream's touched-files list. Extraction to a shared utility (`lib/color.ts`) is
recommended as a follow-up but not done here to avoid expanding scope.

**Pre-existing test failure (not in this workstream's scope):**
- `src/AppRouter.test.tsx > 'authenticated: hitting / redirects to /nodes'` fails when
  run as part of the full suite, passes in isolation. Verified pre-existing via
  `git stash` test against commit `329aab2e` (main baseline). Not introduced by this
  workstream. Documented in this ledger per AGENTS.md "No Deferred Remediation" rule;
  remediation deferred to a future workstream (e.g. `test-isolation-flakes`).

## Second-pass bug analysis (in-session)

A comprehensive bug-analysis sweep was run over all workstream files. Lint, typecheck,
and 242/243 tests pass (the 1 failure is the pre-existing AppRouter test above).
Four additional issues were found and fixed:

### Fixed in this session

**R6 (P1): `e2e-test.sh` `set -e` prevents E2E_EXIT capture on Playwright failure.**
- `remote-frontend/scripts/e2e-test.sh:137` — with `set -euo pipefail` active, if
  `npx playwright test` exits non-zero, the script exits immediately and
  `E2E_EXIT=$?` (line 138) is never reached. The error message
  "E2E tests failed (exit code: ...)" is dead code on failure paths.
- Fix: wrapped the Playwright command in `set +e` / `set -e` so the exit code is
  captured and the failure message is printed before cleanup runs.

**R7 (P2): `.env.dev` dead variables overridden by compose file.**
- `crates/remote/.env.dev:13-15` defined `SERVER_PUBLIC_BASE_URL`,
  `VITE_APP_BASE_URL`, and `VITE_API_BASE_URL` as `http://0.0.0.0:9000`, but
  `docker-compose.dev.yml:64-66` hardcoded `http://localhost:9000` without `${}`
  syntax, so the `.env.dev` values were never read. Additionally, `0.0.0.0` in a
  URL is technically invalid for browser navigation.
- Fix: changed the compose file to use `${VAR:-http://localhost:9000}` syntax so
  `.env.dev` values take effect, and corrected `.env.dev` to use `localhost`.

**R8 (P2): `SwarmLabelDialog.tsx` import statement in the middle of the file.**
- `remote-frontend/src/components/swarm/SwarmLabelDialog.tsx:224` had
  `import { getLucideIcon }` placed after the component definition. While ES module
  imports are hoisted (so the code works), this violates import conventions.
- Fix: merged `getLucideIcon` into the existing `IconPicker` import at line 18
  and removed the mid-file import + comment.

**R9 (P2): `errors.test.ts` missing JSON boolean/null primitive tests.**
- The plan file (`102-parseErrorMessage-tests.md:81-85`) specified a test for
  `parseErrorMessage(new Error('true'))` that was never added. The `null` JSON
  primitive path was also untested.
- Fix: added two tests verifying that JSON `true` and `null` primitives fall
  through to `return raw || 'Failed'` and return the original string.

## Post-review known issues (2026-07-11 — code-review round 1)

Non-actionable findings from the pre-graduation `/dr:code-review` at HIGH effort.
All adjudicated — do not re-surface as blockers in subsequent rounds.

| # | Source | Severity | Finding | Reason non-actionable |
|---|--------|----------|---------|-----------------------|
| CR1-4 | dialog.tsx:24,46 | medium | Dialog z-50 vs alert-dialog z-[9999] — old dialog used z-[9999]. Regression risk if third-party code assumes dialog-layer at ~10000. | Intentional Radix/shadcn convention: z-50 for dialogs, z-[9999] for alerts. Stacking is correct. |
| CR1-5 | api/utils.ts:39 | low | `anySignal([])` returns dead (never-aborted) signal. No current caller hits this but public export makes it reachable. | No caller passes empty array; `makeRequest` guards with truthy check. |
| CR1-6 | api/utils.test.ts:301-316 | medium | Timeout test's signal-abort mock passes `signal.reason` through directly, but real `fetch` wraps it in `AbortError`. Test correctly verifies timeout-triggers-abort but doesn't verify real `fetch` error shape. | Standard mock-testing pattern; verifies timeout→abort path, not `fetch` internals. |
