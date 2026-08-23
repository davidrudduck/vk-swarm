Full report content:

# Integrated phase-1 adversarial review

## Scope

- Range: `41f55c4b..ae5ee15f`
- Workstream: `local-node-browser-oauth`
- Lenses: mechanics and fidelity
- Current commit: `ae5ee15f6353f3e00c9e214a2b2b2414ea2b2071`
- The requested `docs/superpowers/specs/2026-08-21-local-node-browser-oauth-design.md` does not exist at either endpoint. The canonical spec referenced by `docs/plans/local-node-browser-oauth/plan.md:3-5` is `docs/superpowers/specs/2026-08-21-local-node-browser-oauth.md`; that document governed this review.

## Finding

### 1. [BLOCKING] The new startup-sync test can overwrite real macOS Keychain OAuth credentials

**Evidence:** `crates/local-deployment/src/lib.rs:1320-1329` constructs `OAuthCredentials` with a temporary path and calls `save()` with `"test-refresh-token"`. The test assumes that the temporary path isolates persistence.

That assumption is false on macOS:

- `crates/services/src/services/oauth_credentials.rs:93-108` selects `KeychainBackend` when `OAUTH_CREDENTIALS_BACKEND=keychain`, or by default when `debug_assertions` is disabled.
- `crates/services/src/services/oauth_credentials.rs:205-206` uses the fixed service/account pair `services:oauth` and `default`; the supplied temporary path is ignored.
- `crates/services/src/services/oauth_credentials.rs:228-233` writes the test refresh token to that fixed Keychain entry.
- `crates/local-deployment/src/lib.rs:1355-1358` shuts down sync and the event bus but neither restores nor clears the credential entry.

**Impact:** Running this test on macOS with the Keychain backend—including release-mode test builds or a developer environment explicitly selecting Keychain—can replace the operator’s real persisted refresh token with `"test-refresh-token"`. The damage survives the test and forces reauthentication; it may also break an actively configured node.

**Minimal remediation:** Add an explicit file-backed or in-memory credential constructor/backend injection for tests and use it here, ensuring the test cannot select Keychain regardless of build mode or environment. Do not solve this by mutating `OAUTH_CREDENTIALS_BACKEND` process-wide in a parallel test binary. Add a regression proving the test credential backend is path-scoped on macOS.

## Mechanics audit

### Owner state

- The schema structurally enforces one owner through `slot INTEGER PRIMARY KEY CHECK (slot = 1)` at `crates/db/migrations/20260821000000_add_browser_auth.sql:16-20`.
- Pin-or-verify is one `INSERT ... ON CONFLICT ... RETURNING` statement at `crates/db/src/models/browser_auth/owner.rs:26-35`.
- The conflict update is a genuine no-op, and comparison against the returned incumbent rejects another subject without moving `pinned_at` at `crates/db/src/models/browser_auth/owner.rs:37-40`.
- No owner replacement/reset path was introduced.

### Handoff state

- Creation centralizes the ten-minute TTL at `crates/db/src/models/browser_auth/handoff.rs:24-49`.
- Claim is one conditional `UPDATE ... RETURNING` at `crates/db/src/models/browser_auth/handoff.rs:67-80`. Wrong binding, expiry, replay, and unknown IDs match no row without consuming a rightful pending handoff.
- Task 022 invalidates every pending handoff using the existing terminal `claimed` state at `crates/db/src/models/browser_auth/handoff.rs:83-94`. The transition is durable and does not touch owner or session tables.
- The SQLite schema constrains states to `pending` or `claimed` at `crates/db/migrations/20260821000000_add_browser_auth.sql:27-35`.

### Session state

- Session hashes are unique at `crates/db/migrations/20260821000000_add_browser_auth.sql:41-47`.
- Authentication is local and selects only unrevoked rows, with no clock, expiry, or Hive dependency, at `crates/db/src/models/browser_auth/session.rs:35-52`.
- Browser-scoped revocation keys exclusively on the token hash and preserves the first revocation timestamp at `crates/db/src/models/browser_auth/session.rs:54-70`.
- Revoke-all updates only live session rows at `crates/db/src/models/browser_auth/session.rs:73-86`; it does not touch owner or credentials.

