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
siblings: ["docs/configuration-customisation/agent-configurations.mdx","docs/configuration-customisation/network-access.mdx","docs/configuration-customisation/creating-task-tags.mdx"]
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
**After:** an mdx page whose frontmatter matches its siblings (read `agent-configurations.mdx` for the exact `title`/`description` shape and voice). Required sections:
- **What changed** — every browser must complete Hive OAuth before it can read or change node data.
- **Trusted-LAN plaintext risk** — the session cookie is `HttpOnly; SameSite=Lax; Path=/` and deliberately has NO `Secure` attribute so it works on plain HTTP (D9). State plainly: on plain HTTP anyone who can observe LAN traffic can copy the session cookie and act as the owner; run the node only on a network you trust; TLS is future work and out of scope here. Cross-reference `network-access.mdx`, which covers exposing the node beyond loopback, rather than contradicting it.
- **Sign in / sign out / disconnect** — a table of the three actions, the exact endpoint each uses (`POST /api/auth/handoff/init` + `GET /api/auth/handoff/complete`; `POST /api/auth/browser/logout`; `POST /api/auth/logout`), effect of each action (sign in authorizes only the presenting browser and revokes nothing / sign out revokes only this browser / disconnect revokes every browser plus daemon credentials and stops sync), and where each lives in the UI (navbar Sign out vs Settings -> Swarm -> Disconnect from Hive).
- **Node ownership** — the first Hive account to sign in is pinned as the owner; a different account is refused; the owner is RETAINED across disconnect so a disconnected node cannot be claimed by someone else; there is deliberately no operator-facing owner reset in this release, and recovery means recreating the node's database.
- **Hive outages** — established browser sessions and local project/task/execution work keep working through Hive transport, timeout, refresh and 5xx failures; only NEW sign-ins need Hive.
- **Cross-node streaming** — node-to-node HTTP proxy requests use a Hive-issued `node_proxy` token; direct raw/live logs and the production attempt-id diff WebSocket use only a Hive-issued `connection` token; the by-task-id diff WebSocket is browser-session-only. State that neither audience is accepted by the other route class and no anonymous fallback remains.
- **Verifying a deployment** — `bash scripts/verify-local-node-browser-oauth.sh http://<node-host>:<port>` with the expected PASS output copied from a real run.

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
  "Documenting cross-node direct diff as browser-only, or documenting node_proxy access to either diff route — task 013 preserves only the attempt-id connection-token path."
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
