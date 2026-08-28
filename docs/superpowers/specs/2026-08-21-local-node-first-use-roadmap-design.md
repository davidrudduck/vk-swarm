---
doc_type: spec
status: active
workstream: local-node-first-use
change_kind: behaviour
verify_cmd: "bash scripts/verify-local-node-first-use.sh"
---

# Local Node First Use — Implementation Roadmap

## Intent
The refactor program has delivered substantial internal capability, but the operator still cannot
depend on the system for daily work. The next release therefore optimizes for a usable product loop,
not for completion of the full orchestration architecture.

The first release must let one operator:

1. open a node from a trusted LAN browser;
2. authenticate through Hive OAuth;
3. create or select a local project;
4. create a task;
5. execute it through a pre-existing OpenCode server;
6. observe truthful state and complete output;
7. reload the complete persisted result; and
8. cancel or retry without hidden loss or a second writer.

After that release, direct OpenCode, Claude, and automatic crash continuation are added through the
same execution contract. Hive management, accessibility, and phases P5–P7 then continue while the
system is already producing value.


## User stories
- **US1:** As the single node operator, I can authenticate one trusted-LAN browser through Hive and keep using the local node when Hive is temporarily unavailable.
- **US2:** As the operator, I can run a task through my pre-existing OpenCode server in the task worktree and trust its state, output, cancellation, and retry behavior.
- **US3:** As the operator, an accidental node restart fails safely without an orphaned remote session, automatic replacement, or second writer.
- **US4:** As a maintainer, I can add direct OpenCode, Claude, and automatic continuation by extending one durable execution contract rather than replacing first-release lifecycle machinery.
- **US5:** As the program owner, I can release useful milestones independently without closing a workstream while required runtime evidence or remediation remains outstanding.

## Success criteria
SC1: Hive OAuth authorizes only the initiating browser; a second browser remains unauthorized; the persistent local session survives idle, browser restart, and planned idle node restart; transient Hive loss preserves established local access; explicit Hive disconnect invalidates all browser sessions.
→ US1
SC2: Creating and starting a local task produces exactly one attached OpenCode writer in the generated task worktree, with a pre-created and durably persisted remote session identity before the prompt can begin work; the OpenCode server default project remains unchanged.
→ US2
SC3: Starting, running, terminal, protocol-failure, live output, final output, and reloaded history are truthful and complete; history/live handoff, persistence flush, and migration lose and duplicate zero messages.
→ US2
SC4: Cancellation reports success only after both the local attached client and remote OpenCode session are verified quiescent; failed verification stays visible and blocks a second writer.
→ US2
SC5: A retry preserves prior history and the retry draft until replacement startup succeeds; injected replacement failure leaves both available and reports the failure.
→ US2
SC6: Before automatic continuation is certified, startup blocks execution mutations until stale work is reconciled, aborts or fences every known writer, starts no replacement, retains durable history, and leaves one interrupted execution with explicit retry.
→ US3
SC7: M2 introduces the permanent execution-generation and ownership identity used unchanged by attached OpenCode, direct OpenCode, Claude, and M6 recovery claims; later milestones require no replacement lifecycle schema.
→ US4
SC8: The umbrella remains a roadmap while each child passes independently through PRD, design, precheck, decomposition, execution, deterministic gates, live evidence, integrated review, and same-milestone remediation; M0-M3 alone gate the first release.
→ US5

## Users
- **Primary user:** the single operator who runs a node, reaches it from a trusted-LAN browser, and
  delegates local repository work to coding agents.
- **Operators of the first release:** people using a pre-existing authenticated OpenCode server who
  need truthful launch, output, cancellation, retry, and restart behavior before trusting daily work
  to the system.
- **Maintainers:** contributors implementing later executors and orchestration phases against the
  same browser-authorization and execution-lifecycle contracts.


## Constraints
- One operator and one Hive owner identity; no multi-user authorization model.
- Trusted-LAN HTTP deployment with the explicitly accepted plaintext-session risk.
- Hive OAuth is the only login mechanism. There is no local password or secondary identity system.
- Planned node restarts require active work to finish until M6 certifies automatic continuation.
- M1–M3 must reuse durable contracts that later executors and crash continuation can extend without
  replacing first-release identity, ownership, or persistence models.
- Tests, live evidence, operator documentation, review, and remediation are part of each milestone.


## Out of scope
The first release does not certify direct OpenCode CLI, Claude CLI, or automatic crash continuation.
It also does not include Hive project-management parity, remote node-local project creation or
deletion, the node accessibility pass, P5 scheduling, P6 management-agent behavior, P7 external
control, or the optional P8 adapter. These remain ordered later milestones, not hidden requirements
for the first usable local loop.


