# After fix-nonloopback-signin — What's Next

**Date:** 2026-07-10
**Context:** `fix-nonloopback-signin` workstream closed (PR #463 open, all gates green, 32 tournament rounds, 5 final review rounds, code-review converged). PKCE SHA-256 fallback implemented for non-secure contexts.

---

## What Would Make the Implementation World-Class

The PKCE fallback is solid — capability detection, pure-TS SHA-256, FIPS multi-block vectors, 100% line coverage on target files. But "world-class" requires going further:

### 1. PKCE Challenge Method Abstraction

Currently the PKCE logic is inline in `pkce.ts`. Extract to a `src/lib/crypto/pkce.ts` module with:
- `generateVerifier(): string`
- `generateChallenge(verifier: string): Promise<string>`
- `sha256(data: Uint8Array): Promise<Uint8Array>` (with fallback detection)
- `bytesToHex(bytes: Uint8Array): string`
- `base64UrlEncode(array: Uint8Array): string`

This makes the crypto layer testable in isolation and reusable for future OAuth flows.

### 2. OAuth State Machine

The OAuth flow is currently spread across `AppRouter.tsx`, `InvitationPage.tsx`, and `InvitationCompletePage.tsx`. Consider extracting to a `useOAuthFlow()` hook that manages:
- State machine: `idle → initiating → redirecting → callback → redeeming → complete/error`
- Storage: `pkce_verifier`, `invitation_token`, `oauth_state`
- Cleanup: `clearVerifier()`, `clearInvitationToken()`, `localStorage.removeItem('access_token')`
- Error handling: `OAuthError` class with typed error codes

This would centralize the OAuth logic and make it testable without route-level rendering.

### 3. Security Audit Documentation

Add a `docs/security/oauth-pkce.md` that documents:
- Threat model: what attacks does PKCE prevent?
- Token storage: localStorage vs sessionStorage vs httpOnly cookies
- Non-secure context risks: what's the attack surface on plain HTTP?
- Refresh token rotation: how does the backend handle token refresh?
- CSRF protection: how is the `state` parameter validated?

This is critical for compliance and onboarding new developers.

---

## Overarching Masterplan — Three Top Priorities

### Priority 1: Ship pending work (IMMEDIATE)

**PRs ready to merge:**
- PR #463: `fix-nonloopback-signin` (this workstream)
- PR #457: `vk-swarm-hive-ui-polish` (error resilience + PWA)
- PR #458: `remote-docker-build-fix` (Docker build fix)

**Action:** Run `/wai:ship` for each in order.

### Priority 2: Fix remaining high-severity findings (HIGH)

**F-2026-07-04-04:** crisp-river uncommitted Cargo.toml doctest edits on merged branch
- Status: open, owned by another session
- Action: coordinate with crisp-river session to resolve

**F-2026-07-06-02:** Sign-in broken on non-loopback HTTP origins
- Status: **RESOLVED** by this workstream (PKCE SHA-256 fallback)
- Action: update BACKLOG.md to mark as `shipped`

### Priority 3: Bring ignored tests back to life (QUALITY, ongoing)

**Two active workstreams:**
- `remote-services-doctest-revival` — 32 `rust,ignore`'d doctests in remote + services crates
- `terminal-session-pty-tests` — 5 `#[ignore]`'d PTY-spawning tests

**Impact:** These represent real test coverage gaps. The doctests are documentation-as-tests that currently don't run; the PTY tests cover terminal session spawning that's critical for the node UI.

**Approach:** Un-ignore one at a time, fix the underlying issue (DB dependency, PTY device, network), and verify it passes. Create a follow-up workstream per crate.

---

## What's Not Started, Pending, In-Flight, or Complete

### Complete (ready to archive)
| Doc | Status | Action |
|-----|--------|--------|
| `dev-docs/2026-07-03-next-session.md` | **COMPLETE** — hive-redesign shipped (PR #451 merged) | Already archived |
| `dev-docs/2026-07-06-next-session-after-remote-docker-build-fix.md` | **PARTIAL** — PR #458 needs `/wai:ship`, round-4 findings unreviewed | Keep (ship first) |
| `dev-docs/2026-07-06-next-session-after-vk-swarm-hive-ui-polish.md` | **IN-FLIGHT** — PR #457 needs `/wai:ship` | Keep (ship first) |
| `dev-docs/backlog/2026-07-09-after-hive-node-api-key-ui.md` | **COMPLETE** — hive-node-api-key-ui shipped (PR #461 merged) | Archive |

### In-flight (needs action)
| Doc | Status | Action |
|-----|--------|--------|
| `dev-docs/2026-07-06-next-session-after-remote-docker-build-fix.md` | **IN-FLIGHT** — PR #458 needs `/wai:ship` | Ship first |
| `dev-docs/2026-07-06-next-session-after-vk-swarm-hive-ui-polish.md` | **IN-FLIGHT** — PR #457 needs `/wai:ship` | Ship first |

### Draft workstreams (not started)
| Workstream | Status | Dependencies | Ready? |
|------------|--------|--------------|--------|
| `vk-swarm-design-system` | draft | None | Yes — can start anytime |
| `vk-swarm-node-ui-localize` | draft | vk-swarm-node-foundations (shipped) | Yes |
| `vk-swarm-refactor` | draft | Multiple shipped workstreams | Yes (umbrella) |

### Active workstreams (in progress)
| Workstream | Status | Next action |
|------------|--------|-------------|
| `remote-services-doctest-revival` | active | Un-ignore doctests one at a time |
| `terminal-session-pty-tests` | active | Fix PTY test dependencies |

---

## Prompt for Next Session

```
You are picking up after the fix-nonloopback-signin workstream closed.

CURRENT STATE:
- PR #463 is open (fix-nonloopback-signin). Run /wai:ship to merge.
- PR #457 (vk-swarm-hive-ui-polish) needs /wai:ship to merge.
- PR #458 (remote-docker-build-fix) needs /wai:ship to merge.
- F-2026-07-06-02 (sign-in broken on non-loopback HTTP) is RESOLVED by this workstream.
- F-2026-07-04-04 (crisp-river Cargo.toml) is open, owned by another session.

MANDATORY GATES (run before any commit):
cd remote-frontend && npx tsc --noEmit && npm run lint && npx vitest run
cargo clippy --all --all-targets --all-features -- -D warnings
cargo test --workspace

TASK 1 — SHIP PENDING WORK:
1. Run /wai:ship fix-nonloopback-signin to merge PR #463
2. Run /wai:ship vk-swarm-hive-ui-polish to merge PR #457
3. Run /wai:ship remote-docker-build-fix to merge PR #458

TASK 2 — UPDATE BACKLOG:
- Mark F-2026-07-06-02 as "shipped" in dev-docs/BACKLOG.md (resolved by this workstream)
- Archive dev-docs/backlog/2026-07-09-after-hive-node-api-key-ui.md to dev-docs/.archive/

TASK 3 — PICK ONE WORLD-CLASS IMPROVEMENT (after shipping):
Choose based on time available:

A) OAUTH STATE MACHINE (3-4 hours):
   Extract OAuth flow to useOAuthFlow() hook.
   Centralize state machine, storage, cleanup, error handling.
   Highest ROI for maintainability.

B) SECURITY AUDIT DOCUMENTATION (2-3 hours):
   Create docs/security/oauth-pkce.md.
   Document threat model, token storage, non-secure context risks.
   Highest ROI for compliance and onboarding.

C) DESIGN SYSTEM START (full session):
   Run /wai:prd-new vk-swarm-design-system to capture intent.
   The Midnight Terminal component vocabulary is the foundation
   for all future UI work.

Rules:
- Follow CLAUDE.md and AGENTS.md
- No deferred remediation — fix findings in-session
- Open PRs only against davidrudduck/vk-swarm
- If you can't finish, create a dev-docs/workstreams/<name>/README.md follow-up
```

---

## Key Files for Reference

| File | Purpose |
|------|---------|
| `remote-frontend/src/pkce.ts` | PKCE SHA-256 fallback implementation |
| `remote-frontend/src/pkce.test.ts` | PKCE unit tests (native + fallback paths) |
| `remote-frontend/src/AppRouter.tsx` | LoginPage, OAuthCallbackPage, isSafeReturnTo |
| `remote-frontend/src/pages/InvitationPage.tsx` | Invitation OAuth flow |
| `remote-frontend/src/pages/InvitationCompletePage.tsx` | Invitation completion flow |
| `remote-frontend/src/api.ts` | API functions (getInvitation, acceptInvitation, initOAuth, redeemOAuth) |
| `remote-frontend/src/lib/api/oauth.ts` | OAuth API (init, redeem, logout) |
| `remote-frontend/src/lib/api/utils.ts` | makeRequest, anySignal, ApiError |
| `remote-frontend/src/components/ProfileProvider.tsx` | Profile context provider |
| `docs/superpowers/specs/2026-07-08-fix-nonloopback-signin.md` | Frozen spec |
| `docs/plans/fix-nonloopback-signin/decisions-ledger.md` | All decisions + acceptance evidence |
| `dev-docs/BACKLOG.md` | 2 open findings (F-2026-07-04-04, F-2026-07-06-02) |
| `dev-docs/MASTER.md` | Workstream tracker (14 rows) |
