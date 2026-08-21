---
doc_type: spec
status: active
workstream: local-node-browser-oauth
change_kind: behaviour
verify_cmd: "bash scripts/verify-local-node-browser-oauth.sh"
---

# Local Node Browser OAuth

## Intent
Require every trusted-LAN browser to prove the node's single Hive owner identity before it can read or mutate node-local data. Extend the existing node-to-Hive OAuth handoff into a browser-bound, one-time authorization flow without exposing daemon Hive credentials to the browser or making established local operation depend on continuous Hive availability.


## User stories
- **US1:** As the node owner, when I open the node in a fresh browser, I expect only the login shell and public health information until that browser completes Hive OAuth.
- **US2:** As the node owner, when I authorize one browser, I expect that browser alone to receive a persistent local session tied to my stable Hive identity.
- **US3:** As the node owner, when I log out one browser or explicitly disconnect the node from Hive, I expect revocation to match the action without leaking daemon credentials.
- **US4:** As the node owner, when Hive is temporarily unavailable after login, I expect established browser sessions and local project, task, execution, and stream operations to remain available.

## Success criteria
SC1: An unauthenticated request to any protected `/api` endpoint returns `401`, while static assets, health, OAuth initiation/completion, and a minimal auth-state response remain reachable.
→ US1
SC2: An unauthenticated WebSocket or SSE connection to every node-local protected stream is rejected before upgrade or stream establishment.
→ US1
SC3: Completing OAuth in browser A authorizes browser A, leaves clean browser B unauthorized, and copying or replaying A's callback URL cannot authorize B.
→ US2
SC4: A handoff expires after 10 minutes, has exactly one successful consumer, and a completed, expired, copied, or replayed handoff cannot mint another browser session.
→ US2
SC5: A successful browser session uses an opaque persistent HTTP-only cookie, stores only a token hash server-side, survives browser and planned idle node restart, and has no routine server-side expiry.
→ US2
SC6: The first successful owner authorization durably pins the stable Hive subject; later authorization for the same subject succeeds and a different subject is rejected without replacing credentials or existing sessions.
→ US2
SC7: Browser logout revokes only the presenting browser session and leaves other browser sessions plus node-to-Hive credentials and synchronization intact.
→ US3
SC8: Explicit Hive disconnect revokes every browser session, stops synchronization, and removes daemon Hive credentials while retaining the pinned owner identity.
→ US3
SC9: After login, Hive transport, timeout, refresh, or 5xx failure leaves established browser sessions and local protected operations usable; it neither invokes explicit disconnect nor enables new OAuth completion.
→ US4
SC10: Browser-visible storage, URLs, responses, rendered UI, and logs contain no Hive access token or refresh token during initiation, completion, normal use, logout, disconnect, or failure.
→ US3

## Users
- The single operator who owns and runs a local node on a trusted LAN.
- Maintainers of node HTTP, WebSocket, SSE, OAuth, local persistence, and frontend bootstrap paths.
- Later executor milestones, which depend on this authorization boundary but must not receive Hive credentials in browser-visible state.


## Constraints
- Hive remains the only identity provider; no local password, account database, RBAC, or multi-user authorization model is introduced.
- Trusted-LAN plain HTTP remains supported. Session cookies are HTTP-only and persistent but do not require the `Secure` attribute; documentation must state the plaintext-session risk.
- The node supports one durably pinned Hive owner subject. A different subject cannot take ownership through the normal browser OAuth flow.
- Established authorization is checked locally. Transient Hive connectivity or token-refresh failure is distinct from the operator's explicit disconnect action.
- Authorization is deny-by-default for node-local APIs and long-lived streams; only explicitly named bootstrap routes remain public.
- Tests that need SQLite use the shared migrated test-pool utilities rather than duplicated schema.


## Out of scope
- TLS provisioning, the cookie `Secure` attribute, CORS or Origin enforcement, CSRF hardening beyond the selected same-site/browser-bound OAuth flow, secondary credentials, RBAC, and multiple owners.
- An operator-facing owner-reset or ownership-transfer workflow.
- Hive project-management parity, attached/direct executor certification, execution-lifecycle repair, crash continuation, and remote node administration.
- Changing Hive's provider OAuth implementation except where a narrowly scoped handoff contract fix is required to make single-consumer redemption correct.


## Approach
Add an additive local authorization layer around the existing Hive OAuth exchange. Persist a single owner subject, browser-bound handoff claims, and hashed opaque sessions in the migrated local SQLite database. Build explicit public and protected routers so authorization is inherited by default, split frontend bootstrap into minimal public auth state followed by protected application data, and keep browser-session validity independent of ordinary Hive transport or refresh failures. Preserve the existing explicit Hive disconnect behavior as a separate protected action and add a browser-session logout action.


## Design
### Persistence and token model
Add migrated local tables for one UUID-identified owner row constrained to a singleton slot, browser OAuth handoffs, and browser sessions. Handoffs store the Hive handoff UUID, provider, app verifier, SHA-256 hash of a random pre-auth browser-binding token, creation/expiry timestamps, and terminal state. Sessions use UUID v4 identities, store only SHA-256 hashes of 256-bit base64url tokens, reference the pinned Hive subject, and carry creation/revocation metadata but no routine expiry. Raw session and binding tokens never enter logs or API bodies.

### Browser-bound OAuth state machine
OAuth initiation creates or refreshes an HTTP-only pre-auth binding cookie and stores its hash with a handoff expiring exactly 10 minutes later. Callback atomically claims only a pending, unexpired handoff whose binding hash matches the presenting cookie; wrong-browser and expired attempts do not consume a rightful pending handoff, while only one matching concurrent claimant wins. A claimed handoff is terminal after redemption success or failure, so replay never mints another session. A transient or crash failure requires a fresh OAuth initiation rather than guessing whether Hive consumed the one-time code.

