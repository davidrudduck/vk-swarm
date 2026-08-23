# Integrated phase-1 review — grok-4.6

**Seat:** OpenCode / xAI grok-4.6
**Target:** committed range `41f55c4b..ae5ee15f` in
`/home/david/.cache/dr-panel-tmp/dr-panel-gentle-mongoose-ae5ee15f-116053`
(`HEAD` = `ae5ee15f6353f3e00c9e214a2b2b2414ea2b2071`)
**Intent:** `docs/superpowers/specs/2026-08-21-local-node-browser-oauth.md`
(prompt named `*-design.md`; that file is absent — the settled spec above is the
live document), `docs/plans/local-node-browser-oauth/phase-1/*.md`,
`docs/plans/local-node-browser-oauth/decisions-ledger.md`
**Mode:** read-only. No checkout/restore/stash/reset/clean/commit. No source edits.

This pass hunts what isolated task reviews cannot: cross-task state combinations,
auth bypass/replay, restart, clock/expiry, env/config regressions, and API
consumers. Isolated gates and the Stage-2 CONFORMS notes were treated as claims
to falsify, not evidence.

---

## Reconstruction

### Owner
`pin_or_verify_owner` is one `INSERT ... ON CONFLICT(slot) DO UPDATE SET
hive_user_id = hive_user_id RETURNING` (`owner.rs:26-41`). First writer pins.
Same subject is a genuine no-op (`pinned_at` stays). Different subject is
`OwnerMismatch` with no write. There is no owner-clear API. Disconnect is
specified to retain the row (D4/SC8).

### Handoff
`create_handoff` binds `state='pending'` and `expires_at = now + 600_000`
(`handoff.rs:34-49`). `claim_handoff` is one `UPDATE ... WHERE pending AND
expires_at > ? AND binding_hash = ? RETURNING` (`handoff.rs:67-80`). Wrong
browser, expiry, unknown id, and replay all return `None` without consuming a
rightful pending row. `invalidate_pending_handoffs` is one
`UPDATE ... SET state='claimed' WHERE state='pending'` (`handoff.rs:89-95`).
Terminal `claimed` is reused; no third state; no DELETE.

### Session
`create_session` inserts `revoked_at = NULL` (`session.rs:22-32`).
`authenticate_session` is hash + `revoked_at IS NULL` with no time argument
(`session.rs:41-51`). `revoke_session` / `revoke_all_sessions` are idempotent
`AND revoked_at IS NULL` updates (`session.rs:57-86`). No FK to `node_owner`.
No credential or sync writes.

### Epoch
Per-deployment `Arc<Mutex<u64>>` starts at 0 (`local-deployment/src/lib.rs:236`).
`LocalDeployment` is `#[derive(Clone)]` (`lib.rs:43-58`); clones share the Arc.
No process-global static. Not durable. Not yet taken by any route.

### Sync install
`spawn_remote_sync` is unchanged and still detaches (`deployment/src/lib.rs:107-123`).
`install_remote_sync` locks the slot, spawns only if `None`, stores before
return (`deployment/src/lib.rs:125-135`). Configured startup now awaits
`install_remote_sync` before `from_parts` returns (`local-deployment/src/lib.rs:493-495`).
`RemoteSync::spawn` only builds a client and `tokio::spawn`s `run`
(`share.rs:158-178`); it does not block on Hive I/O. `RemoteSyncHandleInner::drop`
sends shutdown and aborts (`share.rs:682-689`).

### Production OAuth (still pre-phase-2)
`oauth.rs` still uses the in-memory `oauth_handoffs` map and detached
`spawn_remote_sync` (`oauth.rs:142-157`). The new tables and epoch are unused
on the live login/logout path. That is the locked TS1 split: 009–012 consume
the primitives. Not a phase-1 defect.

---

## Cross-task combinations checked

