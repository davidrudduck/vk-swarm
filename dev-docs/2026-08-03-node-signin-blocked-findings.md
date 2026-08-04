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

**A possible link between the two symptoms — but the user's own report probably contradicts it.**
The node's login is a POPUP (`frontend/src/components/dialogs/global/OAuthDialog.tsx:116-124`) that
navigates to the **hive origin**, where the hive's service worker is registered and intercepting. So
a node user's login traverses the hive's SW even though the node serves no SW of its own. That is a
mechanism by which one root cause could produce both symptoms.

### 2026-08-04 — ROOT CAUSE CONFIRMED by controlled experiment

User result:

> "opening the hive, unregistering the service worker, switching tabs to the node and logging in via
> github oauth **works**."

| Hive service worker | Node sign-in |
|---|---|
| registered | spins forever |
| unregistered | **works** |

Single variable changed, both directions observed. **The hive's service worker is the root cause of
the node sign-in blocker.** The competing browser-extension hypothesis is disproven — extensions
were active in both arms of this experiment.

**Why the SW sits on the node's login path at all.** Both OAuth legs are hive-origin `/v1/` URLs:

- `authorize_url` = `{public_origin}/v1/oauth/{provider}/start?handoff_id={id}`
  (`crates/remote/src/auth/handoff.rs:157-162`) — what the node's popup navigates to.
- the provider `redirect_uri` = `{public_origin}/v1/oauth/{provider}/callback`
  (`crates/remote/src/auth/handoff.rs:198-202`) — where GitHub sends the browser back.

Both are GET navigations on the hive origin matching
`url.pathname.startsWith('/v1/') && !url.pathname.startsWith('/v1/shape')`
(`remote-frontend/vite.config.ts:19-20`). So the SW intercepts the **entire OAuth redirect chain**,
even though the user started on the node and the node serves no SW of its own.

Two mechanisms are consistent with "spins forever"; the fix is the same either way, and pinning
which one dominates is a task for the workstream, not a blocker for it:

1. **Redirected responses through a SW.** These legs are 302 chains. A navigation served from a
   service worker whose response is `redirected: true` is rejected by the browser, so the popup
   navigation dies silently. The parent then polls `/api/auth/status` forever
   (`OAuthDialog.tsx:95-113`), which is exactly the observed spin.
2. **`NetworkFirst` cache fallback.** Any network hiccup serves a stale `/v1/oauth/*` response
   rather than failing loudly; `/v1/oauth/{provider}/start` rejects a non-`Pending` record
   (`handoff.rs:194-196`), so a replayed leg cannot succeed.

**The fix has a precedent in the same rule.** `/v1/shape` is already excluded from this cache for an
analogous reason (streaming/long-poll traffic must not be cached — recorded in the file as
"adversarial review F3"). Excluding `/v1/oauth` is the same shape of change.

**Second, independent defect surfaced by this.** `OAuthDialog` has **no failure branch** — it polls
until success with no timeout and no error state, which is why a broken flow presents as an
indefinite spinner rather than an error. Worth fixing alongside, so the next auth regression is
diagnosable instead of silent.

### 2026-08-04 — user reproduction data REINSTATES the link

The user tested the discriminating question and reported:

> "logging in direct to the hive, service worker must be disabled. in incognito mode, the node
> auth'ed against github as expected and **worked**. but even after clearing all storage, cookies,
> browser refresh, etc, trying to log in via the node **without** incognito mode results in the
> node's auth page **just spinning** after clicking 'Sign in with Github' on the hive based page."

So the differential is:

| Context | Node sign-in |
|---|---|
| Incognito (no SW, no extensions) | **works** |
| Normal window, after clearing all storage + cookies | **spins forever** |

This retracts the retraction below: the earlier "plain reading" of the original report was wrong,
and the SW-link hypothesis is now **supported by observation**. The node blocker and the hive PWA
issue are very likely one bug after all.

**Why "cleared cookies and storage" does not exonerate the service worker:** clearing cookies and
site storage does **not** unregister a service worker. A registered SW persists until explicitly
unregistered (DevTools → Application → Service Workers → Unregister, or ticking "Unregister service
workers" in Clear site data). Incognito has no SW at all. That asymmetry fits the evidence exactly.

"Just spinning" is the expected UI for this: `OAuthDialog` polls `/api/auth/status` until it flips
to logged-in (`OAuthDialog.tsx:95-113`). If the handoff never completes, it polls forever — there is
no failure branch, which is itself a UX defect worth capturing separately.

**Competing hypothesis that fits the SAME signature — do not skip it.** Browser **extensions** are
also disabled in incognito by default. An extension interfering with the popup or the OAuth
redirect would produce identical incognito-works/normal-fails behaviour. Discriminate cheaply:

1. Normal window → DevTools → Application → Service Workers → **Unregister** on the hive origin.
   Retry node sign-in. Works → service worker confirmed.
2. Still fails → disable extensions (or use a clean browser profile with SW allowed) and retry.
   Works → an extension, not the SW.
3. While reproducing, check Application → Cache Storage → `api-cache` for `/v1/oauth/*` entries, and
   capture the Network tab for the popup — specifically whether any request shows
   `(from ServiceWorker)`.

**Superseded reasoning, kept for the audit trail.** Before the reproduction above, the user's
original wording (*"signin then works on the hive, but a user on a node cant login"*) was read as
meaning node sign-in still failed WITHOUT a service worker, which would have made these two
independent bugs. That reading is now disproven by the incognito result. Recorded so the reversal
is visible rather than silently overwritten.

Neither symptom has been reproduced by the assistant locally; the evidence above is the user's. `F-2026-08-03-02` stands on its own merits regardless
of the answer: caching auth endpoints is a defect independent of what it currently breaks.

**Also unconfirmed:** that `/v1/oauth/*` responses actually land in the cache. `start` returns a
302, and Workbox `NetworkFirst`'s default `cacheableResponse` is `[0, 200]`, so the redirect may
never be cached. Check DevTools → Application → Cache Storage → `api-cache` for `/v1/oauth/*`
entries. Reading the config is not the same as observing the cache — if nothing lands there, this
hypothesis dies cheaply.

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