## Approach
Deliver a local-operability release train instead of waiting for the full orchestration refactor. M0 establishes authoritative WAI tracking. M1 browser authorization and M2 shared lifecycle correctness settle independent contracts, then M3 certifies the pre-existing OpenCode server and gates first release. M4 direct OpenCode, M5 Claude, and M6 automatic continuation extend those contracts after the system is already useful. M7 resumes Hive usability, accessibility, and P5-P7. Each milestone is a separate WAI child workstream and closes only with runtime behavior, deterministic tests, live evidence, documentation, integrated review, and same-milestone remediation complete.


## Design
The roadmap responds to confirmed gaps in the current production paths:

- `crates/server/src/routes/tasks/handlers/core.rs` reports Create & Start success after executor
  startup failure.
- `crates/services/src/services/container.rs` suppresses stop failures and contains the non-atomic
  history/live and recovery wiring paths.
- `crates/utils/src/msg_store.rs` broadcasts before recording history and subscribes after taking a
  history snapshot.
- `crates/local-deployment/src/container.rs` migrates historical logs before the normalizer and
  batcher have completed.
- `crates/executors/src/executors/opencode.rs` lacks typed attached-server configuration, dynamic
  `--dir`, remote abort, and the complete current OpenCode event contract.
- `crates/executors/src/executors/claude.rs` imports global hooks and omits production context in
  follow-ups.
- `crates/executors/src/executors/claude/protocol.rs` consumes the final result before normal raw and
  normalized forwarding and maps abnormal outcomes through success.
- `crates/server/src/routes/oauth.rs` authenticates the node daemon to Hive. It does not establish
  per-browser authorization for the node API.
- `crates/server/src/routes/mod.rs` exposes the node API without a global browser-session boundary.

The original north-star remains
[`2026-06-25-vk-swarm-refactor.md`](./2026-06-25-vk-swarm-refactor.md). This roadmap changes delivery
order, not the final product intent.

### 3. Product and security boundary

#### 3.1 Supported operator model

- One operator.
- A trusted LAN deployment.
- Hive is the only identity provider.
- The node records one stable Hive owner subject and accepts browser authorization only for that
  subject.
- No local account database, password, multi-user role model, or RBAC.

#### 3.2 Browser authorization

The existing Hive OAuth handoff will be extended to authorize each browser separately:

1. A browser starts OAuth through the node.
2. The node creates a random, one-time handoff correlation bound to that browser. The correlation
   expires after 10 minutes and a completed or expired correlation cannot be replayed.
3. Hive authenticates the operator.
4. The node verifies that the redeemed stable Hive subject matches the node's recorded owner.
5. The node creates a persistent opaque browser session after successful redemption.
6. The session identifies the same single operator represented by the node's Hive identity.

The node stores a hash of the random session token in its local database and sends the opaque token
in a same-origin, HTTP-only, persistent cookie. There is no server-enforced session expiry in this
milestone; the cookie uses a five-year lifetime so browser restart does not end the session. The
cookie does not use the `Secure` attribute because plain HTTP LAN operation is explicitly supported.

The browser must never receive the daemon's Hive access or refresh tokens. Static assets, health,
OAuth initiation/completion, and the minimum unauthenticated auth-state response needed to render
login remain public. The public auth-state response contains no protected node, owner, or Hive token
data. Protected HTTP APIs and WebSockets require the local browser-session cookie. Authorization is
deny-by-default outside the named public routes.

Browser logout ends only that browser session. Disconnecting the node from Hive remains a separate
operation and invalidates all browser sessions. An owner identity change also invalidates them.

The first release has no inactivity timeout or routine absolute session expiry. The node's
short-lived Hive access token continues to refresh server-side without ending the browser session.
After a browser session is established, authorization is checked locally and a temporary Hive outage
does not log out the browser or block the local execution loop. New browser authentication requires
Hive to be reachable. A transport outage or transient token-refresh failure is not a Hive
disconnection and does not invalidate browser sessions; only the operator's explicit disconnect
operation or a confirmed different owner identity does so.

#### 3.3 Explicitly accepted deployment constraint

The first release does not require or enforce TLS, the cookie `Secure` attribute, a CORS policy,
Origin enforcement, local credentials, RBAC, or multi-user authorization. The one-time handoff state
is part of the selected OAuth flow, not an additional identity mechanism. Documentation must state
that plaintext LAN traffic can expose authenticated sessions. The trusted-LAN boundary is an
explicit operator assumption, not a claim that the transport is secure.

