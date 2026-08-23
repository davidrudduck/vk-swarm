# Integrated phase-1 adversarial review — SQL/schema/models lens (GLM)

- Range: `41f55c4b..ae5ee15f` (read-only panel worktree `dr-panel-gentle-mongoose-ae5ee15f-116053`)
- Focus: owner singleton, session validation/revocation, handoff claim/invalidate atomicity, SQLite
  constraints/concurrency/time semantics, credential/session non-interference, cross-task
  interaction tracing, task 022 fidelity.
- Method: full source read of every changed production file, byte-fidelity comparison against all
  six task contracts, live re-run of every phase-1 focused suite, independent SQLite
  cross-connection concurrency probes (WAL, per-connection busy timeout — the same C-API semantics
  sqlx uses), and lock-order/linearization tracing across amended tasks 009–012.

## Verdict summary

The phase-1 implementation conforms. One process gap (an unfulfilled tracking commitment made by a
report committed inside this range) must be closed in-session; it requires a one-file workstream
artifact, not a code change. Verdict at end: **APPROVE**.

## Evidence: focused suites re-run (all green)

- `cargo test -p db browser_auth` — 17/17 passed (owner 4, handoff 7, session 5, migration 1).
- `cargo test -p server auth::seams` — 5/5 passed.
- `cargo test -p local-deployment browser_auth_epoch_is_shared_by_deployment_clones` — passed.
- `cargo test -p local-deployment configured_startup_sync_is_installed_before_constructor_returns` — passed.
- `cargo test -p local-deployment raw_api_base_remains_available_when_share_sync_config_is_unavailable` — passed.

## Evidence: independent concurrency probes

Rebuilt the migration schema in raw SQLite (WAL, per-connection 5s busy timeout, two independent
connections — mirroring sqlx 0.8.6's per-connection `sqlite3_busy_timeout`, see
`sqlx-sqlite-0.8.6/src/options/mod.rs:201` and `connection/establish.rs:282-285`):

```text
pin-race: returned=2/2 persisted-winner-correct=True pinned_at=100
claim-race: wins=1 (expect 1) final=claimed
claim-vs-invalidate: [('claim', True), ('invalidate', 0)] final=claimed
revoke-all-vs-create: [('revoke_all', 1), ('create', 1)] live-after=[('h2',)]
```

- Cross-connection concurrent `pin_or_verify_owner`: both statements RETURN, exactly one persisted
  winner, `pinned_at` never moves. The `DO UPDATE SET hive_user_id = hive_user_id` no-op upsert is
  sound (crates/db/src/models/browser_auth/owner.rs:26-41).
- Cross-connection concurrent `claim_handoff`: exactly one winner (handoff.rs:67-80).
- `claim_handoff` vs `invalidate_pending_handoffs` (handoff.rs:89-95): single UPDATE statements,
  serialized by SQLite's single-writer lock; terminal state guaranteed in both orders; no order
  produces a claimable-after-disconnect row or a lost claim.
- `revoke_all_sessions` vs `create_session`: the model-layer gap (a session inserted after
  revoke-all is live) reproduces — this is the known SC8 race validated in round 1
  (`.agents/reports/2026-08-22-round-1-cross-model-phase-one.md:22-46`), correctly NOT attempted at
  model level and closed at route level by the task-022 epoch + amended 011/012 wiring.

## Cross-task interaction trace (primitives → amended wiring)

The committed primitives plus the amended (in-range) phase-2/3 task texts form a closed fence:

1. **Initiation** (009 amendment): epoch guard held only around the durable `create_handoff`
   insert — the linearization point. Insert before disconnect ⇒ invalidated durably; after ⇒
   legitimate fresh login at the new epoch.
2. **Claim** (010 amendment): epoch capture and `claim_handoff` inside one short guard, dropped
   before any Hive I/O. Disconnect cannot fit between capture and claim.
3. **Commit** (011 amendment): re-check `epoch_at_claim` under the epoch guard, then — while still
   holding it — `refresh_guard` → save credentials → create session → synchronous
   `install_remote_sync`. The only login path allowed to mint.
4. **Disconnect** (012 amendment): epoch guard held across increment → `invalidate_pending_handoffs`
   → `revoke_all_sessions` → sync shutdown → credential clear, with `refresh_guard` taken only
   around clear and only after `client.logout` (avoids re-entrant tokio-mutex deadlock).

Lock order is consistent everywhere: `browser_auth_epoch` → (`refresh_guard`, `share_sync_handle`);
no path takes them in the reverse order; no cycle exists. A callback that claimed at epoch N either
commits before disconnect (and is then fully revoked by it) or fails its re-check; a callback can
never claim a pre-disconnect handoff (durable invalidation) nor commit a post-disconnect state with
a stale epoch. Restart resets the in-memory epoch to 0, which is safe: claimed handoffs are
terminal (replay cannot mint), invalidated handoffs are durably terminal, and a fresh initiation
captures the fresh epoch. The model layer (revoke/insert independence) plus this route-level fence
is the only composition that can satisfy SC8 without a new migration; round 1 already rejected the
durable-generation alternative for exceeding task 001's irreversible approval, correctly.

