# Plan: fix-nonloopback-signin

Spec: `docs/superpowers/specs/2026-07-08-fix-nonloopback-signin.md` (frozen, sha `425c83cc429cb2497db64bfc1233cb57d4d0907f`).
Workstream: `dev-docs/workstreams/fix-nonloopback-signin/README.md`.

## Expected outcome

This decomposition implements the full user-visible outcome from the frozen spec: Hive UI sign-in and invitation OAuth must work when the remote frontend is opened from a plain-HTTP non-loopback LAN origin, such as `http://192.168.x.x:3002`. The accepted result is not a helper-only patch and not a partial unit-test fix. The accepted result includes a browser-safe SHA-256 fallback, native-crypto preservation, route-level tests for both OAuth entry points, storage/callback preservation tests, static checks, repository gates, and manual LAN verification.

No item is deferred. If a command, route test, storage check, repository gate, or LAN verification cannot run, the executor stops and escalates in-session instead of marking the workstream complete.

## Approach

The work is sequential. Phase 1 fixes the broken primitive at `remote-frontend/src/pkce.ts` and adds direct unit tests proving both digest paths. Phase 2 then proves that the fixed primitive reaches the actual user flows by adding route-level tests for `/login`, `/oauth/callback`, `/invitations/:token/accept`, and invitation completion storage. Phase 3 is the explicit hardline acceptance task: it runs the supplemental remote-frontend checks, the AGENTS.md mandatory repository gate, and the two manual LAN OAuth checks, then records the evidence in the decisions ledger.

The decomposition deliberately keeps implementation inside `remote-frontend/src/pkce.ts`. It does not change OAuth provider selection, callback URLs, session-storage keys, backend payload shape, route structure, or the node `frontend/`. All tests restore globals and storage so they do not contaminate neighbouring frontend tests.

## Phases

### Phase 1 — PKCE digest primitive (SC1, SC8)

Replace the unconditional `crypto.subtle.digest()` call with a capability-detected native branch plus an in-repo browser-safe SHA-256 fallback. Add direct tests with known SHA-256 vectors and native-branch assertions.

| ID | Title | SCs | dep: | conflicts: |
|---:|---|---|---|---|
| 101 | Implement browser-safe SHA-256 fallback for PKCE | SC1 SC8 | dep: - | conflicts: - |

### Phase 2 — OAuth entrypoint regressions (SC2, SC3, SC4, SC5, SC6, SC7, SC9)

Prove the fixed primitive reaches both user-facing OAuth entry points and does not disturb verifier/invitation storage. These are route-level regressions, not helper-only assertions.

| ID | Title | SCs | dep: | conflicts: |
|---:|---|---|---|---|
| 201 | Cover non-loopback normal login and callback storage | SC2 SC3 SC5 SC6 SC9 | dep: 101 | conflicts: - |
| 202 | Cover non-loopback invitation OAuth and completion storage | SC4 SC5 SC7 SC9 | dep: 201 | conflicts: - |

### Phase 3 — Hardline acceptance verification (SC10, SC11)

Run every required automated gate and both manual LAN OAuth checks. Record exact evidence in the decisions ledger. If any check is red, unavailable, or inconclusive, the task stops and escalates.

| ID | Title | SCs | dep: | conflicts: |
|---:|---|---|---|---|
| 301 | Record full gates and manual LAN OAuth verification | SC10 SC11 | dep: 202 | conflicts: - |

## Gate

Every task uses `task-gate.sh` for its scoped verification. Task 301 additionally runs and records the complete gate set:

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

Manual LAN verification is also required by task 301 for both `/login` and `/invitations/:token/accept` on `http://<lan-ip>:3002`. No task or phase can mark this workstream complete without it.