### 4. Delivery model

The work is a release train of independently tracked milestones:

```text
M0 Goal and tracker reconciliation
       |
       +-----------------+
       v                 v
M1 Browser OAuth   M2 Shared execution correctness
       +-----------------+
                 |
                 v
M3 Attached OpenCode server
                 |
                 v
         FIRST RELEASE GATE
                 |
                 v
M4 Direct OpenCode CLI
                 |
                 v
M5 Claude CLI
                 |
                 v
M6 Automatic crash/resume
                 |
                 v
M7 Hive usability, accessibility, P5–P7
```

M1 and M2 may be investigated and decomposed in parallel. Implementation must be serialized where
they modify the same router or lifecycle code. M3 consumes M2's execution contract. M4–M6 do not
delay first use.

Each milestone can contain small PRs, but it is not complete until its outcome-level acceptance
criteria pass. Tests, live evidence, operator documentation, and review remediation are part of the
milestone—not follow-up work.

### 5. M0 — Goal and tracker reconciliation

#### Scope

Track this document as the umbrella workstream `local-node-first-use`. It is a roadmap and release
gate, not one monolithic `/wai:decompose` target. Create each child through `/wai:prd-new` only when
its intent is ready to settle, so every tracker has a canonical spec rather than an empty placeholder:

1. `local-node-browser-oauth`
2. `local-execution-lifecycle-correctness`
3. `opencode-attached-server`
4. `opencode-cli-first-use`
5. `claude-cli-first-use`
6. `local-execution-crash-resume`

Each child then passes independently through `/wai:spec`, `/wai:precheck`, `/wai:decompose`, and
`/wai:execute`. M1 and M2 are the first child specs. M3 is decomposed against their settled browser
authorization and execution-lifecycle contracts. The umbrella closes only when the declared release
train is complete; a child closes only when its own outcome-level acceptance criteria pass.

Update the `vk-swarm-refactor` tracker to show P3 and P4 shipped and pause P5–P7 behind local
usability. Correct stale `/wai:next` state. Promote relevant findings, consolidate contradictory
normalizer trackers under the confirmed product defect, and give every first-release blocker one
owner.

#### Acceptance criteria

1. `/wai:next` lists only genuinely unfinished ready work.
2. The umbrella roadmap contains the first-release dependency order and gate.
3. Every first-release blocker has exactly one owner.
4. No M1–M3 workstream can close with live or E2E verification outstanding.
5. Every created child tracker points to one canonical child spec and no umbrella-wide implementation
   plan attempts to decompose M1–M6 as one unit.

### 6. M1 — Per-browser Hive OAuth

#### Scope

- Extend OAuth handoff state with a one-time, browser-bound correlation that expires after 10 minutes
  and rejects replay.
- Create a persistent opaque local session after successful Hive redemption.
- Store only a session hash or identifier server-side.
- Bind sessions to the stable Hive owner subject and reject a different redeemed subject.
- Apply session authorization to protected `/api/*` routes and WebSockets.
- Keep static assets, health, and OAuth initiation/completion available before login.
- Separate browser logout from node/Hive disconnection.
- Use no inactivity timeout or routine absolute expiry.
- Invalidate sessions on explicit browser logout, Hive disconnection, or owner identity change.
- Keep established browser sessions and local operation available during transient Hive network or
  token-refresh failures.
- Add no TLS, CORS, RBAC, origin-policy, secondary-password, or multi-user requirement.

#### Acceptance criteria

1. A fresh browser can load login but receives `401` from protected APIs and cannot open protected
   WebSockets.
2. Successful Hive OAuth authorizes only the initiating browser.
3. A second clean browser remains unauthorized until it completes OAuth.
4. The authenticated Hive subject matches the node's single-operator identity.
5. Authorization survives reload, browser restart, idle time, and a planned idle node restart.
6. Server-side Hive token refresh does not expire the browser session.
7. Browser logout affects only that browser and does not stop Hive synchronization.
8. Hive disconnection invalidates all browser sessions.
9. Hive tokens never appear in browser storage, URLs, UI logs, or API responses.
10. The flow works over the existing trusted-LAN HTTP deployment.
11. Handoff correlation expires after 10 minutes, is single-use, is bound to its initiating browser,
    and cannot authorize another browser when copied or replayed.
12. A transient Hive outage preserves established browser authorization and local project/task/
    execution operations; new browser OAuth remains unavailable until Hive returns.
13. A transient outage or token-refresh failure does not take the explicit Hive-disconnect path.

