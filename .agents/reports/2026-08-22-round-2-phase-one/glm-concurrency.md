# Round-2 integrated phase-1 review — GLM (concurrency lens)

- **Range:** `41f55c4b..ae5ee15f` (tasks 001–005 + corrective task 022, worktree `dr-panel-gentle-mongoose-ae5ee15f-116053`)
- **Lens:** mechanics first — clone state, epoch, constructor visibility, sync install/drop/shutdown,
  explicit startup/login/disconnect schedules, API-base compatibility. Fidelity second.
- **Method:** full source read of every changed production file plus the consumers that make the
  primitives meaningful (`crates/server/src/routes/oauth.rs`, `crates/services/src/services/share.rs`,
  `share/config.rs`, `remote_client.rs` refresh path, `auth.rs` refresh guard); focused reruns of the
  claimed gates. Read-only; no source or git state changed (a scratch `TMPDIR` used for cargo was
  removed).

## Verification reruns (all green)

- `cargo test -p db browser_auth` — 17/17 (owner 4, handoff 7 incl. both task-022 invalidation tests,
  session 5, migration 1).
- `cargo test -p local-deployment --lib -- browser_auth_epoch configured_startup raw_api_base` — 3/3.
- `cargo test -p server --lib auth::seams` — 5/5.
- `cargo clippy -p db -p deployment -p local-deployment -p server --all-targets --all-features -- -D warnings` — clean.

## Focus-area reconstruction

### LocalDeployment clone state
`browser_auth_epoch: Arc<Mutex<u64>>` (crates/local-deployment/src/lib.rs:58,236,453) is one `Arc`
created in `from_parts` and cloned into every deployment clone; the trait accessor
(crates/deployment/src/lib.rs:105, local-deployment/src/lib.rs:729-731) hands out `&Arc`, so every
route handler clone shares the same mutex. `browser_auth_epoch_is_shared_by_deployment_clones`
(lib.rs:1304-1314) proves clone-sharing behaviorally. Per-deployment (not process-global) — the task
022 STOP trigger against a global static is respected.

### Browser-auth epoch soundness (schedule counterexamples)
I tried to break the amended protocol (initiation under guard at task 009;
claim+epoch-capture under one guard at task 010; commit re-check under guard at task 011; disconnect
bump+invalidate+revoke+shutdown+clear under guard at task 012) with explicit schedules:

1. **Claim before disconnect:** claim captures epoch 0 → disconnect bumps to 1, invalidates
   (no-op on the already-claimed row), revokes all, clears → commit re-check `1 != 0` → `Disconnected`,
   no session/credentials/sync. ✔
2. **Commit before disconnect:** commit installs sync + session under guard → disconnect (queued on
   the epoch) then revokes that session, takes/shuts the installed handle, clears credentials. Final
   state fully disconnected. ✔
3. **Initiation straddling disconnect:** `create_handoff` is serialized against
   `invalidate_pending_handoffs` by the same epoch mutex (and independently by SQLite's serial
   writers: both are single guarded UPDATE/INSERT statements). Insert-before-invalidate → durably
   `claimed`; insert-after → legitimate post-disconnect login (task 012 test 3 proves it). No
   interleaving leaves a pre-disconnect handoff claimable. ✔
4. **Epoch reset on restart:** requires an HTTP callback to survive process death; a retry re-claims
   a terminal row and fails. The one-process-owns-SQLite assumption is stated
   (decisions-ledger.md:187-189). ✔
5. **Lock ordering:** every guarded path acquires epoch → {refresh_guard, share_sync_handle};
   `require_oauth_token` re-reads credentials *inside* `refresh_guard` before saving
   (remote_client.rs:261-275), so an in-flight refresh either completes its save before the commit
   section or observes cleared credentials and fails without writing. Disconnect takes
   `refresh_guard` only after `client.logout()` (non-reentrant hazard documented, task 012 STOP
   trigger). No cycle exists; the delayed-profile barrier test (task 012 test 1) is the incident
   symptom. ✔

I could not construct a schedule in which a callback linearized before disconnect mutates daemon
state after disconnect returns. The round-1 race
(`revoke_all_rows=1 live_after_disconnect=['after']`) is closed at the primitive level by
`invalidate_pending_handoffs` (handoff.rs:89-95, single guarded UPDATE reusing terminal `claimed`,
no schema change, no third state) + the epoch + synchronous install.