## Fidelity walk (all six tasks)

| Task | Contract verified | Notes |
|---|---|---|
| 001 | Migration SQL byte-identical to contract incl. header comments; additive-only; `20260821000000` is the strictly highest version; CHECK pins `slot=1` and `state IN ('pending','claimed')`; no session expiry column; INTEGER millis, no SQL time defaults; test uses `create_test_pool()` (no hand-written DDL); `001.approved` token exists | Anchor deviation (appended after `test_template_reuse`, not `test_create_test_pool`) documented in ledger |
| 002 | `seams.rs` interfaces exact; fakes public and unconditionally compiled; `hash_token` encoding identical to `routes/oauth.rs::hash_sha256_hex` (oauth.rs:221-229); lockfile delta is exactly `+base64` on `server` | Lockfile plan correction recorded |
| 003 | One-statement upsert, runtime sqlx only, mismatch side-effect free, `pinned_at` immobile | See F2 (comment) |
| 004 | Single-statement terminal claim, strict `expires_at > now` boundary, TTL computed inside `create_handoff`, raw `app_verifier`, `HANDOFF_TTL_MILLIS=600_000` matches Hive `HANDOFF_TTL=10` minutes (crates/remote/src/auth/handoff.rs:33) | — |
| 005 | Revocation-state-only auth (no time, no Hive state), hash-scoped idempotent revoke preserving first timestamp, live-only revoke-all count, UNIQUE hash | `INSERT .. RETURNING` choice recorded in ledger |
| 022 | `invalidate_pending_handoffs` byte-faithful; per-deployment `Arc<Mutex<u64>>` epoch (not process-global); synchronous `install_remote_sync` awaited inside `from_parts` before return (local-deployment/src/lib.rs:494); legacy `spawn_remote_sync` untouched and still used only by the legacy route (server/src/routes/oauth.rs:151); raw `api_base` still drives `RemoteClient` independent of `ShareConfig` (regression test proves); orphan-cleanup guard extracted and invoked by all direct `from_parts` tests; no schema change, no third state, no deletions, no route/credential edits — every STOP trigger honored | — |

`for_test()` now passes `StartupRemoteConfig { None, None }`: previously it inherited env-derived
`ShareConfig`/`VK_SHARED_API_BASE` read inside `from_parts`. This is a strict test-isolation
improvement; production `new()` resolves both from env exactly as before (local-deployment
lib.rs:654-670). No production caller of `from_parts` exists besides `new()` and tests.

## Findings

### F1 — [SHOULD-FIX] Unfulfilled tracking commitment for the pre-existing SQLITE_BUSY_SNAPSHOT calibration flake

- **Citation:** `.agents/reports/2026-08-22-round-1-cross-model-phase-one.md:74-80` (committed
  in-range at `08dc9f9b`): "Per AGENTS.md this will be resolved as the explicit tracked scope
  split `sqlite-busy-snapshot-calibration-stability` before the session closes; it is not silently
  carried forward."
- **Disproof of closure:** no `dev-docs/workstreams/sqlite-busy-snapshot-calibration-stability/`
  exists; `dev-docs/BACKLOG.md` has no row; the decisions-ledger and `dev-docs/MASTER.md` contain
  zero mentions. The sibling split promised and created the same day
  (`local-deployment-test-orphan-cleanup-safety`) does exist, so the mechanism was available and
  simply not executed for this one.
- **Live impact (reproduced today, 3 consecutive runs of `cargo test -p db --lib control_`):**
  `queries.rs:1369 control_read_then_write_shape_reproduces_busy_snapshot` failed 3/3;
  `lifecycle.rs:1110 control_prior_status_read_reproduces_busy_snapshot` failed 2/3 — each failure
  is "0/200 SQLITE_BUSY_SNAPSHOT … calibration control must reproduce at least one
  SQLITE_BUSY_SNAPSHOT". `cargo test -p db` is therefore intermittently-to-consistently red,
  hitting the mandatory gate.
- **Not a phase-1 code defect:** `git diff --stat 41f55c4b..ae5ee15f -- crates/db/src/models/execution_process/`
  is empty; round 1 observed the same oscillation before task 022 landed; the new migration cannot
  plausibly affect WAL write-contention timing in those self-contained pools (one extra
  `CREATE TABLE IF NOT EXISTS` at setup).
- **Why it still matters:** AGENTS.md (No Deferred Remediation / pre-existing debt) permits only
  fix-now, tracked split, or escalation — in-session. The committed report chose "tracked split"
  and the artifact does not exist. Approving phase 1 without closing this would silently carry the
  debt the report explicitly promised not to carry.
- **Minimal remediation (no code change):** create
  `dev-docs/workstreams/sqlite-busy-snapshot-calibration-stability/README.md` (frontmatter
  mirroring `local-deployment-test-orphan-cleanup-safety/README.md`) naming both controls, the
  0/200 reproduction failure mode, and the stabilization requirement; append a decisions-ledger
  entry referencing it. Alternatively stabilize/relax the two controls in-session with per-item
  rationale, or escalate to the user.

### F2 — [INFO] Factually incorrect comment about `create_test_pool()` busy timeout

