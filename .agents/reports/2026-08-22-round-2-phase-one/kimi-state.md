# Integrated phase-1 adversarial review — kimi-state (owner/handoff/session/epoch/remote-sync state machines)

- **Workstream:** `local-node-browser-oauth`, phase 1 (tasks 001–005 + corrective 022)
- **Range:** `41f55c4b..ae5ee15f` (24 commits)
- **Reviewer lens:** reconstruct owner, handoff, session, deployment-epoch and remote-sync state
  machines; attack cross-task transitions, races, replay, invalidation, durability. Read-only.
- **Environment note:** host `/tmp` is quota-limited; all cargo runs used a repo-local `TMPDIR`
  (since removed). No source or git state was modified; `git status` shows only the two
  pre-existing untracked review-tmp directories.

## Independent verification (rerun, not trusted from the ledger)

| Gate | Command | Result |
|---|---|---|
| DB browser_auth | `cargo test -p db browser_auth` | 17/17 pass (owner 4, handoff 7, session 5, migration 1) |
| Server seams | `cargo test -p server --lib auth::seams` | 5/5 pass |
| Epoch clone-sharing | `cargo test -p local-deployment --lib browser_auth_epoch_is_shared_by_deployment_clones` | pass |
| Startup sync install | `cargo test -p local-deployment --lib configured_startup_sync_is_installed_before_constructor_returns` | pass |
| Raw API-base compat | `cargo test -p local-deployment --lib raw_api_base_remains_available_when_share_sync_config_is_unavailable` | pass |
| Full local-deployment lib | `cargo test -p local-deployment --lib` | 43/43 pass (no regression from the `from_parts` refactor) |
| Format | `cargo fmt --all -- --check` | pass |
| Clippy | `cargo clippy -p db -p deployment -p local-deployment -p server --all-targets --all-features -- -D warnings` | exit 0 (verified via direct exit code, not a masked pipeline) |

## Reconstructed state machines and attack results

### Owner (`crates/db/src/models/browser_auth/owner.rs`)

`pin_or_verify_owner` is one `INSERT ... ON CONFLICT(slot) DO UPDATE SET hive_user_id =
hive_user_id ... RETURNING` statement (owner.rs:26-35). The no-op `DO UPDATE` makes RETURNING
fire on the conflict path, so two concurrent first-pins cannot both win and the incumbent's
`pinned_at` is not rewritten (proven by `first_subject_pins_and_same_subject_does_not_move_pinned_at`
and `different_subject_is_rejected_without_side_effects`). Mismatch is side-effect free — no
write, no revocation — matching SC6 and the task-003 contract. The singleton is structural:
`slot INTEGER PRIMARY KEY CHECK (slot = 1)` (migration L17-21); the migration test proves slot 2
is rejected (test_utils.rs:183-191).

**Attack — concurrent first pin loser gets SQLITE_BUSY and flakes the test:** disproved.
`create_test_pool` uses `min_connections(1)` (test_utils.rs:101), so the PRAGMA-primed connection
survives; whichever future draws it waits out the lock and wins. The assertion tolerates a loser
error and the persisted-state read is the real proof (owner.rs:104-113). Passed on my run.

### Handoff (`crates/db/src/models/browser_auth/handoff.rs`)

`claim_handoff` is a single conditional `UPDATE ... WHERE handoff_id=? AND state='pending' AND
expires_at > ? AND binding_hash=? RETURNING` (handoff.rs:67-80). Wrong-browser, expired, replayed
and unknown-id attempts match no row and consume nothing (tests at handoff.rs:121-193). TTL is
exactly 600_000 ms, computed only inside `create_handoff` (handoff.rs:7, 34) so call sites cannot
drift; the strict `expires_at > now` boundary matches the Hive-side `HANDOFF_TTL: i64 = 10` at
`crates/remote/src/auth/handoff.rs:32` (citation verified). `invalidate_pending_handoffs`
(handoff.rs:89-95) reuses the terminal `claimed` state — no third state, no schema change — and
touches neither `node_owner` nor `browser_sessions` (test handoff.rs:256-279).

**Attack — claim vs invalidate interleaving:** both are single-statement UPDATEs; SQLite
serializes writers, so exactly one wins per row. Combined with the planned epoch choreography
(tasks 010/012, see below) a pre-disconnect claim either commits before disconnect or fails its
epoch re-check; a post-disconnect claim matches nothing because the row is durably `claimed`.
The invalidation survives restart (it is a committed UPDATE, not in-memory state), which is what
makes the in-memory epoch sufficient — the ledger's reasoning holds.

### Session (`crates/db/src/models/browser_auth/session.rs`)

