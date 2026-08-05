---
workstream: frontend-prettier-debt
status: active
created: 2026-08-05
parent_session: PR #467 /dr:pr-feedback
---

# frontend-prettier-debt

`cd frontend && npm run format:check` fails on **35 files**. Filed as `F-2026-08-05-02`.

## How it went unnoticed

`frontend/package.json` defines these as *separate* scripts:

```json
"check":        "tsc --noEmit",
"lint":         "eslint . --max-warnings 0",
"format:check": "prettier --check \"src/**/*.{ts,tsx,js,jsx,json,css,md}\""
```

`lint` is **ESLint only** — it does not run Prettier. Every gate sweep during the
`vk-swarm-node-ui-localize` run reported `lint 0, tsc 0, vitest green` and never invoked
`format:check`, even though CLAUDE.md lists it as a required gate. So "all gates green" was
true of the gates that were run, and silent about this one.

Caught by CodeRabbit on PR #467, which correctly contradicted an assistant reply claiming the
individual gates covered what `npm run check` wraps. They do not.

## PRE-EXISTING — not introduced by `vk-swarm-node-ui-localize`

- 34 of the 35 files were never touched by that workstream.
- The single overlap, `frontend/src/pages/Nodes.tsx`, **already failed Prettier at `feff74be`**
  (the pre-workstream merge-base) — verified by checking out that blob and running
  `prettier --check` against it.

## Why a separate workstream

Fixing it means `prettier --write` across 35 mostly-unrelated files — a large mechanical diff
landing directly on `main`, likely to conflict with in-flight branches. That is a scope widening,
not part of any current change. Tracked here per CLAUDE.md "No Deferred Remediation" (fix now,
split as a named tracked follow-up created in THIS session, or escalate) rather than carried
silently.

**Not suppressed.** No config change, no ignore file, no disabled gate.

## What "done" looks like

- `cd frontend && npm run format:check` exits 0.
- Land it as one isolated formatting-only commit, when no long-lived branch is in flight.
- Add `format:check` to the routine gate sweep — and preferably to `frontend/package.json`'s
  `check` script or CI — so the gap cannot silently reopen. That wiring is the durable fix; the
  reformat alone is not.