- **Citation:** `crates/db/src/models/browser_auth/owner.rs:94-101` (and the same copied rationale
  at `handoff.rs:196-201`): "create_test_pool() sets NO busy_timeout … without this the loser gets
  an immediate SQLITE_BUSY and the outcome assertion becomes a coin-flip."
- **Disproof:** sqlx-sqlite 0.8.6 installs a default 5-second busy timeout on every connection —
  `sqlx-sqlite-0.8.6/src/options/mod.rs:201` (`busy_timeout: Duration::from_secs(5)`), applied via
  `sqlite3_busy_timeout` at `connection/establish.rs:282-285`. `create_test_pool()` builds options
  with `SqliteConnectOptions::from_str(..)` (which starts from those defaults) and only overrides
  `journal_mode` (crates/db/src/test_utils.rs:91-93). The task's own "SQLite timeout
  clarification" (task 003, line 165: "sqlx 0.8.6 already installs a 5-second busy timeout on
  every SQLite connection, including create_test_pool(). The explicit PRAGMA … is belt-and-braces
  only") and ledger review-time decision 3 both state the truth; the misleading variant was copied
  verbatim from the task's test block.
- **Impact:** documentation only — the explicit `PRAGMA busy_timeout = 5000` is a harmless no-op
  and every test passes. Risk is a future maintainer mis-diagnosing SQLITE_BUSY behavior or
  "fixing" pool configuration based on the false claim.
- **Remediation:** comment-only edit replacing the rationale with the task's line-165
  clarification (both files), or simply deleting the incorrect sentence.

### F3 — [INFO] Unbounded retention of terminal handoff rows and revoked session rows

- **Citation:** migration comment block (crates/db/migrations/20260821000000_add_browser_auth.sql:22-35)
  and STOP triggers of tasks 004/022 (deletion forbidden — terminal state is replay evidence).
- **Impact:** every OAuth initiation inserts a handoff row that is never removed; every session
  row outlives its revocation. For a single-operator node this is negligible growth and is an
  explicit consequence of the durable-terminal-state design (replay must be observable, SC4).
- **Remediation:** none required in phase 1. A future housekeeping workstream could add
  time-based pruning of pre-epoch terminal rows; that would need its own (reversible) task.

## Suspictions raised and disproved (no finding)

1. *"Concurrent first-pin could double-pin or move `pinned_at`"* — disproved by probe (exactly one
   persisted winner, `pinned_at` immobile) and by the passing in-repo test.
2. *"Claim vs invalidate could interleave to a claimable post-disconnect handoff"* — disproved by
   probe; both are single UPDATE statements serialized by SQLite's writer lock.
3. *"Epoch reset on restart reopens the SC8 race"* — disproved: pre-disconnect handoffs are
   durably `claimed` by `invalidate_pending_handoffs`; claimed rows are terminal by CHECK-bounded
   state machine; an in-flight callback dies with the process and its handoff is already terminal.
4. *"Startup sync could still install after disconnect observed an empty slot"* — disproved:
   `from_parts` awaits `install_remote_sync` before returning (local-deployment/src/lib.rs:491-494),
   and the current-thread constructor test observes the handle synchronously.
5. *"The from_parts refactor changes legacy remote-client configuration"* — disproved:
   `raw_api_base_remains_available_when_share_sync_config_is_unavailable` proves a parseable raw
   base still configures `RemoteClient` when `ShareConfig` is absent; production `new()` reads the
   same two env sources as before.
6. *"Taking refresh_guard in disconnect deadlocks with client.logout"* — the amended 012 takes it
   only after `client.logout` and only around clear; login commit takes epoch → refresh in the
   same order; no cycle.
7. *"Handoff TTL might diverge from Hive's"* — `HANDOFF_TTL_MILLIS = 600_000` matches
   `crates/remote/src/auth/handoff.rs:33` (`HANDOFF_TTL: i64 = 10; // minutes`).
8. *"Timestamp collation could fail open like event_journal"* — all browser-auth timestamps are
   INTEGER millis compared numerically (`expires_at > ?`), never TEXT; the failure mode is
   structurally absent.
9. *"The new tables could collide with existing names or gain a second writer"* — workspace grep:
   the only readers/writers are the browser_auth models; `sqlite_master` uniqueness holds;
   migration is the sole schema source.
10. *"The migration could break additivity / existing db tests"* — `cargo test -p db` full-suite
    green apart from F1's pre-existing calibration controls, which the range demonstrably does not
    touch.

## Verdict

The phase-1 SQL/schema/model implementation — migration, seams, owner singleton, handoff
create/claim/invalidate, session create/authenticate/revoke, epoch and synchronous-install
primitives — is faithful to every task contract, free of discoverable correctness, concurrency,
time-semantics, and non-interference defects, and composes with the amended 009–012 wiring into a
closed disconnect/login fence. F1 is a process gap committed inside the range (a promised tracked
workstream that was never created while its sibling was); it must be closed in-session per the
No-Deferred-Remediation rule before the PR is submitted — it requires one documentation artifact,
not a code change. F2/F3 are informational.

VERDICT: APPROVE