`authenticate_session` has deliberately no time argument and no expiry predicate (session.rs:41-52)
— revocation-state only, so a Hive outage cannot deauthorize (D6/D9/SC5); the migration test
asserts no `expires_at` column exists (test_utils.rs:193-206). `token_hash` is UNIQUE
(migration L42; test session.rs:180-192), so one presented token can never resolve to two rows.
`revoke_session` is scoped to the presenting hash and idempotent without rewriting `revoked_at`
(session.rs:57-71; tests session.rs:110-150). `revoke_all_sessions` rewrites only live rows
(session.rs:79-87). SC7 isolation proven by `revoke_session_is_scoped_to_the_presenting_browser`.

**Attack — session minted concurrently with revoke-all survives:** true at the model layer by
design; closure is the epoch fence (task 011 commit section vs task 012 disconnect, both planned
under the same guard). This is the round-1 finding and is exactly what 022 + amended 009–012
address; phase 1 ships only the primitives, which is the sanctioned split.

### Deployment epoch and remote-sync install (tasks 022)

`browser_auth_epoch` is one `Arc<Mutex<u64>>` per deployment (local-deployment/src/lib.rs:58,
236, 453, 729-731); the clone-sharing test proves clones see the same cell. `install_remote_sync`
(deployment/src/lib.rs:125-134) check-and-sets the sync slot under `share_sync_handle`'s mutex, so
it cannot double-install against the legacy detached `spawn_remote_sync`
(deployment/src/lib.rs:107-122) which takes the same mutex. `from_parts` now awaits
`install_remote_sync` before returning (local-deployment/src/lib.rs:493-495), and the server only
serves after `DeploymentImpl::new()` returns (server/src/main.rs:124), so any disconnect handler
that can run observes the installed handle. The deterministic `current_thread` test
(lib.rs:1316-1354) catches a regression to detached install.

**Attack — `RemoteSync::spawn`'s `.expect("failed to create remote client")`
(services/src/services/share.rs:161-162) now panics inside `from_parts` instead of a detached
task:** disproved as a reachable defect. The startup path is gated on `share_config` parsed +
`share_publisher` Ok + credentials present (lib.rs:238-244); `ShareConfig.api_base` is an
already-parsed `url::Url` (services/src/services/share/config.rs:8-13, 26-37), and
`RemoteClient::new` can only fail on `Url::parse` of an already-valid URL string or on reqwest
client construction (services/src/services/remote_client.rs:203-216). Practically unreachable, and
the previous detached task would have died on the same expect anyway.

**Attack — the from_parts refactor changed legacy remote-client behavior:** disproved. Production
`new()` resolves the raw `VK_SHARED_API_BASE` (env then `option_env!`) and `ShareConfig::from_env`
and injects both (lib.rs:657-674); remote-client construction still keys off the raw string
(lib.rs:194-209), so a base `RemoteClient` accepts but `ShareConfig` cannot derive a WebSocket URL
from (e.g. `ftp://…`) still configures the client — proven by
`raw_api_base_remains_available_when_share_sync_config_is_unavailable`. Every external constructor
caller (`server/src/main.rs:124`, `server/tests/common/mod.rs:106,152`, two test-only route
helpers) goes through `new()`; `from_parts`/`for_test` are `pub(crate)`. `for_test` now passes
`None`/`None`, which *removes* a test-isolation hole (a developer's real env could previously
spawn sync from tests).

### Lock-order audit across the locked 009–012 choreography

Login commit (011): epoch guard → `refresh_guard` → save/mint/install. Disconnect (012): epoch
guard → bump → invalidate → revoke-all → stop sync → `refresh_guard` only around credential clear,
taken *after* `client.logout()` to avoid re-entrant deadlock. In-flight token refresh holds
`refresh_guard` and never takes the epoch. One consistent order (epoch → refresh_guard), no cycle.
Claim + epoch capture share one short guard (010) with all Hive I/O outside it. This closes the
round-1 `revoke_all_rows=1 live_after_disconnect=['after']` race at the plan level; route-level
closure is tasks 009–012, explicitly recorded as outstanding in the ledger (task-022 verification
item 4) — SC8 is not claimed complete in phase 1.

### Fidelity

- Tasks 001–005 and 022 frontmatter all `status: passed`; 001's irreversible gate has the approval
  token (`reviews/001.approved`) predating the migration.
- Public signatures match the task contracts verbatim (003/004/005 docs vs owner.rs/handoff.rs/
  session.rs).
- `Cargo.lock` diff is exactly `+ "base64"` in the `server` package dependency list — no
  resolution drift.
- Migration `20260821000000_add_browser_auth.sql` is the highest-versioned migration, additive
  only, all timestamps caller-bound INTEGER epoch millis (dodging the event_journal TEXT-collation
  regression it cites).
- The pre-existing orphan-cleanup hazard discovered by 022's panel is split into the tracked
  workstream `dev-docs/workstreams/local-deployment-test-orphan-cleanup-safety/README.md`, per
  AGENTS.md's no-silent-carry-forward rule. The two new direct `from_parts` tests call the shared
  `disable_orphan_cleanup_for_tests()` guard before construction (lib.rs:1318, 1363).

