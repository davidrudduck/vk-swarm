# Decomposition tournament 1 — local-node-browser-oauth

Date: 2026-08-21

Target: the complete 21-task decomposition frozen against spec SHA `680587143dbb2f1cbe74d1f7ef78a250705d5686`

Method: three independent external CLI find/remediate seats, followed by rotated non-self peer judges. All final runner artifacts reported `status: ok`; no stub-risk override was used.

## Resilience record

The first Codex attempt observed only the precheck tokens because a concurrent plan-authoring continuation removed the then-untracked generated tree. Claude and OpenCode were stopped, that Codex result was discarded, and none of those attempts was scored. A later Claude attempt saw only tasks 001–011 during the same race and is likewise superseded. The strict envelope was resubmitted, the complete generated tree was staged to make its blobs durable, plan lint passed, and all three final seats were rerun against the same complete staged tree.

Final evidence sources:

| seat | find/remediate artifact | non-self judge | runner result |
|---|---|---|---|
| Codex / gpt-5.6-sol | `/tmp/opencode/browser-oauth-find-codex-rerun.md` | OpenCode / GLM 5.3 | both `ok` |
| Claude / Opus | `/tmp/opencode/browser-oauth-find-claude-staged.md` | Codex / gpt-5.6-sol | both `ok` |
| OpenCode / GLM 5.3 | `/tmp/opencode/browser-oauth-find-opencode-staged.md` | Claude / Opus | both `ok` |

The tables below preserve every submitted finding and every peer ruling in the durable review record; temporary paths identify the runner artifacts but are not the only copy of their content.

## Codex submission and GLM judgment

| id | submitted issue and remediation | peer ruling | final disposition |
|---|---|---|---|
| C1 | Task 016 rendered `login-start` but specified no click behavior. Add public handoff initiation, open `authorize_url`, and test the click/return URL. | issue YES, fix YES | Applied in task 016: `browserAuthApi.startLogin`, popup lifecycle, public `/api/auth/state` polling, and executable click-to-authorized test. |
| C2 | Task 018 frontend scan was vacuous: no sentinel reached a visible seam, protected app stayed unmounted, and `JSON.stringify(Storage)` was not a storage census. | YES / YES | Applied: unauthorized and authorized bootstrap, sentinel-bearing unexpected fields, exact generated JWT plus refresh sentinel, `length/key/getItem`, and mutation self-check. |
| C3 | Task 018 could not inspect `Location`, omitted disconnect, and overclaimed failure surfaces. Extend `Resp` and add executable phases. | YES / YES, with transport/timeout/refresh assigned to task 015 | Applied: all headers captured before body consumption; separate valid sessions exercise browser logout and successful Hive disconnect; task 015 owns general outage classes. |
| C4 | Task 015's later refresh failure was shadowed by the harness's unlimited success mock and no tested route actually forced refresh; timeout/transport were absent. | YES / YES | Applied: priority-1 signalled responders, exact request observation, real post-restart organization refresh trigger, seven deterministic continuity tests, and no retry sleeps. |
| C5 | Task 012 did not prove browser logout preserves sync or disconnect stops it. | YES / YES, using public real-sync seam rather than a private fake | Applied: start through public `spawn_remote_sync(ShareConfig)`; assert handle remains `Some` after browser logout and becomes `None` after disconnect. |
| C6 | Task 003 allegedly configured SQLite `busy_timeout` on only one pooled connection and accepted an arbitrary loser error. | NO | Dismissed. Peer verified sqlx-sqlite 0.8.6 defaults every established connection to a 5-second busy timeout (`options/mod.rs` and `connection/establish.rs`). Persisted-state race proof retained; misleading comment corrected to belt-and-braces wording. |
| C7 | Same alleged per-connection timeout defect for task 004 handoff claim. | NO | Dismissed for the same toolchain evidence; persisted claimed state plus replay remains the load-bearing proof. |
| C8 | Task 021 did not list `decisions-ledger.md` despite requiring close evidence there. | NO | Dismissed. Peer opened `task-gate.sh`: `docs/plans/$TOPIC/*` is explicitly exempted from the ordinary file set, and append-only ledger writes have a dedicated gate. Framework convention retained. |

Codex score: **5 validated issues + 5 validated fixes = 10**.

## Claude submission and Codex judgment

