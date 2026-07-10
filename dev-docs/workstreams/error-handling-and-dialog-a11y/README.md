---
workstream: error-handling-and-dialog-a11y
doc_type: readme
status: active
title: "Shared error parsing, dialog accessibility, mutation guard tests"
staging_pointers:
  - docs/superpowers/specs/2026-07-10-error-handling-and-dialog-a11y.md
depends_on: []
adrs: []
---

# error-handling-and-dialog-a11y

**Origin:** Post-ship improvements identified during `hive-node-api-key-ui` (PR #461) and
`fix-nonloopback-signin` (PR #463) workstreams. Every tournament and code-review round flagged
these gaps.

## What this workstream owns

Three quality improvements to `remote-frontend/`:

1. **Shared `parseErrorMessage` utility** — extract to `src/lib/errors.ts`, update all 6+ dialog
   error call sites. Handles: Error, string, null, symbol, object with `error`/`message` keys,
   JSON body, circular refs, primitive JSON values.

2. **Uncloseable dialog variant** — extend `dialog.tsx` with a documented `uncloseable` prop or
   composition pattern that preserves Radix a11y (focus trap, aria-modal) while suppressing
   close-on-escape and close-on-overlay-click. Used by the API key secret-reveal flow.

3. **Mutation guard test coverage** — add tests for `createAttemptRef` (stale-secret guard) and
   `orgIdRef` (org-change guard) in `NodeApiKeySection.tsx`. At least 4 test cases covering
   create-after-org-change, create-after-closeDialog, revoke-after-org-change, and
   onError-after-org-change.

## Relationship to the program

Independent of all other workstreams. Uses existing shadcn/ui + Radix primitives. The shared
utilities will be consumed by `vk-swarm-design-system` when it ships. No backend changes.
No new dependencies.