### Epoch and synchronization

- `LocalDeployment` stores the epoch as `Arc<Mutex<u64>>` and derives `Clone` at `crates/local-deployment/src/lib.rs:43-59`; the construction and clone-sharing test are at `crates/local-deployment/src/lib.rs:235-236` and `:1303-1313`.
- The original detached `spawn_remote_sync` remains unchanged at `crates/deployment/src/lib.rs:107-123`.
- The new synchronous installer checks and fills the slot while holding its mutex at `crates/deployment/src/lib.rs:125-135`.
- Configured startup awaits that installer before `from_parts` returns at `crates/local-deployment/src/lib.rs:493-506`, preventing disconnect from observing an empty slot followed by a late startup installation.
- Dropping an overwritten final `RemoteSyncHandle` does not orphan its task: `crates/services/src/services/share.rs:682-689` signals shutdown and aborts the join handle.

### Legacy client compatibility

The corrective refactor preserves the prior configuration boundary:

- Raw `api_base` alone drives `RemoteClient` creation at `crates/local-deployment/src/lib.rs:194-210`.
- Parsed `ShareConfig` separately controls startup synchronization at `crates/local-deployment/src/lib.rs:238-243` and `:493-495`.
- `crates/local-deployment/src/lib.rs:1361-1386` verifies a parseable non-HTTP raw base still configures `RemoteClient` when no sync configuration exists.

The private `StartupRemoteConfig` at `crates/local-deployment/src/lib.rs:124-127` does not narrow the `pub(crate)` constructor: it is declared in crate-root `lib.rs`, so it and its fields remain visible to descendant modules throughout the crate.

## Fidelity audit

- **Task 001:** The approved additive migration has the required three tables, singleton owner constraint, integer timestamps, handoff-state constraint, unique session hash, and no session expiry column. The approval token exists and is recorded before implementation.
- **Task 002:** The public, unconditionally compiled clock, token-source, fake, and hashing seams match the required interfaces. The dependency/lockfile change is limited to the existing `base64` package.
- **Task 003:** The owner model uses runtime SQLx and one atomic pin-or-compare statement. No prohibited replacement/reset path or macro-form query exists.
- **Task 004:** Creation, strict expiry, browser-bound claim, terminal replay handling, and single-statement claim match the contract.
- **Task 005:** Create, authenticate-by-hash, revoke-one, and revoke-all match the specified table and revocation boundaries.
- **Task 022:** The four declared production files contain the required invalidation helper, re-export, clone-shared epoch, synchronous installer, startup change, and compatibility injection. It does not edit OAuth routes or schema.
- Task 022 intentionally provides primitives rather than current route wiring. Its accepted residual at `docs/plans/local-node-browser-oauth/phase-1/022-fence-browser-login-commit-against-explicit-disconnect.md:196-202` assigns claim/commit/disconnect wiring to tasks 009–012; this was not treated as a phase-1 defect.
- The pre-existing orphan-worktree cleanup exposure is explicitly tracked in `dev-docs/workstreams/local-deployment-test-orphan-cleanup-safety/README.md:19-53`. The two newly added direct constructor tests call the shared guard before construction at `crates/local-deployment/src/lib.rs:1317-1319` and `:1361-1364`. The separate Keychain issue above is new and is not covered by that scope split.

## Focused checks and tree state

Read-only checks included revision/file-set inspection, `git diff --check`, runtime-query scans, baseline comparisons with `git show`, and an in-memory SQLite exercise of the migration and handoff transition. These confirmed the owner singleton, successful terminal claim, and absence of a session expiry column.

Cargo gates were not rerun because they require build/temp writes under this read-only review.

The initial `git status --short` was empty. The final status unexpectedly contained:

```text
?? .tmp-build/.tmpLLgCcu/template.db
```

That artifact appeared during the review and resembles the database test helper’s template database. Its originating process was not identified. It was not removed, reverted, or otherwise modified. A delegated standalone Rust visibility probe also used compiler temporary storage under `.review-tmp`; its temporary file was subsequently absent, and the directory was empty. No checkout, restore, stash, reset, clean, staging, or source edit was performed.

VERDICT: REJECT