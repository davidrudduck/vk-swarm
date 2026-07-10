# After hive-node-api-key-ui — What's Next

**Date:** 2026-07-09
**Context:** `hive-node-api-key-ui` workstream shipped (PR #461, 36 tests, 100% line coverage, 22 tournament rounds + 5 final review rounds + 5 code-review rounds). Component is at `remote-frontend/src/components/swarm/NodeApiKeySection.tsx` (552 lines).

---

## What Would Make the API Key Implementation World-Class

The component is solid — triple-guarded mutation callbacks, per-key ref-counted pending state, uncloseable dialog during secret reveal, comprehensive `parseErrorMessage`. But "world-class" requires going further:

### 1. Shared `parseErrorMessage` utility
Currently private to `NodeApiKeySection.tsx`. Every other swarm dialog uses `err instanceof Error ? err.message : 'An error occurred'` — showing raw JSON to users. Extract to `src/lib/errors.ts` and update all 6+ call sites (`SwarmLabelDialog`, `MergeProjectsDialog`, `MergeTemplatesDialog`, `SwarmProjectDialog`, `MergeLabelsDialog`, `SwarmTemplateDialog`). The API key component's implementation handles: Error, string, null, symbol, object, JSON body, `{error}` format, `{message}` format, circular refs, primitive JSON values.

### 2. Dialog accessibility (shared `dialog.tsx`)
The custom `Dialog` component at `components/ui/dialog.tsx` lacks `role="dialog"`, `aria-modal="true"`, focus trapping, and Escape handling. This was flagged by every reviewer (tournament R1-R22, final review R1-R5, code review R1). The API key component's `uncloseable` secret-reveal flow makes this user-facing. Fix: replace with `@radix-ui/react-dialog` (already a dependency) or add the missing attributes to the shared component.

### 3. Integration tests for mutation guards
The `createAttemptRef` stale-secret guard (the most subtle defense in the component) has zero direct test coverage. The `orgIdRef` guard on `createMutation.onError` is also untested. Add: create-after-org-change, create-after-closeDialog, revoke-after-org-change tests.

### 4. i18n namespace rename
Keys are `settings.swarm.apiKeys.*` but the component lives on the Nodes page, not Settings. Rename to `nodes.apiKeys.*` across 4 locales (en, es, ja, ko) + component + test.

### 5. Real-time key status
The list uses `staleTime: 30_000` — a revoked key takes 30s to appear revoked on another tab. Consider WebSocket push or shorter staleTime for the keys query.

---

## Overarching Masterplan — Three Top Priorities

### Priority 1: Fix sign-in on non-loopback HTTP origins (HIGH, blocks development)

**Finding:** `F-2026-07-06-02` — `crypto.subtle` is undefined on non-HTTPS origins (except localhost). This means anyone developing on `http://192.168.x.x:3000` or `http://mydev.local:3000` cannot sign in via PKCE.

**Impact:** Blocks remote development, mobile testing, and team demos on non-localhost origins.

**Approach:**
- Add a fallback: if `crypto.subtle` is unavailable, use a pure-JS PKCE implementation (e.g., `pkce-challenge` package or inline SHA-256 via `crypto-js`)
- Or: detect non-secure context and show a clear error with instructions ("Use localhost or enable HTTPS")
- Or: add a dev-mode bypass that uses `plain` code challenge method (not recommended for production)

**Spec anchor:** `remote-frontend/src/pkce.ts:10`

### Priority 2: Ship vk-swarm-hive-ui-polish (READY, PR #457)

**State:** PR #457 open, all gates green, code-review converged, `/wai:close` ran. Needs `/wai:ship`.

**What it delivers:** Error resilience (ErrorBoundary, offline-first PWA, service worker), E2E test suite.

**After shipping, pick one world-class improvement:**
- Real Electric sync integration testing (live Docker Compose stack)
- Accessibility audit (`@axe-core/playwright`, WCAG AA)
- Lighthouse 100 (route-based code splitting, web-vitals, Lighthouse CI)

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
| `dev-docs/2026-07-03-next-session.md` | **COMPLETE** — hive-redesign shipped (PR #451 merged) | Archive |
| `dev-docs/2026-07-06-next-session-after-remote-docker-build-fix.md` | **PARTIAL** — PR #458 needs `/wai:ship`, round-4 findings unreviewed | Keep (ship first) |

### In-flight (needs action)
| Doc | Status | Action |
|-----|--------|--------|
| `dev-docs/2026-07-06-next-session-after-vk-swarm-hive-ui-polish.md` | **IN-FLIGHT** — PR #457 needs `/wai:ship` | Keep (ship first) |

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
You are picking up after the hive-node-api-key-ui workstream shipped.

CURRENT STATE:
- PR #461 is merged. Component is at remote-frontend/src/components/swarm/NodeApiKeySection.tsx.
- PR #457 (vk-swarm-hive-ui-polish) needs /wai:ship to merge.
- PR #458 (remote-docker-build-fix) needs /wai:ship to merge.
- F-2026-07-06-02 (sign-in broken on non-loopback HTTP) is HIGH severity and open.

MANDATORY GATES (run before any commit):
cd remote-frontend && npx tsc --noEmit && npm run lint && npx vitest run
cargo clippy --all --all-targets --all-features -- -D warnings
cargo test --workspace

TASK 1 — SHIP PENDING WORK:
1. Run /wai:ship vk-swarm-hive-ui-polish to merge PR #457
2. Run /wai:ship remote-docker-build-fix to merge PR #458

TASK 2 — FIX SIGN-IN (highest priority finding):
F-2026-07-06-02: crypto.subtle is undefined on non-HTTPS origins.
Fix in remote-frontend/src/pkce.ts — add fallback for non-secure contexts.
This blocks remote development and mobile testing.

TASK 3 — PICK ONE WORLD-CLASS IMPROVEMENT (after shipping):
Choose based on time available:

A) SHARED parseErrorMessage UTILITY (2-3 hours):
   Extract parseErrorMessage from NodeApiKeySection.tsx to src/lib/errors.ts.
   Update 6+ dialog call sites. Highest ROI for code quality.

B) DIALOG ACCESSIBILITY (3-4 hours):
   Replace custom dialog.tsx with @radix-ui/react-dialog.
   Add role="dialog", aria-modal, focus trap, Escape handling.
   Affects every dialog in the app. Highest ROI for a11y.

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
| `remote-frontend/src/components/swarm/NodeApiKeySection.tsx` | Shipped component (552 lines) |
| `remote-frontend/src/components/swarm/NodeApiKeySection.test.tsx` | 36 test cases |
| `remote-frontend/src/pkce.ts:10` | crypto.subtle usage (F-2026-07-06-02) |
| `remote-frontend/src/components/ui/dialog.tsx` | Shared dialog (needs a11y fix) |
| `docs/plans/hive-node-api-key-ui/decisions-ledger.md` | 14 post-review known issues |
| `dev-docs/BACKLOG.md` | 2 open findings (F-2026-07-04-04, F-2026-07-06-02) |
| `dev-docs/MASTER.md` | Workstream tracker (13 rows) |
