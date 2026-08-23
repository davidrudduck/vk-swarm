# Integrated phase-1 adversarial review — fidelity lens (Kimi)

- **Range:** `41f55c4b..ae5ee15f` (24 commits, tasks 001–005 + corrective 022)
- **Workstream:** `local-node-browser-oauth`
- **Lens (primary):** fidelity to the design spec and every phase-1 task file — STOP triggers,
  exact file scopes, test discrimination, and whether accepted rationale hides unfinished
  phase-1 behavior. Mechanics checked where the lenses overlap.
- **Mode:** read-only. No source or git mutations. SQL semantics were falsified against a scratch
  in-memory sqlite3; the disposable worktree was externally removed late in the review, after all
  substantive evidence was captured (final loose checks re-run against the main checkout, which
  contains the identical commits).

## Method

1. Read every phase-1 task file (001, 002, 003, 004, 005, 022), the decisions-ledger, the round-1
   report, the amended 006/009/010/011/012 plan texts, `plan.md`, and the phase-1 summary.
2. Diffed every source file in the range and compared each against its task contract, including a
   byte-exact diff of the task-001 SQL block against
   `crates/db/migrations/20260821000000_add_browser_auth.sql` (identical).
3. Probed the migration + model SQL in scratch in-memory sqlite3: owner no-op upsert, claim
   expiry boundary, pending-handoff invalidation, revoke timestamp preservation, revoke-all live
   count, and the `CHECK (slot = 1)` singleton.
4. Verified every citation the ledger/report relies on (Hive TTL, share-handle Drop, test-pool
   busy-timeout, `refresh_guard`, hash-encoding sibling, Cargo.lock drift, trait implementors).

## Verified — suspicions actively disproved

- **Migration (001).** Byte-identical to the task contract (diff empty), strictly additive,
  highest-versioned (`ls crates/db/migrations/` tail: `20260821000000_add_browser_auth.sql` >
  `20260812000000_add_event_journal.sql`). Second owner slot rejected:
  `CHECK constraint failed: slot = 1` (scratch probe). `browser_sessions` has no expiry column.
- **Owner pin-or-compare (003).** `ON CONFLICT(slot) DO UPDATE SET hive_user_id = hive_user_id`
  returns the incumbent with `pinned_at` unchanged (probe: conflict insert of `x'bb',200` returned
  `AA|100`). One statement, runtime sqlx form, mismatch side-effect free. Tests match the contract
  verbatim, including the belt-and-braces `PRAGMA busy_timeout` the task explicitly sanctions.
- **Handoff claim (004).** Single-statement claim; strict `expires_at > now` boundary (probe:
  claim at exactly `created_at + 600_000` leaves state `pending`). `HANDOFF_TTL_MILLIS = 600_000`
  matches Hive `HANDOFF_TTL: i64 = 10` minutes at `crates/remote/src/auth/handoff.rs:31` — ledger
  citation `handoff.rs:31-34` is accurate. `hash_token` (`crates/server/src/auth/seams.rs:101-111`)
  is byte-identical in encoding to `hash_sha256_hex` (`crates/server/src/routes/oauth.rs:221-230`):
  same `Sha256::digest` + `{:02x}` loop.
- **Sessions (005).** Probe: second `revoke_session` does not rewrite `revoked_at` (stays 10, not
  99); `revoke_all_sessions` counts only live rows (1 of 2 after one pre-revoked). No `expires_at`
  or elapsed-time predicate anywhere in `session.rs`; no DELETE; `node_owner` untouched.
- **Invalidate (022).** `invalidate_pending_handoffs`
  (`crates/db/src/models/browser_auth/handoff.rs:89-95`) reuses terminal `claimed` — no third
  state, no DELETE, no schema change (STOP triggers respected). Probe: flips all pending rows
  (`changes()=2`), re-run affects 0, owner row and live session survive.