| Combination | Result |
|---|---|
| Concurrent `claim_handoff` vs `invalidate_pending_handoffs` | Both are single-statement writers. SQLite serializes. Winner is either `Some` (claim first) or `None` (invalidate first). No half-claimed row. |
| `create_handoff` vs invalidate | Writer serialization. Insert-before-invalidate → durable unclaimable. Invalidate-before-insert → legitimate post-disconnect pending. This is the linearization 009 will pin with the epoch guard. |
| Claim-then-disconnect-then-commit | Invalidate is a no-op on an already-claimed row. The live-process fence is the epoch re-check (011/012), not SQL. Primitives are sufficient: same Arc, increment-then-recheck. |
| `revoke_all_sessions` then later `create_session` | Still possible at the model layer. This is the round-1 finding. 022 does not put them in one transaction (no authorized schema for a generation). Epoch + 011 commit re-check is the designed close. |
| Invalidate vs owner/sessions | Dedicated test (`handoff.rs:255-279`). Owner and live session survive. Matches SC7/SC8 split. |
| Startup install vs disconnect observing empty slot | Closed. `from_parts` awaits install before return (`lib.rs:493-506`). Current-thread test proves the handle is visible without the detached task running (`lib.rs:1317-1358`). |
| `install_remote_sync` vs leftover `spawn_remote_sync` | `install` is if-none; `spawn` still overwrites. Login still uses `spawn` until 011. Pre-existing login-path race, explicitly deferred by 022 STOP / 011 allowed moves. |
| Clone-shared epoch | Test mutates via clone, parent observes (`lib.rs:1304-1314`). Trait accessor returns `&self.browser_auth_epoch`. |
| Restart after disconnect | Epoch resets to 0. Pending rows already `claimed` remain unclaimable. Fresh `create_handoff` is a new PK. In-flight callbacks die with the process. Matches the ledger residual. |
| Restart without disconnect | Pending rows stay pending. Correct: no disconnect happened. |
| Raw API base without `ShareConfig` | `new()` still passes env `api_base` and `ShareConfig::from_env()` separately (`lib.rs:657-672`). `RemoteClient` is built from the raw string (`lib.rs:195-210`). Sync install still requires parsed `ShareConfig` + publisher + credentials (`lib.rs:238-243`). `ftp://example.invalid` test keeps `remote_client().is_ok()` (`lib.rs:1361-1387`). Compared to `41f55c4b` `from_parts`: same split. |
| `for_test()` env isolation | Now injects `None`/`None` instead of reading process env. Intentional (task 022). Existing `for_test` callers in this file are event-bus/compaction tests, not remote-client tests. |
| In-tree `Deployment` consumers | Only `LocalDeployment` implements the trait. Adding `browser_auth_epoch` does not break another impl. |
| Clock / expiry vs Hive | Local claim is `expires_at > now` (`handoff.rs:72`). Hive is `expires_at <= Utc::now()` (`remote/.../handoff.rs:478-479`). Both exclusive at equality. Local TTL `600_000` matches `HANDOFF_TTL = 10` minutes (`handoff.rs:33-145` on the Hive side). |
| `hash_token` vs `hash_sha256_hex` | Same lowercase `{:02x}` SHA-256 loop (`seams.rs:77-85` vs `oauth.rs:221-229`). |
| SQLite constraints | `slot CHECK (slot = 1)`, `state IN ('pending','claimed')`, `token_hash UNIQUE`, no session `expires_at`. Invalidate writes a legal state. Migration is the highest version and additive. `001.approved` is present. |
| Test cleanup | Direct `from_parts` tests call the shared `disable_orphan_cleanup_for_tests` (`lib.rs:509-525`, `1319`, `1363`). Guard writes `DISABLE_WORKTREE_ORPHAN_CLEANUP=1`, which `container.rs:323-325` honors. Pre-existing `new_for_drain_test` exposure is tracked at `dev-docs/workstreams/local-deployment-test-orphan-cleanup-safety/README.md`. |

---

## Fidelity

| Task | Contract | Landed? |
|---|---|---|
| 001 | Exact additive SQL; no session expiry column; structural owner singleton | Yes. `20260821000000_add_browser_auth.sql` matches the task text. Approval token `reviews/001.approved`. |
| 002 | Public `Clock`/`TokenSource`/`hash_token`; fakes not `cfg(test)`; lockfile only adds `base64` to `server` | Yes. `Cargo.lock` diff is one `"base64"` line. |
| 003 | One-statement pin-or-compare; no-op DO UPDATE; runtime SQLx | Yes. |
| 004 | TTL here; one-statement claim; no `Utc::now`; no macros | Yes. |
| 005 | Hash auth; scoped + all revoke; no `expires_at`; no owner/credential writes | Yes. |
| 022 | Invalidate helper + re-export; shared epoch; sync install; inject raw API base vs `ShareConfig`; orphan-cleanup helper; no route/schema/third state | Yes. STOP triggers respected. `oauth.rs` untouched in this range. |