| id | submitted issue and remediation | peer ruling | final disposition |
|---|---|---|---|
| O1 | Task 006 passed literal access-token strings into a callback that calls production `extract_expiration`. Mint a real future JWT and align profile matching. | issue YES, proposed tuple-return fix NO | Applied corrected fix: keep `mock_hive_oauth(...) -> Uuid`; the token argument is a stable label mapped deterministically to one exact JWT used by redeem, profile matching, observation, and task 018. |
| O2 | Unauthorized login had no reachable initiation/completion route because the existing dialog lives inside the protected tree and polls protected `/api/auth/status`. | YES / YES | Applied wholly in task 016 without changing the existing protected dialog: public init, popup, public auth-state polling, bounded cleanup, then protected mount. |
| O3 | Required `execution_process_id` would break mixed-version Hive/node deployment; proposed optional unscoped fallback. | NO | Dismissed because the fallback violates frozen D7 exact resource scoping. Task 013 remains one coordinated required contract update. |
| O4 | Task 013 was too broad and should become three tasks. | NO | Dismissed. Peer judged it one coherent cross-layer security/compatibility slice; split would create red intermediate contracts and violate the fixed 21-task plan. |
| O5 | Task 006 listed only five of fourteen anonymous `/api/events` builders. Raw grep count proposed as guard. | issue YES, raw-count fix NO | Applied structural edit list for all 14 builders at lines 118, 196, 269, 336, 381, 461, 574, 666, 770, 797, 838, 850, 883, 895; comments/raw match counts are not the oracle. |
| O6 | Task 012's post-logout 401 used the cookie jar emptied by `Max-Age=0`, not the server-revoked token. | YES / YES | Applied raw-token capture, pre-logout success, fresh-jar replay 401, and hashed-row `revoked_at` assertion. |
| O7 | Task 018 omitted disconnect and its harness could not inspect general headers. | issue YES, proposed sequencing incomplete | Applied with two valid sessions so disconnect reaches the handler after the sentinel browser logs out; scan body, all headers, cookies, and logs; assert disconnect is not a 401. |
| O8 | Task 015 used WebSocket probing against SSE `/api/events`. | YES / YES | Applied `sse_probe`, status 200 and `text/event-stream`; no `ws_probe("/api/events")` remains. |
| O9 | Generated Done-when footers and task commands pointed at a missing `~/.claude/wai/scripts` tree; task 021 also pinned a Codex cache. | issue YES, proposed versioned/cache fix NO | Envelope commands resolve and validate `$HOME/.agents/wai`. The WAI 0.30.0 submitter was corrected locally to render the same stable unversioned path; strict resubmission produced 21 stable footers and zero obsolete footers. |
| O10 | Task 005's exact baseline included `mod handoff;` from task 004 but did not depend on 004. | YES / YES | Applied dependency `005 -> 004`; task status now shows `001,003,004`. |
| O11 | Task 016 cited the wrong `App.tsx` range. | behavioral issue NO, citation correction YES | Applied exact `function App()` range L254–274 and JSX wrapper L256–272. |
| O12 | Task 017's tests were comments rather than executable interactions. | issue YES, proposed harness incomplete | Applied complete minimal Navbar and SwarmSettings harnesses, menu/click interactions, true/false confirmation cases, API exclusivity, and controlled reload using existing `fireEvent`. |

Claude score: **9 validated issues + 4 validated fixes = 13**.

## OpenCode submission and Claude judgment

| id | submitted issue and remediation | peer ruling | final disposition |
|---|---|---|---|
| F1 | Same incomplete fourteen-site `/api/events` census. | YES / YES | Applied once via task 006 structural census. |
| F2 | Same literal-token/JWT mismatch. | YES / YES with deterministic/memoized identity requirement | Applied once via exact label-to-JWT contract and harness self-test using production expiration extraction. |
| F3 | Task 011 moved `extract_expiration` into `complete_browser_login` but `BrowserLoginError` had no `TokenClaimsError` conversion. | YES / YES | Applied `InvalidToken(#[from] utils::jwt::TokenClaimsError)` with sanitized display and existing generic 400 handler; never OwnerMismatch. |
| F4 | Same missing task-gate path; proposed plugin-cache hardcode. | NO as a plan-specific issue / fix NO | Judge disagreement recorded: Claude classified it as tooling/setup, while Codex independently validated the copy/paste defect. Final deliverable uses stable `$HOME/.agents/wai`, never a versioned cache. |
| F5 | Task 016 App anchor was stale. | YES / YES | Applied L254–274 correction. |
| F6 | Claimed systematic line drift in tasks 012–014. | only logout anchor YES / proposed blanket fix NO | Corrected only `async fn logout` to `oauth.rs:168-189`; verified task 014 loader starts 272/754/866 and other cited regions were already correct. |
| F7 | Task 018 sentinel helper lacked an `app_code` parameter, allegedly making mismatch vacuous. | NO | Dismissed: first login uses `code-a`, sentinel flow `code-1`, and one-shot init ordering keeps mocks distinct. |

OpenCode score: **5 validated issues + 4 validated fixes = 9**.

## Independently verified supplemental findings

The invalid partial Claude seat is not scored, but two concrete compile-contract defects it exposed were independently checked and fixed under the no-deferred-remediation rule:

1. Task 007 now defines test token helpers from the repository's `connection_token.rs` pattern, uses `EncodingKey::from_base64_secret` rather than `from_secret`, and serializes proxy UUID claims as strings.
2. Task 007 now unwraps both `node_runner_context(): Option<&NodeRunnerContext>` and async `node_id(): Option<Uuid>` explicitly and fail-closed, after returning early for a valid browser session.

The independent restart-port review (`/tmp/opencode/review-codex-r4.md`) was rechecked and needed no new edit: task 006 already retains and awaits the old server `JoinHandle`, records generation completion, permits OS port reuse, and forbids address-inequality as evidence.

## Focused recheck and closure

After remediation the envelope was strict-submitted again through WAI 0.30.0 rather than hand-editing generated tasks.

Observed checks:

```text
PLAN-LINT PASS: local-node-browser-oauth — plan/frontmatter consistent, verification + SC-coverage complete
tasks=21 stable_footers=21 obsolete_footers=0
topic=local-node-browser-oauth  total=21  passed=0  ready=21  in-progress=0  blocked=0  rejected=0
```

Focused inspection also confirmed: task 005 depends on 004; task 006 contains all 14 event builders, exact JWT/profile identity, all-header capture, and priority-1 signalled responders; task 007 contains the base64 and nested-option contracts; task 011 owns `InvalidToken`; task 012 contains raw-token replay plus real sync-handle state; task 015 names seven deterministic tests and uses SSE; task 016 has working public login; task 017 has executable interactions; task 018 enumerates real Storage entries and exercises disconnect.

Plan lint still emits only the acknowledged rotating sibling advisories and the non-fatal unavailable live-SQL-schema advisory. Their evidence and disposition are recorded in `decisions-ledger.md`.

Tournament termination rule is satisfied: every peer-validated or independently verified real finding is remediated, every rejected finding has cited evidence, and focused strict resubmission/recheck passes. No implementation began.