### `spawn_remote_sync` / `install_remote_sync`, handle drop/shutdown
- `spawn_remote_sync` is byte-unchanged (deployment/src/lib.rs:107-123) per the STOP trigger; the
  legacy route call at oauth.rs:151 is the only production caller and is untouched by task 022 —
  the route-level race remains open *by contract* until tasks 011/012 (see residual R1).
- `install_remote_sync` (deployment/src/lib.rs:125-135) holds the slot lock only across synchronous
  `RemoteSync::spawn` (no awaits inside), and installs only `if slot.is_none()` — it can never
  overwrite, so it never silently drops a live handle.
- Overwrite-safety of the legacy detached path re-verified at share.rs:646-691:
  `RemoteSyncHandleInner::drop` sends shutdown and aborts the join handle, so a raced overwrite
  leaves no unreachable live task.
- Startup linearization (cc70f9d7): `from_parts` awaits `install_remote_sync` before returning
  (local-deployment/src/lib.rs:493-495); no server traffic can exist before that, so disconnect can
  always observe the startup handle. The current-thread test (lib.rs:1316-1359) is deterministic:
  on `current_thread` the old detached spawn could not have run before `from_parts` returned, so the
  `is_some()` assertion catches exactly the fixed race. The test's `shutdown().await` join is bounded:
  on an empty migrated DB `migrate_unlinked_projects` and `sync_all_hive_linked_projects`
  short-circuit (share.rs:707-714, 754-765) and the run loop reaches the shutdown arm immediately.

### Constructor visibility and test cleanup
`from_parts` is `pub(crate)` (unchanged), `StartupRemoteConfig` private, tests in-module — no
visibility surface changed. Both direct-constructor tests call the same shared
`disable_orphan_cleanup_for_tests()` Once-guard as `for_test()` (lib.rs:509-525, 1318, 1363), exactly
as the amended task requires; the pre-existing `new_for_drain_test` exposure has a legitimate
tracked split (`dev-docs/workstreams/local-deployment-test-orphan-cleanup-safety/README.md`).
`LocalDeployment` is the workspace's only `Deployment` implementor, so the new required
`browser_auth_epoch()` accessor breaks nothing.

### API-base compatibility
Diffed `from_parts`/`new` against `41f55c4b`: `new()` resolves `VK_SHARED_API_BASE` (env →
`option_env!`) and `ShareConfig::from_env()` exactly as the inline code did and injects both;
`remote_client` remains driven by the **raw** string (lib.rs:195-210) independent of `ShareConfig`
parseability; the `share_sync_config` gating (Some(config) ∧ Ok(publisher) ∧ credentials present)
is untouched; `node_auth_client` env coupling unchanged. `ftp://example.invalid` +
`share_config: None` (lib.rs:1361-1387) pins the legacy boundary — `from_env` genuinely returns
`None` for non-HTTP bases (`derive_ws_url` failure, share/config.rs:38-48) while `RemoteClient::new`
accepts any parseable URL (remote_client.rs:200-206). `for_test()` now passes `None/None`, which is
strictly better isolation than the old in-`from_parts` env read. No regression found.

### Fidelity spot-checks
- Migration byte-identical to the task-001 contract; additive-only; version 20260821000000 is the
  highest (verified `ls`); approval token `reviews/001.approved` predates it; epochs are INTEGER
  caller-bound millis; sessions have no expiry column (structurally asserted, test_utils.rs:195-208).
- `claim_handoff` remains a single-statement conditional UPDATE..RETURNING; wrong-browser/expired/
  replay consume nothing; strict `expires_at > now` boundary proven at exactly TTL
  (handoff.rs:121-149).
- Task 022's five source commits (6eece603, a32804bc, cc70f9d7, 94e5aecc, 53a962b8) touch exactly
  the four declared files; no route edit in the range (oauth.rs absent from the diffstat); no
  migration/STOP violations; both STOP-relevant invalidation tests match the task text verbatim.
- Ledger records the task-022 evidence, the disproved overwrite finding, the startup-race
  remediation, and the compat rationale (decisions-ledger.md:195-240).

## Findings