### 7. M2 — Shared execution lifecycle correctness

#### 7.1 Launch state

Use explicit `starting -> running -> terminal` transitions. M2 introduces the permanent durable
execution-generation identity used by M3–M6: each launch has an immutable generation, execution ID,
owner server instance, canonical worktree, and adapter process/session identities. M6 adds automatic
claim and continuation to this same model; it does not replace it with a second recovery identity.

Do not mark an execution running before required ownership and process identity are durable. PID or
ownership persistence failure must terminate the child and fail launch. Initial runs, retries, and
follow-ups receive the real `SpawnContext`.

#### 7.2 Create and start

If task creation succeeds but executor start fails:

- retain the task as a usable Draft;
- remove or terminally classify the broken attempt/process;
- return structured `start_failed` information with the task identity;
- show the failure and retry action in the UI; and
- never present the operation as successfully started.

#### 7.3 Cancellation

Propagate query, stop, timeout, and verification failures. Report success only after process
quiescence is confirmed. Leave unresolved execution visibly `stopping` or `stop_failed`.

#### 7.4 Retry

Stop the previous execution, but preserve history and the retry draft while the replacement starts.
Hide superseded history and clear the draft only after successful replacement startup. Failed
replacement startup retains both.

#### 7.5 Output durability

- Make `MsgStore` history-plus-live subscription atomic.
- Add acknowledged batcher shutdown.
- Await raw persistence and normalization.
- Migrate historical logs only after both are durable.
- Make partial migration resumable and idempotent.
- Persist final and trailing output before completion is announced.

#### 7.6 Pre-M6 restart policy

First release does not promise automatic continuation. After accidental restart:

1. reconcile stale work before accepting execution mutations;
2. fence or abort known stale activity;
3. launch no automatic replacement;
4. retain durable history;
5. mark the execution interrupted; and
6. expose explicit retry.

#### Acceptance criteria

1. Failed Create & Start is visible and leaves no false running attempt.
2. Successful Create & Start produces exactly one writer.
3. Cancel returns success only after verified quiescence.
4. Injected stop failure remains visible and does not report success.
5. Failed retry startup preserves history and draft.
6. History/live boundary stress loses and duplicates zero messages.
7. Fast and slow executions retain first, final, and trailing entries.
8. Reloaded history matches durable execution output.
9. Partial migration resumes without gaps or duplicates.
10. Protocol or transport failure cannot become successful completion.
11. Startup reconciliation cannot race a new execution mutation.
12. Accidental restart cannot create a second writer.
13. The generation/ownership schema used by M3–M5 is the same schema M6 later claims and resumes;
    automatic continuation requires no replacement identity migration.

### 8. M3 — Pre-existing OpenCode server

#### 8.1 Typed configuration

Replace free-form attachment arguments with explicit fields:

- connection mode `attached_server`;
- server URL;
- optional username;
- password environment-variable name, defaulting to `OPENCODE_SERVER_PASSWORD`;
- model;
- agent; and
- supported non-sensitive arguments.

Reject credentials in URLs and reject free-form overrides of attachment, directory, session,
credential, or lifecycle arguments. Persist only the environment-variable name; read its value in
the child environment. Passwords must not be stored in profile JSON or passed on the command line.

#### 8.2 Availability and compatibility

Before a profile is usable, resolve the local OpenCode executable and verify server reachability,
authentication, required session/run/abort behavior, and model/agent availability where discovery is
supported. Return actionable failure classes for missing executable, unreachable server,
unauthorized connection, incompatible server, unavailable model, and invalid configuration.

The first release certifies the audited OpenCode `1.18.19` client/server contract. The executable and
server must report the certified version before the profile becomes usable. A different version is
reported as unverified and blocked until its command/API behavior, event fixtures, and real golden
path are added to the certified compatibility set. Do not silently assume protocol compatibility.

#### 8.3 Worktree and identity

Every initial run, retry, and follow-up uses a durable two-stage remote handshake:

1. create the OpenCode server session for the canonical execution worktree without sending the task
   prompt;
2. persist the returned remote session ID with the execution generation, server identity, worktree,
   and owner;
3. only after that commit, submit the prompt to the known session and attach the local event client;
4. persist the local attached-client PID and remaining process identity before `running`.

The OpenCode server API supports session creation, prompting an existing session, and
`POST /session/{id}/abort`. The adapter may use the API directly or launch the compatible client
against the pre-created session, but it must not discover the session ID from output after agent work
has already begun. If the compatible client is used, it dynamically receives the known session and
worktree, equivalent to:

```text
opencode run --attach <server-url> --session <persisted-session-id> --dir <execution-worktree> ...
```

If the node fails after session creation but before prompt submission, reconciliation can delete or
abandon the known idle session. If it fails after prompt acceptance, reconciliation can abort the
known session. Do not transition to `running` until required local and remote identity is durable.

#### 8.4 Event contract

Model current OpenCode `text`, `tool_use`, `reasoning`, `error`, `step_start`, and `step_finish`
events. Structured errors influence terminal state. Unknown events remain available in raw logs.
Malformed events produce diagnostics and cannot silently become success. Fixtures come from the
pinned supported OpenCode version.

#### 8.5 Cancel and fence

Cancellation must identify the remote session, request abort, terminate the local attached client,
verify local process-group exit, verify remote-session quiescence, and only then return success.
Failure to verify quiescence leaves `stop_failed` visible and blocks another writer in that
worktree. Startup reconciliation uses the same operation.

#### 8.6 Restart before M6

On unexpected restart, complete reconciliation before mutations, abort/fence the remote session,
terminate discoverable local clients, verify no writer remains, retain durable output, mark the run
interrupted, and offer explicit retry. Do not resume automatically.

#### 8.7 Operator experience and documentation

Distinguish attached OpenCode from direct CLI. Show actionable configuration errors and truthful
`starting`, `running`, `stopping`, `stop_failed`, `interrupted`, and terminal states. Document the
supported version, server startup, credential environment, profile setup, diagnosis, worktree
behavior, cancellation, and restart limitation.

#### Acceptance criteria

1. Free-form parameters cannot override attachment, worktree, session, or credentials.
2. Credentials are absent from profiles, process arguments, logs, diagnostics, and responses.
3. Invalid, unavailable, unauthorized, or incompatible configuration prevents launch.
4. Every execution uses its generated task worktree.
5. The OpenCode server's default project remains unchanged.
6. Initial runs, retries, and follow-ups preserve worktree and execution context.
7. Session and ownership identity are durable before `running`.
8. Current text, tool, reasoning, step, and error events normalize correctly.
9. Structured failure cannot become successful completion.
10. Final output survives immediate completion and reload.
11. Cancel verifies local and remote quiescence.
12. Failed quiescence blocks another writer and remains visible.
13. Accidental restart leaves one interrupted execution and zero continuing or replacement writers.
14. Mock-server integration and real LAN browser golden-path tests pass.
15. A crash after remote-session creation but before prompt submission starts no agent work, and a
    crash after prompt acceptance always leaves a durable session ID that startup can abort.
16. Only explicitly certified OpenCode client/server versions can launch.

### 9. Exact first-release gate

M0–M3 gate first release. M4–M6 do not.

#### 9.1 Scope state

- M0, M1, M2, and M3 are complete.
- No M1–M3 acceptance criterion is deferred.
- Direct OpenCode, Claude, and automatic continuation are identified as uncertified later milestones.
- No open P0/P1 finding affects authorization, launch, ownership, output durability, terminal state,
  cancellation, retry, attached OpenCode isolation, or restart fail-safe behavior.
- No actionable finding of any severity remains when its remediation is required to satisfy the
  supported journey or an M1–M3 acceptance criterion.

#### 9.2 Required authentication behavior

1. Protected HTTP APIs and WebSockets reject an unauthenticated browser.
2. Hive OAuth authorizes only the initiating browser.
3. A second browser remains unauthorized.
4. Authorization survives idle, reload, browser restart, and planned idle node restart.
5. Browser logout affects only that browser.
6. Hive disconnect invalidates browser sessions.
7. Hive tokens never enter browser-visible storage or responses.
8. A transient Hive outage leaves established browser sessions and the local execution loop usable.
9. OAuth handoff state is browser-bound, expires after 10 minutes, and rejects replay.

#### 9.3 Required execution behavior

1. Startup failure is visible and leaves no false running process.
2. Successful start creates exactly one writer.
3. The task runs in its assigned worktree; the OpenCode server default directory is unchanged.
4. Current text, tool, reasoning, error, and final output appear correctly.
5. Reload preserves complete terminal history.
6. Cancel succeeds only after local and remote quiescence.
7. Failed cancel remains visible and blocks another writer.
8. Failed retry preserves history and draft.
9. Structured failure cannot become successful completion.
10. The remote session is created and durably identified before its task prompt can start agent work.
11. The attached client and server match an explicitly certified OpenCode version.

#### 9.4 Required restart fail-safe

