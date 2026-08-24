---
id: "020"
phase: 4
title: "Add the trusted-LAN browser-authorization operator document"
status: ready
depends_on: ["012","019"]
parallel: false
conflicts_with: []
files:
  - "docs/configuration-customisation/browser-authorization.mdx"
  - "docs/docs.json"
siblings: ["docs/configuration-customisation/agent-configurations.mdx","docs/configuration-customisation/network-access.mdx","docs/configuration-customisation/creating-task-tags.mdx","docs/configuration-customisation/database-performance.mdx"]
irreversible: false
scope_test: "N/A"
allowed_change: mixed
covers_criteria: []
covers_tests: []
---
## Failing test (write first)
N/A — this task ships operator prose and one navigation entry; a unit test cannot cheaply assert whether a document is truthful or complete. Verification is the `## Manual verification` block below, which checks the JSON parses, the page is reachable in the nav, and — the part that matters — that every endpoint and behaviour the page claims is checked against the shipped code.


## Change
**File:** `docs/configuration-customisation/browser-authorization.mdx` — create.
**Anchor:** new page beside `network-access.mdx` in the same directory.
**Before:** (does not exist)
**After:** create this page **byte-for-byte** (frontmatter is the sibling `title`/`description` shape from `agent-configurations.mdx`; headings are sentence-case like `creating-task-tags.mdx`; do not add `sidebarTitle`/`audience`/`generated` — those belong to the remote-frontend network-access page). Grounded labels: login button is `Log in` (`AuthBoundary.tsx:100`); navbar item is `Sign out` (`frontend/src/i18n/locales/en/common.json:106`); disconnect button default is `Disconnect from Hive` (`SwarmSettings.tsx:205`). Cookies: `vks_browser_session` and `vks_browser_binding` are `HttpOnly; SameSite=Lax; Path=/` with **no** `Secure` (`cookies.rs:32-54`). Verifier transcript is from a real `bash scripts/verify-local-node-browser-oauth.sh` run against the task-019 compliant fixture (not invented; do not start `pnpm run dev`).

```mdx
---
title: "Browser Authorization"
description: "How browsers sign in to a local node over a trusted LAN, and what that means for cookies, ownership, and Hive outages"
---

Every browser must complete Hive OAuth before it can read or change data on this node.

## What changed

Before this release, a browser on the same network as the node could use it without signing in. Now every browser must finish Hive OAuth first. The node still serves local project, task, and execution work itself; Hive is required only to authorize a new browser or to keep the node's daemon credentials fresh.

This page is about the **local node's** browser session. Binding the node beyond loopback, and Hive's own remote-frontend OAuth, are covered in [Network Access](/configuration-customisation/network-access). Do not mix the two.

## Trusted-LAN plaintext risk

The session cookie is named `vks_browser_session`. The short-lived OAuth binding cookie is named `vks_browser_binding`. Both are set as `HttpOnly; SameSite=Lax; Path=/`. They deliberately have **no `Secure` attribute** so they are sent on plain HTTP (the supported trusted-LAN deployment).

<Warning>
On plain HTTP, anyone who can observe LAN traffic can copy `vks_browser_session` and act as the owner. Run the node only on a network you trust. TLS is future work and is out of scope here. How to bind the node beyond loopback is in [Network Access](/configuration-customisation/network-access).
</Warning>

## Sign in / sign out / disconnect

| Action | Endpoints | Effect | Where in the UI |
|---|---|---|---|
| Sign in | `POST /api/auth/handoff/init` then `GET /api/auth/handoff/complete` | Authorizes only the presenting browser. Revokes nothing. | Login shell → **Log in** |
| Sign out | `POST /api/auth/browser/logout` | Revokes only this browser's session. Other browsers and the node's Hive credentials stay. | Navbar menu → **Sign out** |
| Disconnect | `POST /api/auth/logout` | Revokes every browser session, removes the node's Hive credentials, and stops synchronisation. The owner pin is retained. | Settings → Swarm → **Disconnect from Hive** |

## Node ownership

The first Hive account to sign in is pinned as the owner. A different account is refused. The owner is retained across disconnect, so a disconnected node cannot be claimed by someone else.

There is no operator-facing owner reset in this release. Recovery means recreating the node's database.

## Hive outages

Established browser sessions and local project, task, and execution work keep working through Hive transport, timeout, refresh, and 5xx failures. Only new sign-ins need Hive.

## Cross-node streaming

- Node-to-node HTTP proxy requests use a Hive-issued `node_proxy` token.
- Direct raw logs, live logs, and the production attempt-id diff WebSocket use only a Hive-issued `connection` token.
- The by-task-id diff WebSocket is browser-session-only.
- Neither audience is accepted by the other route class. There is no anonymous fallback. `node_proxy` is not accepted on either diff WebSocket.

## Verifying a deployment

```bash
bash scripts/verify-local-node-browser-oauth.sh http://<node-host>:<port>
```

Expected output from a real run of that script:

```
PASS health is public
PASS auth state is public
PASS auth state has the exact minimal shape
PASS info is protected
PASS projects are protected
PASS status is protected
PASS events SSE is protected
PASS live logs are protected
PASS unknown api path is 404
PASS unknown api path is not SPA html
All browser-authorization boundary checks passed
```
```