## Findings

No `[BLOCKING]` or `[SHOULD-FIX]` findings.

### [INFO-1] Stale SAFETY comment references `ShareConfig::from_env` inside `from_parts`

`crates/local-deployment/src/lib.rs:516-518` says sibling tests "may be inside `from_parts`
calling `ShareConfig::from_env`", but 022 moved that resolution into `new()` (lib.rs:658);
`from_parts` now receives it injected. The comment's safety conclusion is unaffected (env reads
still exist in `from_parts` via `NodeRunnerConfig::from_env`/`database_path()`), but the citation
is stale. Minimal remediation: s/calling `ShareConfig::from_env`, `NodeRunnerConfig::from_env`/calling
`NodeRunnerConfig::from_env`/ when the file is next touched. Docs-only; does not affect behavior.

### [INFO-2] Task-022 plan snippets would not compile as written

`docs/plans/local-node-browser-oauth/phase-1/022-...md:28,42` show `let pool =
create_test_pool().await;` but `create_test_pool` returns `(SqlitePool, TempDir)`; the landed
tests correctly destructure (handoff.rs:229, 261). The TempDir must also be bound (not `_`-dropped
inline) or the database file vanishes mid-test — the implementation gets this right
(`let (pool, _t) = …`). Plan/implementation drift, docs-only; the gate transcripts confirm the
compiled tests are what ran.

### [INFO-3] Legacy OAuth callback retains the check-then-detached-install TOCTOU (accepted residual)

`crates/server/src/routes/oauth.rs:144-151` checks the sync slot is empty, drops the guard, then
calls detached `spawn_remote_sync`; `logout` (oauth.rs:168-173) can observe the empty slot and
return before the detached task installs, leaving sync running after credential clear (it then
fails auth and logs, harmlessly). This is pre-existing behavior, deliberately unchanged by 022
("Do not change `spawn_remote_sync`; the legacy OAuth route retains its current behavior until
task 011", 022 doc L158-159) and recorded in the ledger. Restated here only so the phase-2 gate
cannot lose it: task 011 must move this path to the fenced synchronous `install_remote_sync`.

### [INFO-4] `spawn_remote_sync` overwrites the slot without re-checking

`crates/deployment/src/lib.rs:107-122` installs unconditionally inside the detached task; losing a
race overwrites the incumbent handle. Worst case is the dropped incumbent's task being aborted by
`RemoteSyncHandleInner::drop` (services/src/services/share.rs:682-689) — one live sync remains, no
leak. Pre-existing, unchanged by this range; the new `install_remote_sync` (lib.rs:125-134) checks
`is_none()` under the lock and is race-free.

## Disproved suspicions (summary)

1. Double-install between `install_remote_sync` and `spawn_remote_sync` — same mutex, check under
   lock. deployment/src/lib.rs:115-121 vs 125-134.
2. Startup install landing after disconnect's slot observation — `from_parts` awaits install
   before `Ok(deployment)` (local-deployment/src/lib.rs:493-495); serve starts only after `new()`.
3. Epoch not shared across clones — `Arc` cloned into the struct (lib.rs:453); test proves it.
4. `expect` panic propagation on the startup path — unreachable: parsed `Url` input, gated by
   `share_publisher` Ok.
5. Concurrent-claim/pin test flakiness via per-connection PRAGMA — `min_connections(1)` keeps the
   primed connection; assertions tolerate loser error; passed on rerun.
6. Cross-crate TTL drift — Hive `HANDOFF_TTL = 10` minutes verified at
   crates/remote/src/auth/handoff.rs:32; local constant computed in exactly one place.
7. Legacy client-boundary regression from the `from_parts` refactor — raw-base-driven construction
   preserved and pinned by the `ftp://` regression test; all external callers use `new()`.
8. Session/handoff non-interference of invalidation — `handoff_invalidation_does_not_touch_owner_or_sessions`
   passes; SQL touches only `browser_oauth_handoffs`.
9. Lockfile drift — diff is the single `base64` line in the `server` package.
10. Schema drift — migration additive, highest-versioned, CHECK-pinned singleton, no session
    expiry column, all asserted by the migration test.

## Residual risk (accepted, documented, not phase-1 defects)

- O8 crash window between SQLite revoke-all and file/Keychain credential clear (round-1 report;
  ledger). Recovery is disconnect retry; revoke-first ordering never leaves an authorized browser.
- Route-level epoch/invalidation wiring (SC8 end-to-end) is tasks 009–012 in phases 2–3, as the
  ledger explicitly records.
- `new_for_drain_test` orphan-cleanup exposure is tracked in
  `dev-docs/workstreams/local-deployment-test-orphan-cleanup-safety/`.

VERDICT: APPROVE
