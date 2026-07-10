---
doc_type: spec
status: active
workstream: error-handling-and-dialog-a11y
change_kind: behaviour
---

# error-handling-and-dialog-a11y — shared error parsing, dialog accessibility, mutation guard tests

> **Origin:** Post-ship improvements identified during `hive-node-api-key-ui` (PR #461) and
> `fix-nonloopback-signin` (PR #463) workstreams. Every tournament and code-review round flagged
> these gaps.
>
> **Relationship to other workstreams:** This is a quality/polish workstream with no feature
> dependencies. It touches shared infrastructure (`dialog.tsx`, error utilities) used by all
> swarm dialogs. The shared utilities will be consumed by `vk-swarm-design-system` when it ships.

## Intent (what / why)

The `remote-frontend` has three quality gaps that affect every dialog and mutation in the app:

1. **No shared error parsing.** Each dialog handles errors differently — some show raw JSON
   bodies to users, some show `err.message` (which may be `"Internal Server Error"`), some show
   generic strings. `NodeApiKeySection.tsx` has a local `parseErrorMessage` that handles Error,
   string, null, symbol, JSON body, circular refs, and primitive JSON values — but it's private
   to that file. The other 6 dialog call sites use bare `err instanceof Error ? err.message : 'An error occurred'`.

2. **Dialog has no a11y foundation.** `dialog.tsx` is a fully custom HTML implementation using
   plain `<div>` elements — it does NOT use `@radix-ui/react-dialog` (despite the package being
   installed as a dependency). The component lacks `role="dialog"`, `aria-modal="true"`, focus
   trapping, and Escape-to-close handling. The `uncloseable` prop (used by NodeApiKeySection's
   secret-reveal flow) prevents overlay click and hides the close button, but provides no
   accessible signal that the dialog is intentionally uncloseable.

3. **Mutation guard tests are missing.** The `createAttemptRef` stale-secret guard (prevents
   creating a key after the dialog has been closed and reopened) and the `orgIdRef` guard on
   `createMutation.onError` (prevents clearing form state when org has changed) have zero direct
   test coverage. These are the most subtle defenses in the component.

## Users / who is affected

- **End-users (operators/admins):** see raw error JSON or unhelpful error messages when dialogs
  fail. Screen reader users get no signal that a dialog exists, is modal, or is intentionally
  uncloseable. Keyboard users cannot Escape out of dialogs.
- **Developers:** duplicate error handling code across 6+ dialogs. No documented pattern for
  uncloseable dialogs. Mutation guard behavior is undocumented and untested.

## Success criteria

SC1: `parseErrorMessage` lives at `src/lib/errors.ts` and handles: Error (including `ApiError`
   with `error_data`), string, null, symbol, object with `error` key, object with `message`
   key, JSON body, circular refs, primitive JSON values. Returns a user-friendly string in
   all cases. 100% line coverage on the utility itself.

SC2: All 7 dialog error call sites (SwarmLabelDialog, MergeProjectsDialog, MergeLabelsDialog,
   MergeTemplatesDialog, SwarmProjectDialog, NodeTemplatesSection, NodeProjectsSection) use
   the shared `parseErrorMessage` instead of inline `instanceof Error` checks.

SC3: `dialog.tsx` is rewritten to use `@radix-ui/react-dialog` (already installed at `^1.1.18`),
   gaining `role="dialog"`, `aria-modal="true"`, focus trapping, and Escape-to-close for free.
   The `uncloseable` prop is preserved as a first-class variant that suppresses close-on-escape
   and close-on-overlay-click while maintaining focus trap and aria-modal.

SC4: `createAttemptRef` guard has at least 3 test cases: create-after-org-change,
   create-after-closeDialog, revoke-after-org-change.

SC5: `orgIdRef` guard on `createMutation.onError` has at least 1 test case.

SC6: All 28 existing NodeApiKeySection tests continue to pass. No regressions in dialog behavior.

SC7: `npm run lint`, `npx tsc --noEmit`, `npx vitest run` all pass locally.

## Constraints

- **No new dependencies.** `@radix-ui/react-dialog` is already installed. Error parsing is pure TS.
- **Backward compatible.** The shared `parseErrorMessage` must not change the user-visible message
  for any existing error path — only improve messages for paths that currently show raw JSON.
- **i18n scope.** The `settings.swarm.apiKeys.*` → `nodes.apiKeys.*` namespace rename is NOT in
  scope for this workstream. It's a separate concern (correctness, not quality).
- **Dialog API preserved.** The `Dialog`, `DialogContent`, `DialogHeader`, `DialogTitle`,
  `DialogDescription`, `DialogFooter` exports and their props must remain backward compatible.
  Existing callers must not need changes except to add the `uncloseable` prop where appropriate.

## Out of scope

- **i18n namespace rename** (`settings.swarm.apiKeys.*` → `nodes.apiKeys.*`) — separate workstream
- **Real-time key status** (WebSocket push or shorter staleTime) — separate workstream
- **Design system** (`vk-swarm-design-system`) — separate workstream, this workstream's shared
  utilities will be used by the design system when it ships
- **Backend error format changes** — we parse what the backend sends today
- **Deployed-host integration tests** — tests requiring a live Docker Compose stack or deployed
  host are deferred to a follow-up workstream after this PR is merged and deployed

---

## Approach

Three deliverables, executed in order. Each is independently testable and mergeable.

### Deliverable 1: Shared `parseErrorMessage` utility

**What:** Extract `parseErrorMessage` from `NodeApiKeySection.tsx:34-64` to `src/lib/errors.ts`.
Update all 7 dialog call sites to import and use it.

**Files touched:**
- NEW: `remote-frontend/src/lib/errors.ts` — the shared utility
- NEW: `remote-frontend/src/lib/errors.test.ts` — unit tests (100% line coverage)
- MODIFY: `remote-frontend/src/components/swarm/NodeApiKeySection.tsx` — remove local function, import from `@/lib/errors`
- MODIFY: `remote-frontend/src/components/swarm/SwarmLabelDialog.tsx:87` — replace inline check
- MODIFY: `remote-frontend/src/components/swarm/MergeProjectsDialog.tsx:72` — replace inline check
- MODIFY: `remote-frontend/src/components/swarm/MergeLabelsDialog.tsx:88` — replace inline check
- MODIFY: `remote-frontend/src/components/swarm/MergeTemplatesDialog.tsx:72` — replace inline check
- MODIFY: `remote-frontend/src/components/swarm/SwarmProjectDialog.tsx:77` — replace inline check
- MODIFY: `remote-frontend/src/components/swarm/NodeTemplatesSection.tsx` — replace inline check (if present)
- MODIFY: `remote-frontend/src/components/swarm/NodeProjectsSection.tsx:157` — replace inline check

### Deliverable 2: Dialog accessibility via Radix

**What:** Replace the custom `dialog.tsx` (plain `<div>` implementation) with a Radix-based
implementation using `@radix-ui/react-dialog`. The `uncloseable` prop becomes a first-class
variant.

**Files touched:**
- MODIFY: `remote-frontend/src/components/ui/dialog.tsx` — rewrite to use Radix primitives
- MODIFY: `remote-frontend/src/components/swarm/NodeApiKeySection.tsx` — adapt to any API changes
- NEW: `remote-frontend/src/components/ui/dialog.test.tsx` — a11y and behavior tests

### Deliverable 3: Mutation guard tests

**What:** Add focused test cases for `createAttemptRef` and `orgIdRef` guards in
`NodeApiKeySection.test.tsx`.

**Files touched:**
- MODIFY: `remote-frontend/src/components/swarm/NodeApiKeySection.test.tsx` — add 4+ new test cases

---

## Design / architecture

### 1. `parseErrorMessage` design

```typescript
// src/lib/errors.ts

/**
 * Parse an unknown error into a user-friendly string.
 *
 * Handles:
 * - Error instances (including ApiError with error_data)
 * - Plain strings
 * - null / undefined / symbol
 * - Objects with {error} or {message} keys
 * - JSON-encoded error bodies (e.g., '{"message":"denied"}')
 * - Circular references (graceful fallback)
 * - Primitive JSON values (numbers, booleans)
 */
export function parseErrorMessage(err: unknown): string {
  // Implementation extracted from NodeApiKeySection.tsx:34-64
  // with ApiError enhancement (check err.error_data if present)
}
```

**Behavior contract** (from existing NodeApiKeySection implementation + enhancements):

| Input | Output | Source |
|-------|--------|--------|
| `new Error('boom')` | `'boom'` | `err.message` |
| `new ApiError('denied', 403, resp, {code:'E_DENIED'})` | `'denied'` | `err.message` (ApiError extends Error) |
| `'plain failure'` | `'plain failure'` | string passthrough |
| `null` | `'Failed'` | null guard |
| `undefined` | `'Failed'` | null guard |
| `Symbol('x')` | `'Failed'` | symbol guard |
| `{code:'E_DENIED'}` | `'Failed'` | JSON.stringify → no message/error key |
| `new Error('{"message":"server denied"}')` | `'server denied'` | JSON.parse → extract .message |
| `new Error('{"error":"not found"}')` | `'not found'` | JSON.parse → extract .error |
| `new Error('"just a string"')` | `'just a string'` | JSON.parse → string primitive |
| `new Error('42')` | `'42'` | JSON.parse → number primitive → raw |
| `{self: circular}` | `'Failed'` | JSON.stringify throws → catch → fallback |

**What changes from the local version:** The local `parseErrorMessage` in NodeApiKeySection
returns `'Failed'` as the generic fallback. The shared version preserves this behavior exactly
to maintain backward compatibility. No existing user-visible message changes.

### 2. Dialog accessibility design

**Current state:** `dialog.tsx` is 116 lines of custom HTML:
- `Dialog` renders a portal-like `<div>` with a backdrop overlay
- `uncloseable` prop hides the close button and prevents overlay click
- No `role`, no `aria-modal`, no focus trap, no Escape handling
- `DialogContent` is just a `<div>` with flex layout

**Target state:** Rewrite `dialog.tsx` to use `@radix-ui/react-dialog` (already at `^1.1.18`).
Radix provides all the a11y primitives for free:

```tsx
import * as DialogPrimitive from "@radix-ui/react-dialog";

const Dialog = DialogPrimitive.Root;
const DialogTrigger = DialogPrimitive.Trigger;
const DialogPortal = DialogPrimitive.Portal;
const DialogClose = DialogPrimitive.Close;

const DialogOverlay = React.forwardRef(...) => (
  <DialogPrimitive.Overlay className={cn("fixed inset-0 z-50 bg-black/80 ...", className)} />
);

const DialogContent = React.forwardRef(({ className, children, uncloseable, ...props }, ref) => (
  <DialogPortal>
    <DialogOverlay />
    <DialogPrimitive.Content
      ref={ref}
      onEscapeKeyDown={uncloseable ? (e) => e.preventDefault() : undefined}
      onPointerDownOutside={uncloseable ? (e) => e.preventDefault() : undefined}
      className={cn("fixed left-[50%] top-[50%] z-50 ...", className)}
      {...props}
    >
      {children}
      {!uncloseable && (
        <DialogPrimitive.Close className="absolute right-4 top-4 ...">
          <X className="h-4 w-4" />
          <span className="sr-only">Close</span>
        </DialogPrimitive.Close>
      )}
    </DialogPrimitive.Content>
  </DialogPortal>
));
```

**What Radix gives us for free:**
- `role="dialog"` on the content element
- `aria-modal="true"` on the content element
- Focus trap (Tab cycles within dialog, Shift+Tab wraps)
- Escape-to-close (blocked when `uncloseable` is true)
- `onPointerDownOutside` prevention (blocked when `uncloseable` is true)
- Proper portal rendering
- Animation support via `data-state` attributes

**API preservation:**
- All existing exports preserved: `Dialog`, `DialogContent`, `DialogHeader`, `DialogTitle`,
  `DialogDescription`, `DialogFooter`
- `Dialog` keeps `open`, `onOpenChange`, `uncloseable` props
- `DialogContent` keeps `className` and children
- `DialogHeader`, `DialogTitle`, `DialogDescription`, `DialogFooter` unchanged (plain divs)
- NodeApiKeySection's `uncloseable` usage works as-is

**New exports added:**
- `DialogTrigger` (if needed by future callers)
- `DialogClose` (explicit close button composition)
- `DialogPortal` (for advanced portal control)

### 3. Mutation guard test design

Four new test cases in `NodeApiKeySection.test.tsx`:

**Test 1: create-after-org-change (createAttemptRef guard)**
```
1. Render with orgId="org-1"
2. Open create dialog, type name, click Create (mutation starts, createAttemptRef=0)
3. Rerender with orgId="org-2" (org change effect runs, createAttemptRef becomes 1)
4. Resolve the create mutation from step 2
5. Assert: the onSuccess callback does NOT set createdSecret (attemptId 0 !== createAttemptRef 1)
6. Assert: no secret is shown in the UI
```

**Test 2: create-after-closeDialog (createAttemptRef guard)**
```
1. Render with orgId="org-1"
2. Open create dialog, type name, click Create (mutation starts, createAttemptRef=0)
3. Close the dialog (closeDialog runs, createAttemptRef becomes 1)
4. Resolve the create mutation from step 2
5. Assert: the onSuccess callback does NOT set createdSecret (attemptId 0 !== createAttemptRef 1)
6. Assert: no secret is shown in the UI
```

**Test 3: revoke-after-org-change (orgIdRef guard on revoke)**
```
1. Render with orgId="org-1", mock a key
2. Click Revoke (mutation starts with orgId="org-1")
3. Rerender with orgId="org-2" (orgIdRef.current becomes "org-2")
4. Reject the revoke mutation from step 2
5. Assert: the onError callback does NOT call setError (orgId "org-1" !== orgIdRef "org-2")
6. Assert: no error alert is shown
```

**Test 4: create-onError-after-org-change (orgIdRef guard on create)**
```
1. Render with orgId="org-1"
2. Open create dialog, type name, click Create (mutation starts with orgId="org-1")
3. Rerender with orgId="org-2" (orgIdRef.current becomes "org-2")
4. Reject the create mutation from step 2
5. Assert: the onError callback does NOT call setError (orgId "org-1" !== orgIdRef "org-2")
6. Assert: no error alert is shown in the dialog
```

---

## Decisions

### D1: Replace custom dialog.tsx with Radix (IRREVERSIBLE)

**Decision:** Replace the 116-line custom `dialog.tsx` implementation with `@radix-ui/react-dialog`
primitives. The custom implementation is deleted entirely — this is a full replacement, not a
wrapper.

**Rationale:** The custom implementation lacks fundamental a11y (role, aria-modal, focus trap,
Escape). Radix provides all of these for free and is already a dependency. The custom
implementation has 9 callers; all are updated to the same API surface.

**Irreversibility:** This deletes the custom implementation. However, the API surface is preserved,
so reverting would mean restoring the custom file and removing Radix imports — straightforward but
requires touching all 9 caller files again.

**ADR:** `dev-docs/adr/0012-replace-custom-dialog-with-radix.md`

### D2: Shared parseErrorMessage uses 'Failed' as generic fallback

**Decision:** The shared `parseErrorMessage` returns the string `'Failed'` as the generic
fallback (matching the existing NodeApiKeySection behavior), NOT a more descriptive message
like `'An unknown error occurred'`.

**Rationale:** The 6 other dialogs currently return `'An error occurred'` as their fallback.
Changing to `'Failed'` is a minor message change but keeps the shared utility consistent with
the most battle-tested implementation (NodeApiKeySection went through 22 tournament rounds).
The i18n key `settings.swarm.apiKeys.error` wraps the message as `'Failed: {{message}}'`, so
the final user-visible text remains `'Failed: Failed'` for the API key component and
`'Failed: An error occurred'` → `'Failed: Failed'` for others — a minor improvement.

**Not irreversible:** Can be changed at any time by updating the fallback constant.

### D3: uncloseable via Radix event prevention (not composition)

**Decision:** Implement `uncloseable` as a prop on `DialogContent` that calls
`e.preventDefault()` on `onEscapeKeyDown` and `onPointerDownOutside` Radix events, rather
than using a composition pattern with separate components.

**Rationale:** Radix's `DialogContent` accepts `onEscapeKeyDown` and `onPointerDownOutside`
callbacks that can prevent default close behavior via `e.preventDefault()`. This is the
idiomatic Radix pattern for controlling close behavior. The prop approach keeps the API simple:
`<Dialog uncloseable={true}>` vs requiring callers to compose different components.

**Not irreversible:** Can switch to composition pattern later without breaking callers (just
add new exports).

### D4: Update AGENTS.md with remote-frontend gates

**Decision:** Add `remote-frontend` lint, typecheck, and test gates to AGENTS.md's mandatory
gate list. The updated gate block becomes:

```bash
cargo clippy --all --all-targets --all-features -- -D warnings
cargo test --workspace
cd frontend && npm run lint
cd frontend && npx tsc --noEmit
cd remote-frontend && npm run lint
cd remote-frontend && npx tsc --noEmit
cd remote-frontend && npx vitest run
```

**Rationale:** AGENTS.md currently only gates `frontend/` (the node UI). The hive frontend
(`remote-frontend/`) has no mandatory gates. This is a process gap — bugs in the hive frontend
pass through undetected because no lint/typecheck/test gate catches them. Every workstream that
touches `remote-frontend/` must run these gates before declaring the PR ready.

**Not irreversible:** Can be reverted by removing the lines from AGENTS.md.

**Scope:** This is a process change, not a code change. The AGENTS.md update is committed as
part of this workstream's PR.

---

## Test strategy

### The gap: AGENTS.md gates don't cover remote-frontend

AGENTS.md and CLAUDE.md specify four mandatory gates before any PR is complete:

```bash
cargo clippy --all --all-targets --all-features -- -D warnings
cargo test --workspace
cd frontend && npm run lint
cd frontend && npx tsc --noEmit
```

These gates cover `frontend/` (the node UI) and the Rust workspace. They do **not** cover
`remote-frontend/` (the hive UI). This is why bugs have crept through — the hive frontend has
no mandatory lint, typecheck, or test gate in the development process.

This workstream fixes the gap by:
1. Running the full remote-frontend test suite as a mandatory gate before PR
2. Adding a decision (D4) to update AGENTS.md with remote-frontend gates
3. Making the execute phase run all gates explicitly before declaring "ready for merge"

### Unit tests for `parseErrorMessage` (`src/lib/errors.test.ts`)

- 100% line coverage on the utility
- Test every input type from the behavior contract table above
- Test `ApiError` instances with `error_data`
- Test circular references
- Test JSON-encoded primitive values (number, boolean, string)
- Test nested JSON bodies with both `message` and `error` keys

### Dialog a11y tests (`src/components/ui/dialog.test.tsx`)

- Render open dialog → assert `role="dialog"` present
- Render open dialog → assert `aria-modal="true"` present
- Open dialog → press Escape → assert dialog closes
- Open dialog with `uncloseable` → press Escape → assert dialog stays open
- Open dialog with `uncloseable` → click overlay → assert dialog stays open
- Open dialog without `uncloseable` → click overlay → assert dialog closes
- Tab key cycles within dialog (focus trap)
- Close button is hidden when `uncloseable` is true
- Close button is visible when `uncloseable` is false

### Mutation guard tests (`NodeApiKeySection.test.tsx`)

- 4 new test cases as described in the Design section above
- All 28 existing tests continue to pass

### Mandatory local testing gates (run BEFORE declaring PR ready)

These gates run in sequence. Every gate must pass. No gate is skipped.

```bash
# Gate 1: Rust workspace — clippy + tests
cargo clippy --all --all-targets --all-features -- -D warnings
cargo test --workspace

# Gate 2: Node frontend — lint + typecheck
cd frontend && npm run lint
cd frontend && npx tsc --noEmit

# Gate 3: Hive frontend — lint + typecheck + unit tests
cd remote-frontend && npm run lint
cd remote-frontend && npx tsc --noEmit
cd remote-frontend && npx vitest run

# Gate 4: Cross-check — no uncommitted changes
git status  # must show "nothing to commit, working tree clean"
```

**Gate 3 is the new addition.** It was missing from AGENTS.md, which is why hive frontend
regressions were not caught. This spec mandates it for this workstream; Decision D4 mandates
it for all future workstreams.

### What's deferred to a post-deploy workstream

Tests requiring a deployed host are NOT run locally and are NOT part of this workstream's
definition of done. They belong in a separate follow-up workstream that runs after this PR
is merged and deployed:

- Browser-based a11y audit (`@axe-core/playwright`, WCAG AA)
- E2E dialog behavior on real browser (Playwright against deployed instance)
- Manual screen reader testing (NVDA/VoiceOver)
- Lighthouse audit (performance, a11y scores)

The follow-up workstream is created only after this PR merges. Its scope is validation-only —
it does not modify code, only reports findings.