- **Startup linearization (022).** `LocalDeployment::from_parts` awaits
  `deployment.install_remote_sync(sc).await` before returning
  (`crates/local-deployment/src/lib.rs:493-495`); `install_remote_sync`
  (`crates/deployment/src/lib.rs:125-135`) installs synchronously under the slot lock.
  `spawn_remote_sync` (`crates/deployment/src/lib.rs:107-123`) is byte-untouched and the legacy
  OAuth route still calls it (`crates/server/src/routes/oauth.rs:151`). The current-thread
  constructor test genuinely discriminates: against the old detached spawn, the slot is
  observably `None` when `from_parts` returns on a single-thread runtime.
- **Legacy remote-client boundary preserved (022).** `new()` now resolves env-derived
  `api_base`/`ShareConfig` and injects them (`crates/local-deployment/src/lib.rs:657-676`);
  `from_parts` builds `remote_client` from the raw `api_base` exactly as before
  (`lib.rs:195-210`). `raw_api_base_remains_available_when_share_sync_config_is_unavailable`
  injects `ftp://example.invalid` with no `ShareConfig` and asserts `remote_client().is_ok()` —
  the pre-task behavior (raw value accepted independent of sync-config parseability) is preserved
  and pinned by test.
- **Epoch (022).** Per-deployment `Arc<Mutex<u64>>` (`local-deployment/src/lib.rs:236,453,729-731`),
  not a process-global static (STOP trigger respected); clone-sharing proven by test. Sole
  `Deployment` implementor is `LocalDeployment` (grep `impl Deployment for` — one hit), so the new
  required trait method breaks nothing else. `refresh_guard` exists at
  `crates/services/src/services/auth.rs:51` (amended task 012's reference is grounded).
  `RemoteSyncHandleInner::drop` does send shutdown + abort (ledger citation
  `crates/services/src/services/share.rs:682-689` accurate). `create_test_pool()` sets no
  `busy_timeout` (owner.rs:95-97 citation accurate).
- **File scopes.** Every task's diff touches exactly its declared `files:` (001: migration +
  test_utils; 002: Cargo.lock, auth/mod.rs, seams.rs, lib.rs, Cargo.toml; 003: mod.rs×2 + owner.rs;
  004/005: model + mod.rs; 022: the four declared files). `crates/db/src/models/mod.rs` is exactly
  one alphabetical line. `Cargo.lock` adds only `"base64"` to the existing `server` dependency
  list — no resolution drift (002 STOP trigger respected). Docs/ledger/report files fall under the
  gate's recorded exemption (ledger review-time decision 2).
- **Test discrimination.** Task 022's `configured_startup_sync_is_installed_before_constructor_returns`
  fails against the pre-fix detached shape and passes against the fix (current-thread runtime makes
  the old race deterministic). The handoff/session/owner suites assert persisted state, not just
  return values. `hash_token_pins_the_stored_encoding` pins the SHA-256 of `""` from outside the
  implementation.
- **Accepted-rationale audit (task 004, GPT challenge dismissal).** Task 004's manual-verification
  item 3 lists "hash-only persistence" among behaviors with a named test, and no such model-layer
  test exists. The ledger (Task 004 section, dismissal 2) documents this as the TS1 split:
  `create_handoff` accepts only `binding_hash`, so the raw secret cannot reach the seam, and task
  009's `initiation_issues_a_binding_cookie_and_persists_only_its_hash` compares the stored value
  to `hash_token(raw)` and rejects equality with the raw cookie. I confirmed the amended
  phase-2/009 task text still contains that test contract. This is documented, sound reasoning —
  not hidden unfinished phase-1 behavior.
- **Amended 009–012 consistency.** The plan amendments match the ledger remediation exactly:
  initiation guards only `create_handoff`; claim + epoch capture share one short guard with Hive
  I/O unlocked; commit re-checks the epoch before credential/session/sync side effects under
  `refresh_guard`; disconnect holds the epoch across increment, invalidation, revoke-all, sync
  stop and credential clear, taking `refresh_guard` only after `client.logout` (re-entrancy
  noted). Task 012 gains the four barrier-controlled race tests with mutation proofs. `plan.md`
  ordering paragraph and task table (22 tasks, 022 in phase 1) agree.