Task 022 closes the *primitive* half of the round-1 disconnect/login race. SC8 is
not complete until 009–012 wire epoch/invalidation/synchronous install. The
ledger already records that (`decisions-ledger.md:199-200`). Reporting the
unwired `oauth.rs` path as a phase-1 defect would relitigate planned phase-2
work.

---

## Findings

No `[BLOCKING]` or `[SHOULD-FIX]` finding survived verification.

### [INFO] I1 — O8 crash window is still real and accepted

`revoke_all_sessions` (`session.rs:79-86`) and credential file/Keychain deletion
cannot share a transaction. A crash between them can leave credentials present
on an over-locked-out node. Recovery is retry-disconnect. The ledger already
accepts this (`decisions-ledger.md:187-192`). Closing it needs a new durable
generation and a second irreversible approval.

### [INFO] I2 — In-memory epoch resets on process start

`browser_auth_epoch` is constructed as `Arc::new(Mutex::new(0u64))`
(`local-deployment/src/lib.rs:236`). Restart coverage is the durable `claimed`
invalidation, not the counter. Sufficient while one process owns the SQLite
file. Documented residual, not a silent hole.

### [INFO] I3 — Detached `spawn_remote_sync` still exists for the live login path

`deployment/src/lib.rs:107-123` and `oauth.rs:142-157` are unchanged. 022 STOP
forbids editing that method or any route. 011's allowed moves replace the login
call site with `install_remote_sync`. Do not treat this leftover as a 022
regression.

### [INFO] I4 — Stale constructor comment

`local-deployment/src/lib.rs:155` still says “the `Deployment` trait is
untouched.” Task 022 added required `browser_auth_epoch`
(`deployment/src/lib.rs:105`). In-tree only `LocalDeployment` implements it, so
this is comment drift, not a consumer break.

### [INFO] I5 — Startup diagnostic still reads process env, not the injected pair

`from_parts` logs `VK_SHARED_API_BASE` via `std::env::var` (`lib.rs:470-489`)
even when `StartupRemoteConfig` injected a base. Production `new()` still
derives both from env, so shipped behavior matches. Injected tests can log
“standalone” while holding a client. Log-only.

---

## Suspicions disproved (not filed)

1. **`install_remote_sync` blocks serving on Hive.** `RemoteSync::spawn`
   (`share.rs:158-178`) parses a URL, builds `reqwest`, and detaches `run`.
   Startup now waits only for that return, not for `sync_unshared_tasks_on_startup`.
2. **Raw API base disabled when `ShareConfig` is `None`.** Current
   `from_parts` builds `RemoteClient` from `api_base` only (`lib.rs:195-210`).
   Pre-022 `from_parts` at `41f55c4b` did the same. `94e5aecc` preserved it.
3. **Clone epoch is a distinct mutex.** Field is `Arc<Mutex<u64>>`;
   `#[derive(Clone)]`; test proves sharing.
4. **Invalidate can un-claim a winner or consume a wrong-browser row.**
   Claim already set `claimed`, so invalidate's `WHERE state='pending'` misses
   it. Wrong-browser never matches `binding_hash`, so it never becomes
   `claimed` via claim; invalidate *does* mark it `claimed`, which is the
   intended disconnect effect, not a wrong-browser consume during a live
   claim attempt.
5. **Hive vs local expiry off-by-one.** Both exclusive at `expires_at == now`.
6. **`hash_token` encoding drift.** Byte-identical loop to `hash_sha256_hex`.
7. **Trait addition breaks another deployment.** Grep of `impl Deployment`
   under `crates/` is only `LocalDeployment`.
8. **Direct constructor tests can sweep real worktrees.** They call the same
   guard `for_test()` uses; `container.rs` honors `"1"` / `"true"`. Remaining
   drain-test hazard is a named workstream, not deferred inside this one.
9. **`create_test_pool` template omits the new migration.** Template is built
   from `sqlx::migrate!("./migrations")` (`test_utils.rs:48-51`). Task 001's
   own test asserts the three tables exist through that helper.

---

## Verdict

Phase-1 primitives (001–005 + corrective 022) match the locked contracts, close
the startup/disconnect install race, preserve the legacy remote-client
boundary, and give 009–012 a sufficient fence (shared epoch + durable
invalidation + synchronous install) for the round-1 SC8 race. Remaining holes
are either accepted residuals (O8, in-memory epoch) or explicitly scheduled
route wiring.

VERDICT: APPROVE
