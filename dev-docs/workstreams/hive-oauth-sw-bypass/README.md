---
workstream: hive-oauth-sw-bypass
status: shipped
created: 2026-08-05
parent_session: vk-swarm-node-ui-localize close-out
staging_pointers:
  - docs/plans/hive-oauth-sw-bypass
  - docs/superpowers/specs/2026-08-05-hive-oauth-sw-bypass.md
---

# hive-oauth-sw-bypass

The hive's service worker intercepts the OAuth redirect chain, making sign-in impossible on both
the hive and any node without manually unregistering the PWA. **Highest-priority blocker.**

Findings: `F-2026-08-03-02` (root cause), `F-2026-08-03-01` (node handoff, caused by -02),
`F-2026-08-04-01` (`OAuthDialog` polls forever with no timeout/error branch).

**Root cause CONFIRMED by controlled experiment** (SW registered → node sign-in spins; unregistered
→ works; extensions active in both arms, so they are excluded). Both OAuth legs are hive-origin
`/v1/` GET navigations — `authorize_url` (`crates/remote/src/auth/handoff.rs:157-162`) and the
provider `redirect_uri` (`:198-202`) — matching the `NetworkFirst` rule at
`remote-frontend/vite.config.ts:19-20`.

Full diagnosis, mechanism, and three RULED-OUT hypotheses:
`dev-docs/2026-08-03-node-signin-blocked-findings.md`. Read it before investigating.

Likely fix has a precedent in the same rule — `/v1/shape` is already excluded; `/v1/oauth` needs
the same. Confirm which mechanism dominates (SW-handled redirected navigation vs stale cache hit)
before settling on it.
