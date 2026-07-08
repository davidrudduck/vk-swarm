---
doc_type: decisions-ledger
workstream: fix-nonloopback-signin
---

# Decisions ledger — fix-nonloopback-signin

## Precheck findings

### 2026-07-08 — anchor-check false positive for nested package paths

The first `/wai:precheck fix-nonloopback-signin` run that reached anchor grounding failed on
paths extracted as root `src/...` anchors:

- `src/AppRouter.tsx`
- `src/pages/InvitationPage.tsx`
- `src/pkce.test.ts`
- `src/pkce.ts`

This is a false positive. The spec intentionally references files under the
`remote-frontend` package, such as `remote-frontend/src/pkce.ts`, but the anchor extractor scans
for `(src|extensions|ui|packages|apps)/...` substrings and strips the package prefix from nested
paths.

Evidence against `main`:

```bash
git cat-file -e main:remote-frontend/src/pkce.ts
git cat-file -e main:remote-frontend/src/AppRouter.tsx
git cat-file -e main:remote-frontend/src/pages/InvitationPage.tsx
```

All three real repo-root anchors exist on `main` (exit 0). The extracted root paths do not exist:

```text
fatal: path 'src/pkce.ts' does not exist in 'main'
fatal: path 'src/AppRouter.tsx' does not exist in 'main'
fatal: path 'src/pages/InvitationPage.tsx' does not exist in 'main'
```

Resolution: keep the spec's precise `remote-frontend/src/...` anchors and rerun precheck with the
script's explicit false-positive escape hatch: `--no-anchor-check`.

## Decompose findings

### 2026-07-08 — plan-lint sibling advisories acknowledged

`wai-plan-lint.sh fix-nonloopback-signin` passed with three advisory `W:` sibling warnings:

- Task 101 creates `remote-frontend/src/pkce.test.ts` beside unlisted sibling
  `remote-frontend/src/toolchain.test.ts`. This is not a pattern sibling: `toolchain.test.ts`
  verifies toolchain wiring, while `pkce.test.ts` is a colocated unit test for the PKCE helper.
  The task already lists and requires reading `remote-frontend/src/setupTests.ts` and
  `remote-frontend/src/api.ts` as the relevant same-directory context.
- Task 202 creates `remote-frontend/src/pages/InvitationPage.test.tsx` beside unlisted sibling
  `remote-frontend/src/pages/InvitationCompletePage.tsx`. The task's purpose is to create tests
  for the invitation flow and it already lists the production completion page test target separately
  as `remote-frontend/src/pages/InvitationCompletePage.test.tsx` plus the page-test pattern sibling
  `remote-frontend/src/pages/Nodes.test.tsx`.
- Task 202 creates `remote-frontend/src/pages/InvitationCompletePage.test.tsx` beside unlisted
  sibling `remote-frontend/src/pages/InvitationCompletePage.tsx`. This is the production file under
  test, but task 202 is create-only and must not edit production code. The test task is constrained
  to create the new test files and read the page/test siblings.

No advisory indicates a missing implementation task or deferred work. The plan-lint hard gate passes
with full SC coverage.

### 2026-07-08 — tournament round 1 closure

The required breakdown tournament ran with Codex, Gemini, and OpenCode/GLM-5.2 competitors.
Round record: `docs/plans/fix-nonloopback-signin/reviews/tournament-round-1.md`.

Validated remediation applied:

- Task 301 no longer has a hollow `true` gate. It now runs all automated gates through
  `WAI_TYPECHECK_CMD` and then runs `docs/plans/fix-nonloopback-signin/verify-301-evidence.sh`.
- `verify-301-evidence.sh` rejects missing acceptance evidence, missing PASS lines, placeholder
  `PASS/FAIL` text, failures, unavailable checks, and inconclusive checks.
- Tasks 201 and 202 no longer prescribe brittle jsdom `window.location.assign` spying. Their route
  tests keep `initOAuth()` pending after challenge/storage assertions, and task 301 remains the
  required browser-level proof that the provider authorization URL is reached over LAN HTTP.

Focused re-checks after remediation:

- `bash -n docs/plans/fix-nonloopback-signin/verify-301-evidence.sh` — PASS.
- `wai-plan-lint.sh fix-nonloopback-signin` — PASS with only acknowledged sibling advisories.
