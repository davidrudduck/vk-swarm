---
id: "301"
phase: 3
title: Record full gates and manual LAN OAuth verification
status: passed
depends_on: ["202"]
parallel: false
conflicts_with: []
files:
  - docs/plans/fix-nonloopback-signin/decisions-ledger.md
irreversible: false
scope_test: "N/A"
allowed_change: edit
covers_criteria: [SC10, SC11]
---

## Failing test (write first)

N/A — this is the acceptance evidence task. It is verified by the manual commands and observations below, which must be recorded in `docs/plans/fix-nonloopback-signin/decisions-ledger.md` under `## Acceptance evidence`.

## Change

### File: `docs/plans/fix-nonloopback-signin/decisions-ledger.md`

- **Anchor:** end of file.
- **Before:** the current file ends after the precheck false-positive note.
- **After:** append this section, filling in exact command output summaries and manual observations:

  ```md
  ## Acceptance evidence

  ### Task 301 — full gates and LAN OAuth verification

  Automated gates:

  - `cd remote-frontend && npm run test:run -- src/pkce.test.ts src/AppRouter.test.tsx src/pages/InvitationPage.test.tsx src/pages/InvitationCompletePage.test.tsx` — PASS/FAIL, include failing test count if not PASS.
  - `cd remote-frontend && npm run test:run` — PASS/FAIL, include suite summary.
  - `cd remote-frontend && npm run lint` — PASS/FAIL.
  - `cd remote-frontend && npx tsc --noEmit` — PASS/FAIL.
  - `cargo clippy --all --all-targets --all-features -- -D warnings` — PASS/FAIL.
  - `cargo test --workspace` — PASS/FAIL.
  - `cd frontend && npm run lint` — PASS/FAIL.
  - `cd frontend && npx tsc --noEmit` — PASS/FAIL.

  Manual LAN verification:

  - Normal login over `http://<lan-ip>:3002/login`: provider button clicked, provider authorization URL reached, no local `crypto.subtle` error shown — PASS/FAIL with origin used.
  - Invitation OAuth over `http://<lan-ip>:3002/invitations/<token>/accept`: provider button clicked, provider authorization URL reached, no local `crypto.subtle` error shown — PASS/FAIL with origin used.

  Result: PASS only if every line above is PASS. If any line is FAIL, unavailable, or inconclusive, stop and escalate; do not mark the workstream complete.
  ```

## Allowed moves

- Run every automated command listed in the appended section.
- Perform both manual LAN checks from a non-loopback plain-HTTP origin.
- Edit only `docs/plans/fix-nonloopback-signin/decisions-ledger.md` to record exact evidence. Do not edit `docs/plans/fix-nonloopback-signin/verify-301-evidence.sh` during execution; it is a prewritten plan verifier used by the Done-when command.
- Do not mark a command PASS unless the fresh command exited 0 in this task.
- Do not mark LAN verification PASS unless the browser reaches the provider authorization URL without a local digest error.

## STOP triggers

- Any automated gate fails.
- A manual LAN environment is unavailable.
- A provider redirect cannot be observed.
- A check is inconclusive.
- Any fix would require editing code in this task; stop, create/execute a code task or escalate rather than hiding it in acceptance evidence.

## Manual verification (record in decisions-ledger)

Run exactly these commands from the repo root unless the command itself changes directories:

```bash
cd remote-frontend && npm run test:run -- src/pkce.test.ts src/AppRouter.test.tsx src/pages/InvitationPage.test.tsx src/pages/InvitationCompletePage.test.tsx
cd remote-frontend && npm run test:run
cd remote-frontend && npm run lint
cd remote-frontend && npx tsc --noEmit
cargo clippy --all --all-targets --all-features -- -D warnings
cargo test --workspace
cd frontend && npm run lint
cd frontend && npx tsc --noEmit
```

Then verify both browser flows over plain HTTP non-loopback LAN access:

- Open `http://<lan-ip>:3002/login`, click GitHub or Google, and confirm the browser reaches the provider authorization URL without a local `crypto.subtle` error.
- Open `http://<lan-ip>:3002/invitations/<token>/accept`, click GitHub or Google, and confirm the browser reaches the provider authorization URL without a local `crypto.subtle` error.

Record exact PASS/FAIL evidence in the ledger section described above. Unavailable is not PASS.

## Done when

`WAI_TYPECHECK_CMD="(cd remote-frontend && npm run test:run -- src/pkce.test.ts src/AppRouter.test.tsx src/pages/InvitationPage.test.tsx src/pages/InvitationCompletePage.test.tsx) && (cd remote-frontend && npm run test:run) && (cd remote-frontend && npm run lint) && (cd remote-frontend && npx tsc --noEmit) && cargo clippy --all --all-targets --all-features -- -D warnings && cargo test --workspace && (cd frontend && npm run lint) && (cd frontend && npx tsc --noEmit) && bash docs/plans/fix-nonloopback-signin/verify-301-evidence.sh" WAI_TEST_CMD="true" bash ~/.claude/wai/scripts/task-gate.sh fix-nonloopback-signin 301` exits 0 after the ledger records every required automated and manual check as PASS.
