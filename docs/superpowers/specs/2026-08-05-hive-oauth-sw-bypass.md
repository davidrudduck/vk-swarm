---
doc_type: spec
status: active
workstream: hive-oauth-sw-bypass
change_kind: bugfix
verify_cmd: "curl -fsS http://127.0.0.1:9000/sw.js | grep -q 'v1/oauth'"
---

# hive-oauth-sw-bypass — hive SW must not intercept the OAuth redirect chain

## Intent
Sign-in is broken for real users on both surfaces. (1) A user cannot sign in on a node: the GitHub OAuth popup navigates to the hive origin and 'just spins' forever; the user ends up with no permissions and cannot work (F-2026-08-03-01). (2) Sign-in on the hive itself only works after manually unregistering the hive's service worker / PWA (F-2026-08-03-02).

Root cause is CONFIRMED by controlled experiment (hive SW registered → node sign-in spins; unregistered → works; extensions active in both arms; incognito — no SW — works): the hive's Workbox NetworkFirst rule at `remote-frontend/vite.config.ts:19-20` matches `url.pathname.startsWith('/v1/') && !url.pathname.startsWith('/v1/shape')`, and BOTH OAuth legs are hive-origin /v1/ GET navigations: `authorize_url` = `{public_origin}/v1/oauth/{provider}/start?handoff_id={id}` (`crates/remote/src/auth/handoff.rs:157-162`) and the provider `redirect_uri` = `{public_origin}/v1/oauth/{provider}/callback` (`crates/remote/src/auth/handoff.rs:198-202`). So the SW intercepts the entire OAuth redirect chain — even for node users, whose popup navigates to the hive origin where the SW is registered. Full mechanism, ruled-out hypotheses (crypto.subtle PKCE, return_to allowlist, returnTo host derivation), and the two candidate failure modes (SW-served redirected:true navigation rejected by the browser vs NetworkFirst stale-cache fallback) are in `dev-docs/2026-08-03-node-signin-blocked-findings.md` — read it before re-investigating anything.

A second, independent defect makes every auth regression silent: `OAuthDialog` (`frontend/src/components/dialogs/global/OAuthDialog.tsx:95-113`) polls `/api/auth/status` forever with no timeout and no error branch, so a dead flow presents as an endless spinner (F-2026-08-04-01). Fixed here so the next auth failure is diagnosable.


## User stories
- **US1:** As a node user in a normal browser profile (hive SW registered), I can complete GitHub sign-in from the node and receive my permissions, without ever touching DevTools.
- **US2:** As a hive user with the PWA/service worker registered, I can sign in on the hive without unregistering the service worker first.
- **US3:** As a user whose OAuth flow fails or stalls, I see a visible error with a retry option instead of an endless spinner, so the failure is diagnosable.

## Success criteria
SC1: The deployed hive's served service worker excludes /v1/oauth from the runtime api-cache: `curl -fsS http://127.0.0.1:9000/sw.js` contains the compiled `v1/oauth` exclusion, and after a sign-in DevTools Cache Storage `api-cache` holds no `/v1/oauth/*` entries.
→ US2
SC2: With the hive SW registered (normal window, no manual unregister), GitHub sign-in initiated from the node completes: the node's `/api/auth/status` flips to authenticated and the signed-in user holds their permissions on the running node.
→ US1
SC3: Sign-in on the hive itself completes with the SW registered — observed as an authenticated session on the running hive with no unregister step performed.
→ US2
SC4: On a stalled OAuth flow on the running node, OAuthDialog stops polling `/api/auth/status` after the bounded deadline and renders a visible error state with retry — observable as the polling requests ceasing in the network log and the error UI appearing.
→ US3

