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

### 2026-08-03 follow-up: the two findings share a mechanism

**The hive's OAuth endpoints live under `/v1/`, i.e. inside the cached prefix.**
`crates/remote/src/routes/oauth.rs:27-30` declares:

```text
POST /oauth/web/init
POST /oauth/web/redeem
GET  /oauth/{provider}/start
GET  /oauth/{provider}/callback
```

and `crates/remote/src/routes/mod.rs:112-113` mounts both routers with `.nest("/v1", ...)`. So the
real paths are `/v1/oauth/{provider}/start` and `/v1/oauth/{provider}/callback` — **GETs**, which is
precisely what Workbox's `NetworkFirst` rule caches. (The two `POST`s are not cached by default.)

Caching `GET /v1/oauth/{provider}/start` is the concrete hazard: that endpoint mints an OAuth
`state`. A replayed/stale response reuses a dead `state`, so the handoff record never matches and
sign-in cannot complete. `NetworkFirst` also falls back to cache on any network hiccup, which
serves a stale redirect rather than failing loudly.

**This is why the node symptom and the hive symptom are probably one bug.** The node's login is a
POPUP (`frontend/src/components/dialogs/global/OAuthDialog.tsx:116-124`) that navigates to the
**hive origin** — where the hive's service worker is registered and intercepting. So a node user's
login traverses the hive's SW even though the node itself has no SW. Unregistering the PWA clears
that cache, which matches the user's report that hive sign-in then works.

Still a hypothesis, not a verdict: not yet reproduced.

### Ruled out (do not re-investigate)

- **`crypto.subtle` / non-secure-context PKCE (the F-2026-07-06-02 class).** The node runs at
  `http://10.69.96.233:9001`, a non-loopback plain-HTTP origin, so this looked likely. It is not:
  `grep -rn "crypto\.|isSecureContext|subtle" frontend/src` returns only CSS/comment matches. The
  node does PKCE **server-side** in Rust (`routes/oauth.rs:48-49`, `generate_secret` +
  `hash_sha256_hex`).
- **Hive `return_to` allowlist rejecting the node's origin.**
  `is_allowed_return_to` (`crates/remote/src/auth/handoff.rs:488-507`) special-cases loopback and
  the public origin, then **falls through to `true`** for everything else ("allowing external
  redirect URL ... rely on PKCE"). It rejects nothing.
- **`returnTo` pointing at the wrong host.** Derived from
  `import.meta.env.VITE_APP_BASE_URL || window.location.origin`
  (`OAuthDialog.tsx:120`), which is correct for a user browsing the node directly.

## Constraints for whoever picks this up

- The deployed node lives at `/home/david/Tools/vk-swarm`. **Do not edit it directly** — all work
  goes through `/data/Code/vk-swarm` and the WAI workflow. Treat that directory as build output.
- That path has previously hosted a separate vibe-kanban instance on port 9002; confirm the
  topology before touching any process, and never use `pkill`/`killall` (CLAUDE.md).
- After `/wai:ship` merges `vk-swarm-node-ui-localize`, the node at `10.69.96.233:9001` must be
  redeployed off the feature branch onto `main`.