- **O8 residual.** Explicitly accepted in both the ledger and round-1 report: crash between SQLite
  revoke-all and credential clear leaves an over-locked-out node; recovery is disconnect retry;
  durable fix needs a separately approved migration. Not hidden.

## Findings

### 1. [SHOULD-FIX] Promised `sqlite-busy-snapshot-calibration-stability` scope split does not exist in the tree

- **Citation:** `.agents/reports/2026-08-22-round-1-cross-model-phase-one.md:74-80` — "Per AGENTS.md
  this will be resolved as the explicit tracked scope split `sqlite-busy-snapshot-calibration-stability`
  before the session closes; it is not silently carried forward."
- **Evidence of absence:** `dev-docs/workstreams/` contains no such directory (full listing
  checked at `ae5ee15f`); `grep -i snapshot dev-docs/BACKLOG.md` returns nothing; the
  decisions-ledger never mentions it. The calibration controls themselves (db `lifecycle.rs` /
  `queries.rs`) are untouched by this range, so the intermittent hazard the report describes is
  neither fixed nor tracked.
- **Impact:** governance, not code correctness — but AGENTS.md's no-carry-forward rule states a
  written remediation note for "later" does not satisfy the rule. The phase-1 diff is clean; the
  session's ledger is not, until this artifact exists.
- **Minimal remediation:** create
  `dev-docs/workstreams/sqlite-busy-snapshot-calibration-stability/README.md` (finding, evidence,
  required outcome) and reference it from the decisions-ledger — or fix the flake and record the
  fix — before the session closes.

### 2. [INFO] Task 022 ledger section omits the literal "SC8 not complete until 009–012" record

- **Citation:** task 022 manual-verification item 4
  (`docs/plans/local-node-browser-oauth/phase-1/022-fence-browser-login-commit-against-explicit-disconnect.md:200`)
  requires recording that "tasks 009-012 must still wire the epoch/invalidation into real routes
  before SC8 is complete." The ledger's Task 022 section
  (`docs/plans/local-node-browser-oauth/decisions-ledger.md:195-240`) records gates and Stage-2
  evidence but not that sentence.
- **Mitigating evidence:** the substance is recorded — the ledger's "Integrated phase-1 review"
  section (`decisions-ledger.md:170-193`) scopes remediation as "corrective task 022 plus amended
  tasks 009–012", and the round-1 report (`:84-87`) states "Route-level closure occurs through the
  amended tasks 009–012." A reader cannot reasonably conclude phase 1 alone delivers SC8.
- **Minimal remediation:** one sentence in the task-022 ledger section.

### 3. [INFO] Startup config-summary logging reads env, not the injected value

- **Citation:** `crates/local-deployment/src/lib.rs:470` — `has_shared_api` is computed from
  `std::env::var("VK_SHARED_API_BASE")` rather than the injected `StartupRemoteConfig.api_base`.
- **Impact:** in production (`new()` derives the injected value from the same env) the two are
  identical, so no behavioral defect; in injected-config tests the summary line could misreport.
  Cosmetic; no task contract covers it.
- **Minimal remediation:** derive the log booleans from the destructured `api_base`/config when
  the code is next touched; do not churn for it now.

## Verdict

Every phase-1 task contract is met: migration byte-identical and additive; seams exact with public
un-gated fakes; owner/handoff/session semantics probed correct at the SQL level; task 022's
primitives close the startup detached-install race and supply the disconnect fence without
altering legacy remote-client configuration behavior; all STOP triggers and file scopes respected;
accepted rationales (task-004 TS1 split, O8 crash window, in-memory epoch) are documented and
sound. Finding 1 is an outstanding session-governance obligation (tracking artifact for a
pre-existing, unrelated flake), not a defect in the reviewed implementation, and is not yet past
its own "before session closes" deadline. Findings 2–3 are informational.

VERDICT: APPROVE