The callback redeems into candidate credentials, fetches `ProfileResponse.user_id` with those candidate credentials before replacing daemon credentials, and atomically pins the first owner subject or confirms equality with the existing owner. A different subject is rejected without saving candidate credentials, changing the owner, or revoking existing sessions. After owner validation, credential persistence must succeed before a local session is committed. A crash after first-owner pinning can leave only that subject pinned; the same owner can safely retry.

### Cookie and revocation contracts
The authorized-session cookie is `HttpOnly`, `SameSite=Lax`, `Path=/`, persistent for five years, and intentionally lacks `Secure` for supported plain-HTTP LAN deployment. Server authorization evaluates only the stored token hash and revocation state, not Hive availability or time-based session expiry. Browser logout deletes/revokes only the presenting session and expires its cookie. The current daemon/Hive disconnect operation remains a separately named protected action: revoke all sessions first, stop sync, then remove daemon credentials; retain the pinned owner so another Hive subject cannot claim a disconnected node through normal OAuth. Owner replacement/reset remains out of scope, but any internal owner-replacement operation must revoke all sessions in the same transaction.

### HTTP and stream boundary
Construct explicit public and protected routers rather than annotating handlers individually. Public routes are static SPA assets, health, OAuth initiation/callback, and a minimal auth-state response containing only browser authorization and OAuth availability fields. `/api/info`, configuration, projects, tasks, attempts, execution/log data, diagnostics, terminal access, SSE, and node-local WebSockets are protected. Unknown `/api/*` requests terminate inside the API boundary and never fall through to SPA HTML. Browser session authentication occurs before route-specific resource validation or protocol upgrade. Direct Hive-issued connection-token log streaming may remain a separately authenticated, explicitly scoped mode, but absence of both a valid connection token and browser session is unauthorized.

### Frontend bootstrap and outage behavior
Application startup requests only public auth state. An unauthorized browser renders the login shell and starts no protected query, SSE, or WebSocket. An authorized browser then loads `/api/info` and the normal application; a later `401` tears down protected live connections and returns to login. The navbar logout invokes browser-session logout, while protected settings expose explicit Hive disconnect. Hive timeout, transport, refresh, and 5xx errors preserve owner, daemon credentials, and browser sessions; only a confirmed authentication rejection may clear invalid daemon credentials, and even that does not impersonate the explicit disconnect action. Existing local sessions continue to authorize local APIs while new OAuth requiring Hive is unavailable.

### Verification and documentation
Use injected clock/random seams for expiry and token tests, the migrated DB helpers for persistence tests, the served-router/Wiremock harness with independent cookie jars for real HTTP behavior, protocol-level WebSocket/SSE rejection tests, and frontend tests that prove protected startup does not happen before authorization. Add an operator document for trusted-LAN plaintext risk, login/logout/disconnect semantics, outage behavior, and owner-reset limitations. Add `scripts/verify-local-node-browser-oauth.sh` as a deployed-node observation that proves public health/auth-state remain reachable and a protected endpoint returns `401` without a session.


## Decisions
D1 (reversible): Split the node router into explicit public and protected subtrees so future routes inherit denial by default.

D2 (reversible): Persist owner, handoff, and session records in local SQLite and store only SHA-256 token hashes.

D3 (reversible): Bind each handoff to an HTTP-only pre-auth browser cookie and atomically claim it before Hive redemption.

D4 (reversible): Pin the first Hive subject, reject different subjects, retain owner identity across explicit disconnect, and leave owner reset out of scope.

D5 (reversible): Add browser-session logout while preserving explicit daemon/Hive disconnect as a separate protected operation.

D6 (reversible): Authorize established sessions locally and distinguish connectivity/refresh failure from explicit disconnect.

D7 (reversible): Require browser sessions for node-local streams while retaining valid Hive connection tokens only as an explicit alternative on existing direct-log routes.

D8 (reversible): Bootstrap the frontend from minimal public auth state before loading protected configuration or streams.

D9 (reversible): Use a five-year `HttpOnly; SameSite=Lax; Path=/` session cookie without `Secure` on the accepted plain-HTTP LAN boundary.


## Test strategy
TS1: Deterministic database/service tests use injected clock and randomness to cover exact handoff expiry, wrong-browser non-consumption, concurrent single claim, replay rejection, hash-only persistence, owner pin races, session persistence, and scoped/all-session revocation.
TS2: Real served-router tests with Wiremock Hive and two independent cookie jars cover public/protected HTTP routing, browser-A isolation, callback copying/replay, cookie attributes, same-owner and different-owner redemption, browser logout, and explicit disconnect.
TS3: A table-driven protocol test attempts every protected WebSocket and SSE route anonymously and with an authorized browser session, asserting authentication runs before upgrade and route-specific validation; direct-log connection-token mode rejects missing authentication.
TS4: Restart and outage tests reuse the same migrated SQLite/assets directory, make Hive time out or return 5xx, and prove an established cookie still reaches local project/task/stream seams while fresh OAuth remains unavailable and no disconnect side effect occurs.
TS5: Frontend tests prove unauthorized bootstrap requests only minimal auth state, authorized bootstrap then loads protected application state, `401` tears down protected live connections, and browser logout remains distinct from Hive disconnect.
TS6: Sentinel access/refresh tokens are injected through the real OAuth harness and assertions scan responses, redirect locations, cookies, browser storage, rendered text, and captured logs for zero browser-visible disclosure.
TS7: A real non-loopback trusted-LAN browser acceptance run on the feature-branch build records two-browser isolation, restart persistence, transient Hive outage continuity, logout/disconnect scope, and the deployed verify script's public-versus-protected HTTP observations.