## Users
Every node user: cannot sign in at all from a normal browser profile (the popup traverses the hive's SW) — a hard blocker on doing any work on a node. Every hive user with the PWA/SW registered: sign-in fails until they manually unregister the SW, a step no end user will discover. Operators: auth failures are undiagnosable because the UI never surfaces an error.


## Constraints
The deployed node lives at `/home/david/Tools/vk-swarm` — treat as build output; never edit directly. All work happens in `/data/Code/vk-swarm` (or a worktree of it) via WAI. Confirm the dominating failure mechanism (redirected-response rejection vs stale cache) during deploy verification — the exclusion fix is correct under both, so this is evidence-gathering, not a fix blocker. Precedent to follow: `/v1/shape` is already excluded from the same rule (adversarial review F3, recorded in `remote-frontend/vite.config.ts`); mirror that shape, do not invent a new caching strategy. The shell-cache rule already special-cases `/oauth/callback` and `/invitations/*/complete`; keep those exclusions intact. Standard repo gates (CLAUDE.md): clippy -D warnings, cargo test --workspace, both frontends lint + tsc, remote-frontend vitest.


## Out of scope
Node-side OAuth handoff hardening (single-use replay tolerance, in-memory-state restarts in `crates/server/src/routes/oauth.rs`) — promoted into scope only if the confirmed SW fix proves insufficient; otherwise it remains a hypothesis, not a defect. Any broader PWA/caching strategy redesign for the hive. A hive-side audit of other cached auth/session endpoints beyond the `/v1/oauth` exclusion (file a follow-up finding if surfaced during work).


## Approach
Three small, independently testable changes. (1) Exclude `/v1/oauth` from the hive SW's NetworkFirst api-cache rule, exactly mirroring the existing `/v1/shape` exclusion in the same predicate. (2) Extract that URL predicate from `remote-frontend/vite.config.ts` into a small pure module (`remote-frontend/src/lib/swCachePredicate.ts` or equivalent) imported by the vite config, so vitest can pin the exclusion and prevent regression — the config today is untestable inline data. (3) Give `OAuthDialog` (node frontend) a bounded polling window and an error branch: stop polling `/api/auth/status` after a deadline, render a visible error state with a retry affordance, and clear timers on unmount. Deploy verification then observes the rebuilt `sw.js` on the running hive and a real sign-in with the SW registered, capturing which failure mechanism dominated for the audit trail.


## Design
Change 1 — predicate: the api-cache `urlPattern` becomes `url.pathname.startsWith('/v1/') && !url.pathname.startsWith('/v1/shape') && !url.pathname.startsWith('/v1/oauth')`. `/v1/oauth` covers both legs (`/v1/oauth/{provider}/start` and `/v1/oauth/{provider}/callback`) and the two POSTs are unaffected (Workbox only caches GET). Excluded requests fall through to the network — no SW respondWith, so redirected responses reach the browser natively and nothing lands in `api-cache`.

Change 2 — testability: the predicate lives in a pure exported function `isApiCacheable(pathname: string): boolean` in `remote-frontend/src/lib/` (no vite imports), consumed by `vite.config.ts` inside the `urlPattern` closure. Vitest pins: `/v1/oauth/github/start` false, `/v1/oauth/github/callback` false, `/v1/shape/...` false, `/v1/projects` true, `/other` false. The compiled `sw.js` therefore contains the `v1/oauth` literal — which is what `verify_cmd` greps on the deployed hive.

Change 3 — OAuthDialog: introduce `POLL_DEADLINE_MS` (120000) and an `error` state. The status-poll effect records its start time; when exceeded without success it clears the interval and sets the error state. The dialog renders an i18n'd error message plus a retry button that resets state and restarts the flow; popup-closed-without-auth also transitions to error rather than silent spin. Timers cleared on unmount. Follows existing component/i18n conventions (en/ja/ko/es keys).

No backend changes. No wire-format, schema, or contract changes.


## Decisions
D1 — Fix by cache-exclusion prefix `/v1/oauth`, not by disabling the SW or narrowing the whole `/v1/` rule. Reversible (single predicate term). Mirrors the `/v1/shape` precedent in the same rule. D2 — Extract the predicate into a pure module so the exclusion is pinned by a unit test; the vite config keeps zero logic. Reversible. D3 — OAuthDialog gets a 120s poll deadline and an explicit error state with retry, instead of infinite polling. 120s is generous for a human OAuth round-trip and bounded enough to surface failure; value is a named constant. Reversible. No irreversible decisions — nothing is deleted, migrated, renamed, or contract-changing; no ADR required.


## Test strategy
TS1: remote-frontend vitest unit test on the extracted cache predicate: `/v1/oauth/github/start` and `/v1/oauth/github/callback` are not cacheable; `/v1/shape/...` not cacheable; `/v1/projects` cacheable; non-/v1 paths not cacheable. Written failing-first against the current inline predicate location.
TS2: frontend (node UI) vitest for OAuthDialog with fake timers: exceeding the poll deadline clears the interval and renders the error state with retry; a successful status flip before the deadline still resolves; timers cleared on unmount. Failure branch written failing-first.
TS3: Full repo gates green: cargo clippy -D warnings, cargo test --workspace, frontend lint + tsc, remote-frontend lint + tsc + vitest run. Deploy verification (pre-close) captures the rebuilt sw.js grep and a live sign-in with the SW registered.