1. Startup accepts no execution mutation before reconciliation.
2. The stale attached client is fenced.
3. The remote OpenCode session is aborted.
4. No replacement starts automatically.
5. Exactly one interrupted execution remains.
6. Durable pre-crash history remains visible.
7. Explicit retry starts one new writer.

#### 9.5 Automated gate

All commands pass on the same final commit:

```bash
cargo clippy --all --all-targets --all-features -- -D warnings
cargo test --workspace
cargo build --release --workspace

cd frontend && npm run lint
cd frontend && npx tsc --noEmit
cd frontend && npm run test:run
cd frontend && npm run build
cd frontend && npm run test:e2e

cd remote-frontend && npm run lint
cd remote-frontend && npx tsc --noEmit
cd remote-frontend && npx vitest run
cd remote-frontend && npm run build
```

M3 adds the currently missing local-frontend Playwright setup and `test:e2e` script.

The deterministic suite covers browser-session isolation, API and WebSocket rejection, launch
compensation, verified stop and stop failure, non-destructive retry failure, `MsgStore` concurrency,
acknowledged output flush, resumable migration, attached-server authentication, dynamic `--dir`,
current OpenCode events, remote abort, and accidental-restart no-duplicate behavior.
It also injects failure after remote-session creation and after prompt acceptance to prove that every
possibly active remote writer has a durable abort identity.

#### 9.6 Real LAN acceptance run

Use a clean fixture repository, fresh node database, final release build, real authenticated
pre-existing OpenCode server, and browser reaching the node through its non-loopback LAN address.

The run must prove:

1. an unauthenticated browser cannot access protected data or execution WebSockets;
2. Hive OAuth authorizes that browser while a second browser remains unauthorized;
3. the operator creates or selects a local project;
4. the operator creates and starts a task with attached OpenCode;
5. a known sentinel change occurs only in the task worktree;
6. the server default directory remains unchanged;
7. running state and live text/tool activity appear;
8. the execution reaches the correct terminal state;
9. reload preserves complete history, including final output;
10. a long-running execution can be cancelled;
11. the remote session is aborted and produces no further output or filesystem activity;
12. injected replacement-start failure preserves retry history and draft;
13. planned idle restart preserves browser authorization and history; and
14. restart during execution produces one interrupted execution, no continuing remote session, and
    no replacement writer;
15. temporarily making Hive unreachable after login does not end the browser session or block local
    project, task, and execution operations; and
16. the accepted OpenCode client and server versions match the certified compatibility entry.

#### 9.7 Review and evidence

Archive the release commit and build identity, redacted authorization evidence, OpenCode version and
configuration shape, process/session identifiers without credentials, worktree isolation evidence,
history completeness evidence, cancellation/quiescence evidence, Playwright report, and full gate
output.

Run the required integrated adversarial review over M1–M3. Fix every real finding in the same release
train. Document false positives with cited repository evidence. Rerun automated and live gates after
the final fix. Any failed criterion blocks release.

The exact release declaration is:

> Certified for one operator using Hive OAuth from a trusted LAN, with local projects and a
> pre-existing authenticated OpenCode server. Active execution must finish before planned node
> restart. Accidental restart fails safely without automatic continuation or duplicate writers.

### 10. M4 — Direct OpenCode CLI

#### 10.1 Installation and availability

Require the exact globally resolvable OpenCode executable that runtime launches. Verify its supported
version, authentication, model, and agent. Correct one-off `npx` documentation and disable profiles
that cannot launch.

#### 10.2 Command construction

The command builder owns canonical worktree, prompt transport, model, agent, JSON event format,
approval mode, continuation session, and execution identity. Profiles cannot override worktree,
format, session, or lifecycle identity. Every run and follow-up receives real `SpawnContext`.

#### 10.3 Launch handshake

Persist launch intent and generation, spawn with inspectable execution identity, persist PID and
ownership, attach persistence workers, capture and persist session ID, and confirm the process is
alive before `running`. Failure at any step terminates and reaps the child. Until M6, crash uses the
interrupted fail-safe.

#### 10.4 Output and outcome

Reuse M3's OpenCode event model. OS exit and structured error jointly determine terminal state; zero
exit cannot override structured failure. Use M2's acknowledged persistence and migration order.

#### 10.5 Cancellation

Signal the process group, await bounded graceful exit, escalate by exact process identity when
required, verify no matching execution process remains, and return failure if quiescence is unknown.

#### Acceptance criteria

