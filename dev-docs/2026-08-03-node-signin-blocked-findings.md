# 2026-08-03 — Node sign-in blocked (F-2026-08-03-01, F-2026-08-03-02)

Reported by the user during the `vk-swarm-node-ui-localize` close-out. Filed as findings rather
than fixed in-flight: the frozen spec (ADR-0001) for that workstream covers hive-proxy route
restoration only, and auth is in none of its success criteria.

## Symptoms (user-reported)

1. **A user cannot sign in on a NODE.** The OAuth process "doesn't follow through". Because sign-in
   never completes, the user holds no permissions and cannot work on the node. This is the blocking
   one.
2. **The hive is treated as an installed PWA**, causing issues until the user *unregisters it as a
   PWA first*. After unregistering, **sign-in on the hive works**.

## Not caused by `vk-swarm-node-ui-localize` — static proof

Stated precisely: the branch diff **cannot** have caused this. This is a static argument about the
diff, NOT an empirical before/after of the login flow (the 501 live capture covered six API paths,
not sign-in).

```text
$ git diff $(git merge-base HEAD origin/main)..HEAD -- \
    crates/server/src/middleware/ crates/server/src/routes/oauth.rs crates/server/src/routes/organizations.rs
(empty)
```

The only change to `crates/server/src/routes/mod.rs` is additive — four `.merge()` calls appended;
`oauth::router()` and `organizations::router()` are untouched and keep their position. The sole
frontend delta touching an org surface is `OrganizationSettings.tsx` (−5 lines, the task-201
API-key section removal), which is not on the sign-in path.

## F-2026-08-03-01 — node OAuth handoff does not complete

**Location:** `crates/server/src/routes/oauth.rs:80-119` (`handoff_complete`).

The node's flow is `POST /auth/handoff/init` → browser → `GET /auth/handoff/complete`. The
correlating state is consumed by `deployment.take_oauth_handoff(&query.handoff_id)` — a **single-use
take from in-memory state**. On a miss the handler returns `400` with
`"OAuth handoff not found or already completed"` (`oauth.rs:98-107`).

Hypotheses to test (not verdicts — no reproduction attempted yet):

- **Single-use + replay.** Any second delivery of the callback (browser retry, prefetch, a service
  worker replaying the navigation, double-click) consumes the handoff on the first and fails the
  second. The user-visible result is exactly "doesn't follow through".
- **In-memory + restart.** A server restart between `init` and `complete` drops the handoff
  entirely. Relevant for a node redeployed mid-flow.
- **Downstream leg.** `handoff_complete` also calls `client.handoff_redeem(...)` and then
  `save_credentials`; a failure in either surfaces differently. Check node logs for the
  `"received callback for unknown handoff"` warning (`oauth.rs:99-102`) to discriminate — its
  presence points at the take/replay path, its absence at the redeem or persistence leg.

**First diagnostic step:** reproduce with the node's `RUST_LOG=debug` and capture whether that
warning fires.

## F-2026-08-03-02 — hive service worker caches auth responses

**Location:** `remote-frontend/vite.config.ts:16-28`.

The first `runtimeCaching` rule is:

```ts
urlPattern: ({ url }) =>
  url.pathname.startsWith('/v1/') && !url.pathname.startsWith('/v1/shape'),
handler: 'NetworkFirst',
options: { cacheName: 'api-cache', expiration: { maxEntries: 100, maxAgeSeconds: 300 } },
```

This caches **every** `/v1/` REST response for up to 300s, excluding only `/v1/shape`. Auth
endpoints under `/v1/` are therefore cached. A stale `401`/session response served from `api-cache`
would explain "unregister the PWA and then sign-in works" — unregistering drops the cache.

Note the shell-cache rule below it already special-cases `/oauth/callback` and
`/invitations/*/complete` (returns `false` for them), so the risk of caching auth-adjacent
navigations was recognised for the shell but **not** for the `/v1/` API rule.

`registerType: 'autoUpdate'` plus `remote-frontend/src/lib/pwa.ts` reloading the window on an
updated SW activation (`pwa.ts:15-19`) may compound this by reloading mid-flow.

**Possible interaction between the two findings:** if the hive SW reloads or replays a navigation
during the node handoff, it would consume the node's single-use handoff (F-...-01) and produce the
node symptom. Worth testing together, but do not assume a shared root cause at filing time.

## Constraints for whoever picks this up

- The deployed node lives at `/home/david/Tools/vk-swarm`. **Do not edit it directly** — all work
  goes through `/data/Code/vk-swarm` and the WAI workflow. Treat that directory as build output.
- That path has previously hosted a separate vibe-kanban instance on port 9002; confirm the
  topology before touching any process, and never use `pkill`/`killall` (CLAUDE.md).
- After `/wai:ship` merges `vk-swarm-node-ui-localize`, the node at `10.69.96.233:9001` must be
  redeployed off the feature branch onto `main`.