### F1 [SHOULD-FIX] Round-1 report promises a tracked scope split that does not exist anywhere in the tree
`.agents/reports/2026-08-22-round-1-cross-model-phase-one.md:78-80` states the pre-existing
`SQLITE_BUSY_SNAPSHOT` calibration-control flake "will be resolved as the explicit tracked scope
split `sqlite-busy-snapshot-calibration-stability` before the session closes". At `ae5ee15f`:
`dev-docs/workstreams/` contains no such directory, `dev-docs/BACKLOG.md` has no such row, and
`git log --all --grep=calibration` returns nothing. The only in-tree documentation of the flake risk
is a residual note inside the older event-bus ledger
(`dev-docs/workstreams/vk-swarm-event-bus/plans/vk-swarm-event-bus/decisions-ledger.md:6094-6099`),
which is not a tracked follow-up for this session. Impact: a pre-existing discovered failure is
currently carried forward only as prose inside a report — exactly the "remediation prompt written for
the next session" pattern AGENTS.md prohibits; if the PR is submitted in this state the round-1
promise is silently dropped. Remediation (in-session, ~10 lines): create
`dev-docs/workstreams/sqlite-busy-snapshot-calibration-stability/README.md` (or a `finding-new`
BACKLOG row) citing the event-bus ledger residual and the round-1 reproduction, and reference it
from the local-node-browser-oauth decisions-ledger. Not BLOCKING: the flake predates the range,
touches no file in the diff, and the session is still open (this review is part of it).

### F2 [INFO] `install_remote_sync` puts `RemoteSync::spawn`'s inherent panic on the synchronous startup path
`RemoteSync::spawn` constructs its client with `RemoteClient::new(...).expect(...)`
(crates/services/src/services/share.rs:160-161). The old detached startup path contained that panic
inside a dying spawned task; `install_remote_sync` (crates/deployment/src/lib.rs:125-135, called at
local-deployment/src/lib.rs:493-495) propagates it through `from_parts` → `new()`, aborting server
startup. Practically unreachable: the passed `api_base` is a re-serialized `url::Url`
(round-trip parse cannot fail) and `Client::builder().build()` failure is exotic. Task 022's contract
explicitly dictated calling `RemoteSync::spawn()` as-is, so this is inherited house behavior, not a
task deviation. No action required in this workstream; a future hardening pass could return a
`Result` from the install seam.

### F3 [INFO] Contradictory busy-timeout claims between a test comment and the ledger
`crates/db/src/models/browser_auth/owner.rs:95-97` asserts "create_test_pool() sets NO busy_timeout
... without this the loser gets an immediate SQLITE_BUSY", while the ledger's review-time decision 3
(decisions-ledger.md:21) asserts sqlx-sqlite 0.8.6 applies a default 5-second busy timeout to every
established connection (making the per-test `PRAGMA busy_timeout = 5000` redundant). At most one is
true. Both affected tests tolerate a losing future's error by design, and my reruns were green, so
there is no behavioral risk — but one of the two factual claims is wrong and will mislead the next
concurrency test author. Remediation: correct whichever claim is false (one focused probe against a
raw `create_test_pool()` pool settles it).

### R1 [INFO — verified accepted residual, not a defect] Route-level disconnect/login race still open at `ae5ee15f`
The legacy `handoff_complete` still detached-spawns sync (oauth.rs:143-158) and the legacy `logout`
still has no epoch/invalidation (oauth.rs:168-189), so today's HTTP surface can still reproduce the
round-1 race. This is exactly the contract: task 022's STOP trigger forbids route edits, its
verification item 4 and the ledger (decisions-ledger.md:200, 179-183) record that tasks 009–012 must
wire the primitives before SC8 is complete, and those tasks are amended in-range to do so (verified
against plan.md's ordering note and the 009–012 texts). Reported for completeness so the round-2
record shows it was checked, not missed.

## Verdict

The phase-1 diff is mechanically sound: every scheduled counterexample I constructed against the
epoch/invalidation/refresh-guard/install protocol resolves to a correct final state; the startup
linearization is deterministic and provable on a current-thread runtime; clone state, constructor
visibility, handle drop/shutdown, and raw-API-base compatibility all check out; and all focused
gates rerun green. One process gap (F1) must be closed in-session before submission; the rest are
informational.

VERDICT: APPROVE