1. Documentation installs the executable runtime launches.
2. Missing executable, authentication, model, or agent prevents use.
3. OpenCode starts in the assigned worktree.
4. Initial and follow-up context contains correct identifiers.
5. PID or session persistence failure cannot leave an unowned writer.
6. Direct and attached modes share one normalized event contract.
7. Structured errors cannot become success.
8. Final output survives completion and reload.
9. Cancel confirms no matching process remains.
10. Create, observe, reload, cancel, retry, and injected-failure scenarios pass.
11. Accidental restart leaves one interrupted execution and no replacement writer.
12. A direct-CLI browser golden path passes before certification.

### 11. M5 — Claude CLI

#### 11.1 Hooks policy

Do not merge global `~/.claude/settings.json` hooks into certified initialization. Send `hooks: null`
or omit hooks according to the proven protocol. Forwarding user hooks remains unsupported until a
pinned real-process test proves it safe. Convert the confirmed global-hooks A/B case into a
regression test.

#### 11.2 Protocol order

Parse each message, forward raw output, normalize and persist it, update durable session metadata,
and only then process completion. Persist the final `result` before terminating the protocol reader.

#### 11.3 Outcome classification

Map successful result to completed; result error and decode error to failed; unexpected EOF and
transport loss to failed or interrupted; cancellation to cancelled; timeout to timed out. Success
cannot represent an abnormal protocol outcome. Commit and next-action behavior use classified
outcome rather than OS exit alone.

#### 11.4 Context, persistence, and cancellation

Initial runs, retries, and follow-ups receive real task, attempt, process, worktree, project, and
session context. Remove production nil-UUID fallbacks. Use the full M2 persistence lifecycle. Verify
process-group quiescence before cancel succeeds. Before M6, restart interrupts rather than resumes.

#### 11.5 Verification seam

Add a deterministic fake-Claude executable for normal result, result error, malformed protocol,
unexpected EOF, delayed and fast output, session identity, hooks, and cancellation. Run one
authenticated live browser journey.

#### Acceptance criteria

1. Global hooks cannot stall initialization.
2. Final result appears in raw, live, and reloaded history.
3. Result error, decode error, EOF, and transport failure cannot become success.
4. Failed execution cannot trigger success-only downstream behavior.
5. Initial, retry, and follow-up execution contains real context.
6. No production execution receives nil task context.
7. Cancel succeeds only after quiescence.
8. Fast and slow execution retains complete output.
9. Accidental restart creates no replacement or duplicate writer.
10. Deterministic protocol tests and one authenticated browser golden path pass.

### 12. M6 — Automatic crash and resume

#### 12.1 Continue the durable launch generation

Reuse the durable execution-generation and ownership schema introduced in M2 and exercised by
M3–M5. M6 adds atomic recovery claims and automatic continuation; it must not introduce a parallel
execution identity or require migrating first-release runs to a replacement lifecycle model.

Persist launch intent, spawn an identifiable but inactive child or supervisor, persist
PID/generation/ownership, then activate agent work. Persist the continuation key immediately when
available and mark the execution recoverable only afterward.

If crash occurs before the key is durable, discover and fence the child, do not guess or reuse an
older session, mark the generation interrupted, and launch no concurrent replacement.

#### 12.2 Recovery readiness barrier

At startup, expose health as `recovering`, reject execution mutations, enumerate stale non-terminal
generations, discover and fence old writers, atomically claim recoverable work, restore the complete
runtime pipeline, resume or safely interrupt each execution, and only then accept new mutations.

#### 12.3 Exactly-one-writer fencing

Use execution ID, generation, process identity marker, server-instance ownership, worktree, executor
session, and remote session identity—not PID alone. Claim each generation once. Ignore stale or
delayed completion from older generations.

#### 12.4 Equivalent recovered pipeline

Recovered execution receives the same message store, raw persistence, normalizer, batcher, session
persistence, spawn context, cancellation handles, outcome classification, migration, and lifecycle
events as a fresh run.

#### 12.5 Adapter recovery

- Attached OpenCode: abort old activity, verify quiescence, resume the durable session in its original
  worktree, and establish new attached-client ownership.
- Direct OpenCode: fence every matching old process and resume only the execution's durable session.
- Claude: fence the old generation, resume through its durable continuation identity, apply certified
  hooks/protocol policy, and preserve spawn context.

#### 12.6 Log continuity and operator control

Persist a normalization high-water mark. Retain pre-crash history, suppress replay duplicates without
dropping new output, append resumed output to the same conversation, and persist final result before
terminal state. Show recovering, resumed, recovery-failed, manual retry, and abandon/cancel states.

#### 12.7 Fault-injection matrix

