---
doc_type: spec
status: draft
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
> swarm dialogs.

## Intent (what / why)

The `remote-frontend` has three quality gaps that affect every dialog and mutation in the app:

1. **No shared error parsing.** Each dialog handles errors differently — some show raw JSON
   bodies to users, some show `err.message` (which may be `"Internal Server Error"`), some show
   generic strings. `NodeApiKeySection.tsx` has a local `parseErrorMessage` that handles 3 cases
   (Error, string, unknown) but it's not shared. The other 6+ dialog call sites use bare
   `instanceof Error` checks.

2. **Uncloseable dialog pattern lacks a11y documentation.** The `dialog.tsx` component wraps
   `@radix-ui/react-dialog` which provides role, aria-modal, focus trap, and Escape handling.
   But `NodeApiKeySection.tsx` implements a custom uncloseable flow for secret reveal that
   overrides Escape behavior — this pattern needs to be a documented, accessible variant rather
   than per-component ad-hoc overrides.

3. **Mutation guard tests are missing.** The `createAttemptRef` stale-secret guard (prevents
   creating a key after the dialog has been closed and reopened) and the `orgIdRef` guard on
   `createMutation.onError` (prevents clearing form state when org has changed) have zero direct
   test coverage. These are the most subtle defenses in the component.

## Users / who is affected

- **End-users (operators/admins):** see raw error JSON or unhelpful error messages when dialogs
  fail. Screen reader users get no signal that a dialog is intentionally uncloseable.
- **Developers:** duplicate error handling code across 6+ dialogs. No documented pattern for
  uncloseable dialogs. Mutation guard behavior is undocumented and untested.

## Success criteria

1. `parseErrorMessage` lives at `src/lib/errors.ts` and handles: Error, string, null, symbol,
   object with `error` key, object with `message` key, JSON body, circular refs, primitive JSON
   values. Returns a user-friendly string in all cases.
2. All 6+ dialog error call sites use the shared `parseErrorMessage` instead of inline checks.
3. The uncloseable dialog pattern is a documented variant of `DialogContent` (prop or composition)
   that preserves Radix a11y (focus trap, aria-modal) while suppressing close-on-escape and
   close-on-overlay-click.
4. `createAttemptRef` guard has at least 3 test cases: create-after-org-change,
   create-after-closeDialog, revoke-after-org-change.
5. `orgIdRef` guard on `createMutation.onError` has at least 1 test case.
6. All existing tests continue to pass. No regressions in dialog behavior.
7. `npm run lint`, `npx tsc --noEmit`, `npx vitest run` all pass.

## Constraints

- **No new dependencies.** `@radix-ui/react-dialog` is already installed. Error parsing is pure TS.
- **Backward compatible.** The shared `parseErrorMessage` must not change the user-visible message
  for any existing error path — only improve messages for paths that currently show raw JSON.
- **i18n scope.** The `settings.swarm.apiKeys.*` → `nodes.apiKeys.*` namespace rename is NOT in
  scope for this workstream. It's a separate concern (correctness, not quality).
- **Dialog component structure.** `dialog.tsx` is the standard shadcn/ui wrapper. We extend it,
  not replace it.

## Out of scope

- **i18n namespace rename** (`settings.swarm.apiKeys.*` → `nodes.apiKeys.*`) — separate workstream
- **Real-time key status** (WebSocket push or shorter staleTime) — separate workstream
- **Design system** (`vk-swarm-design-system`) — separate workstream, this workstream's shared
  utilities will be used by the design system when it ships
- **Full dialog replacement** — we don't rewrite `dialog.tsx`, we extend it with an uncloseable variant
- **Backend error format changes** — we parse what the backend sends today