**File:** `docs/docs.json`
**Anchor:** the configuration-customisation page list, L83-91.
**Before:**
```json
          "configuration-customisation/network-access",
          "configuration-customisation/event-journal-retention"
```
**After:**
```json
          "configuration-customisation/network-access",
          "configuration-customisation/browser-authorization",
          "configuration-customisation/event-journal-retention"
```
One added line; no other JSON changes.

**Sibling alignment (rubric 9).** Read `docs/configuration-customisation/agent-configurations.mdx` for the frontmatter and heading conventions used by pages in this directory, and `docs/configuration-customisation/network-access.mdx` as the closest topical sibling — it already discusses binding the node beyond loopback, so this page must reference it, not restate or contradict it.

**Symbol grounding:** This task introduces no code symbols. It documents endpoints and behaviours introduced elsewhere: `auth_state()` (task 008), `browser_logout()` (task 012), `logout()` (pre-existing, amended by task 012) and `check_status()` in the verifier script (task 019).


## Allowed moves
[
  "Create the one mdx page and add exactly one line to docs/docs.json.",
  "No code, test or script changes."
]


## STOP triggers
[
  "The doc omitting or softening the plaintext-session risk, or implying that an operator-facing owner reset exists.",
  "Any endpoint, cookie name or UI label in the doc that does not match the shipped code — check each against the source before writing it.",
  "Editing any docs.json entry other than the single added page, or leaving docs.json unparseable.",
  "Documenting cross-node direct diff as browser-only, or documenting node_proxy access to either diff route — task 013 preserves only the attempt-id connection-token path.",
  "Inventing verifier output, starting pnpm run dev, or departing from the locked mdx page (including adding sidebarTitle/audience/generated or documenting remote-frontend /oauth/callback as the local-node sign-in)."
]


Declared decision points (from the spec; do not edit here):
- DP1: Execution reaches the additive SQLite migration task and requires explicit approval before applying an irreversible production schema migration.  [codes: human_gate_required]


## Manual verification (record in decisions-ledger)
1. `python3 -c "import json; json.load(open('docs/docs.json'))"` exits 0.
2. Every endpoint named in the doc is confirmed against the source: `git grep -n '\"/auth/' crates/server/src/routes/oauth.rs crates/server/src/routes/browser_auth.rs` — paste the output next to the doc's action table in the ledger and confirm they agree.
3. Cookie names and attributes in the doc match `crates/server/src/auth/cookies.rs` exactly (`vks_browser_session`, `vks_browser_binding`, no `Secure`).
4. The verifier output pasted into the doc is copied from a REAL run of `bash scripts/verify-local-node-browser-oauth.sh`, not invented.
5. Read the page end to end as an operator would; confirm the ownership section says plainly that there is no reset path in this release.
6. Cross-node classification matches the task 013/014 route census: attempt-id diff is browser OR connection, by-task-id diff is browser-only, and node_proxy is HTTP-only.


## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash "$HOME/.agents/wai/scripts/task-gate.sh" local-node-browser-oauth 020` exits 0