Inject crash before spawn; after spawn before PID; after PID before activation; after activation
before session persistence; after session persistence; during raw and normalized output; during
cancel; during recovery claim; after replacement spawn before ownership; and across two consecutive
restarts. Exercise QA mock and each certified executor's nearest real seam.

#### Acceptance criteria

1. Each execution has at most one active generation.
2. Agent work cannot begin before durable ownership.
3. Persistence failure terminates an unactivated child.
4. Mutations remain unavailable until recovery completes.
5. Ownership transfers atomically to the new server instance.
6. Every old writer is fenced before resume.
7. Pre-session crash fails safely without guessing continuation.
8. Recoverable crash resumes the correct session and worktree.
9. Recovered execution has complete fresh-run logging and completion wiring.
10. Replay creates no gaps or duplicates.
11. Old-generation completion cannot overwrite current state.
12. Cancel works during and after recovery.
13. Two repeated restarts still produce at most one writer.
14. Attached OpenCode, direct OpenCode, and Claude each pass real kill/restart/resume.
15. Final history contains pre-crash output, resumed output, final result, and truthful status.
16. Integrated adversarial review has no unresolved recovery defect.

### 13. M7 — Resume product and refactor expansion

After the system is producing value, proceed in this order:

1. Hive browser token longevity.
2. Hive project-management parity and two-sided lifecycle consistency.
3. Remote node-local project creation/deletion design.
4. Node semantic-color and interaction-state accessibility.
5. P5 scheduling foundation.
6. P6 management agent and P7 external control in parallel.
7. Optional P8 adapter.

### 14. Definition of done

A milestone is complete only when its runtime behavior, deterministic tests, live evidence,
documentation, mandatory repository gates, integrated review, and review remediation are complete on
the same committed state. An in-scope defect discovered during implementation or review is fixed in
that milestone. A workstream cannot close by moving a necessary part of its stated outcome into a
follow-up tracker.


## Decisions
D1 (reversible): **Delivery objective** — choose **Local usable loop first**. The operator explicitly prioritized a usable local product before further refactor completion.

D2 (reversible): **Browser authorization** — choose **Per-browser Hive OAuth session**. Hive remains the only identity provider while browser authorization becomes explicit.

D3 (reversible): **Browser-session lifetime** — choose **Persistent session without routine server expiry**. The first release favors generous persistence and explicit invalidation.

D4 (reversible): **WAI artifact shape** — choose **Umbrella roadmap plus child workstreams**. The release train needs independently shippable, reviewable milestones.

D5 (reversible): **Execution identity ownership** — choose **Permanent generation model in M2**. The first usable release must not create lifecycle follow-up debt.

D6 (reversible): **Attached OpenCode launch handshake** — choose **Pre-create and persist remote session before prompt**. OpenCode exposes create, prompt, and abort session operations, enabling a two-stage durable handshake.

D7 (reversible): **OpenCode compatibility** — choose **Explicit certified versions**. OpenCode evolves quickly and current event drift is already confirmed.

D8 (reversible): **Restart behavior before M6** — choose **Safe interruption without automatic continuation**. The operator accepts finish-before-planned-restart, but accidental restart must fail safely.

D9 (reversible): **Attached-server credentials** — choose **Environment-variable name only**. The attached-server adapter needs authentication without expanding secret exposure.

D10 (reversible): **Trusted-LAN transport controls** — choose **Do not enforce TLS, Secure cookie, CORS, or Origin policy**. The operator explicitly accepted trusted-LAN plaintext operation for this release.

D11 (reversible): **Executor certification order** — choose **Attached OpenCode, direct OpenCode, Claude**. The operator supplied this priority order.


## Test strategy
TS1: Browser-authorization integration and Playwright tests cover unauthenticated HTTP/WebSocket rejection, initiating-browser isolation, replay rejection, persistence, logout, explicit disconnect, and transient Hive outage.
TS2: Shared lifecycle tests inject launch-persistence, stop, retry, history/live, batch flush, migration, protocol, and startup-reconciliation failures and assert truthful state with at most one writer.
TS3: An authenticated OpenCode mock server covers version probing, session create-before-prompt, directory binding, current event types, abort/quiescence, and crashes at every remote-session handoff boundary.
TS4: A real non-loopback LAN browser journey against the final release build and authenticated OpenCode 1.18.19 server proves worktree isolation, complete output/reload, cancellation, retry preservation, outage continuity, and restart fail-safe behavior.
TS5: Repository clippy, workspace tests, release build, both frontend lint/type/test/build suites, and local Playwright pass on the same final commit.
TS6: Integrated adversarial review covers the combined M1-M3 diff; every real finding is fixed before gates and live acceptance are rerun.
