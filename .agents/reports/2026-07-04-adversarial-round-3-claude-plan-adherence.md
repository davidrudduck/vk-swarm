ExitPlanMode is not in the deferred tools list — it appears to be a built-in not accessible from here. Since the Write tool is blocked by plan mode for the report path, I'll output the full report as text so the runner can capture it (matching how the Gemini round 3 report was handled when their write also failed).

---

**REPORT FOR:** `.agents/reports/2026-07-04-adversarial-round-3-claude-plan-adherence.md`

```markdown
# Adversarial Plan-Adherence Review — Round 3

**Panelist:** claude  
**Branch:** `fix/preexisting-gate-failures`  
**Commits reviewed:** `6b5c9adb`, `7fc7955e`, `9e20efb4`  
**Date:** 2026-07-04

---

## Verdict: PARTIALLY FOLLOWED

The implementation achieved the stated mechanical goal (all four gate checks report green)
and properly remediated the intermediate `doctest = false` violation raised by Gemini round-1
review. However, two divergences from the governing rules remain in the final committed state:
(1) the terminal_session PTY tests are excluded via a command-line `--skip` flag rather than
resolved at source level, which the newly-added AGENTS.md paragraph explicitly prohibits;
and (2) 35 `rust,ignore`'d doctests in the `remote` and `services` crates were left without
a tracked follow-up workstream, raising an ambiguous-but-real silent-deferral concern.
The `create_test_pool_with_migrations()` replacement and `extract_project_name` test addition
are minor warranted and benign divergences respectively.

---

## 1. Was the plan followed?

**Goal:** All four mandatory gate checks green on the final committed state with no silent
debt carry-forward.

### Gate check status (final committed state — commit `9e20efb4`)

| Gate | Status | Evidence |
|------|--------|----------|
| `cargo clippy --all --all-targets --all-features -- -D warnings` | ✅ Green | Confirmed in all three commit messages |
| `cargo test --workspace` | ⚠️ Partial | Green only with `--skip terminal_session` (see D1) |
| `cd frontend && npm run lint` | ✅ Green | Confirmed in all three commit messages |
| `cd frontend && npx tsc --noEmit` | ✅ Green | Confirmed in all three commit messages |

The three-commit sequence demonstrates genuine, responsive remediation:

- **`6b5c9adb`**: Added `doctest = false` to `crates/remote/Cargo.toml` and
  `crates/services/Cargo.toml` — a rule violation under No Deferred Remediation.
- **`7fc7955e`**: Removed `doctest = false` entries; replaced 37 globally-suppressed
  doctests with per-item `rust,ignore` fences; codified the gate-bypass prohibition
  in AGENTS.md and CLAUDE.md.
- **`9e20efb4`**: Applied all tournament round-1 findings — fixed 34 non-standard
  `` `,ignore `` fences to `rust,ignore`, made 3 zero-I/O doctests live, added
  `extract_project_name` unit tests, and eliminated duplicated schema via
  `create_test_pool_with_migrations()`.

**What was well-executed:**
- The intermediate `doctest = false` violation was caught and fixed in the same session —
  consistent with No Deferred Remediation.
- The AGENTS.md/CLAUDE.md rule addition was correctly applied retroactively, then
  immediately self-applied.
- Tournament findings (F001–F005 + gemini:F002) were all remediated.
- The 3 live doctests (`NodeApiKeyError`, `SwarmProjectError`, `HiveSyncConfig`) are
  correctly structured with valid public paths and zero-I/O semantics (verified below).

**Where it fell short:**
- `terminal_session` PTY tests are excluded via `--skip` at invocation time, not via
  source-level `#[ignore]` as the new AGENTS.md rule requires.
- 35 `rust,ignore`'d doctests remain without a tracked follow-up workstream.

---

## 2. Divergences identified

| # | Divergence | Citation | Needed? | Remediation/Doc |
|---|-----------|---------|---------|-----------------|
| D1 | `cargo test --workspace` passes only with `--skip terminal_session`; 16 PTY-dependent tests in terminal_session.rs are NOT marked `#[ignore]` at source level | `crates/services/src/services/terminal_session.rs:840–990`; commit `6b5c9adb` message: "known PTY hang" | Needed (no PTY in headless env) | AGENTS.md doc update OR source-level `#[ignore]` on the 9 PTY-spawning tests (see §3) |
| D2 | 35 `rust,ignore`'d doctests in `remote` (30) and `services` (5) left without a tracked follow-up workstream | `crates/remote/src/db/tasks.rs:239`; `crates/remote/src/nodes/ws/session.rs:980`; `crates/services/src/services/remote_client.rs:911` (and 32 others) | Needed (genuinely require live DB environment) | AGENTS.md clarification that per-item `rust,ignore` is compliant only with a workstream (see §3) |
| D3 | `create_test_pool_with_migrations()` replaces hand-rolled `setup_db()` schema | `crates/services/tests/electric_task_sync.rs:284,328,392,483` | Needed — eliminates schema drift risk | Testing standards doc update for AGENTS.md (see §3) |
| D4 | Added 6 `#[test]` unit tests for private `extract_project_name` | `crates/remote/src/nodes/ws/session.rs:5596–5634` | Not needed — was not a gate failure | None; benign scope creep, no regressions |

### Verification: the 3 live doctests are correctly structured

All three public paths were verified against the crate module tree:

| Doctest | Path used | Verification |
|---------|-----------|-------------|
| `service.rs:105` NodeApiKeyError→NodeError | `remote::db::node_api_keys::NodeApiKeyError`, `remote::nodes::NodeError` | `crates/remote/src/db/mod.rs:8` → `pub mod node_api_keys`; `crates/remote/src/nodes/mod.rs:14` → `pub use service::NodeError`. Both public. ✅ |
| `service.rs:131` SwarmProjectError→NodeError | `remote::db::swarm_projects::SwarmProjectError`, `remote::nodes::NodeError` | `crates/remote/src/db/mod.rs:17` → `pub mod swarm_projects`; `crates/remote/src/db/swarm_projects.rs:9` → `pub enum SwarmProjectError`. Public. ✅ |
| `hive_sync.rs:70` HiveSyncConfig::default() | `services::services::hive_sync::HiveSyncConfig` | `crates/services/src/lib.rs:1` → `pub mod services`; `crates/services/src/services/mod.rs:30` → `pub mod hive_sync`; `crates/services/src/services/hive_sync.rs:45` → `pub struct HiveSyncConfig`. Public. ✅ |

All three are zero-I/O assertions (pure enum mapping, struct default values). No
regression risk introduced.

### Verification: create_test_pool_with_migrations() migration risk

The `./migrations` path in `crates/db/src/test_utils.rs:125` resolves at compile time
to `crates/db/migrations/`. Grep for Postgres-specific syntax (`SERIAL`, `BIGSERIAL`,
`bytea`, `::uuid`, `ARRAY`, `PG_`) across all 5 migration files returns zero results.
All migrations are pure SQLite DDL. Risk of future Postgres contamination: LOW (the `db`
crate is SQLite-only; `remote`'s Postgres migrations are managed separately by that crate).

### Verification: the `--skip terminal_session` issue

The 16 tests in `terminal_session.rs:840–990` are `#[tokio::test]` with no `#[ignore]`
marker. They call `manager.create_session(temp_dir.path()).await`, which spawns real PTY
sessions via `portable-pty`. In a headless environment these block indefinitely. The
pre-existing practice of skipping via `--skip terminal_session` at invocation time is
documented in the round-4 scoreboard (`.agents/reports/2026-07-03-round-4-tournament-
scoreboard.md:128`) and in commit `6b5c9adb`'s message. This session acknowledged it as
"known PTY hang" but did not fix it at source level. The new AGENTS.md paragraph (added
in `7fc7955e`) states: _"Broken tests or documentation examples must be resolved at the
source level — fixed, or selectively marked with the standard per-item attributes."_
Runtime `--skip` is not a source-level fix.

---

## 3. Needed divergences — proposed documentation updates

### D1 — terminal_session PTY hang

**Option A (stricter — recommended):** Mark the 9 PTY-spawning tests `#[ignore]` at
source level. Tests that don't spawn PTY sessions (`test_detect_tmux_available`,
`test_manager_initialization`, `test_session_not_found`, `test_session_exists`,
`test_list_sessions_empty`, `test_get_session_not_found`, `test_session_count`) continue
running normally.

```rust
// crates/services/src/services/terminal_session.rs (~line 895)
#[tokio::test]
#[ignore = "requires live PTY device; pass --include-ignored in interactive shells"]
async fn test_create_session_in_directory() {
```

Apply to all 9 tests calling `manager.create_session(...)` (lines ~895, 924, 946, 965,
982 and neighbors — exact set confirmed via `cargo test -p services -- terminal_session
--list`).

**Option B (pragmatic — if PTY test value must be preserved in interactive runs):**

> **Proposed addition to AGENTS.md, under "Finish What We Start":**
>
> ```
> Known invocation-time exceptions to `cargo test --workspace`:
>
> - `--skip terminal_session` — 9 tests in
>   `crates/services/src/services/terminal_session.rs` spawn live PTY sessions
>   (portable-pty) and hang in headless environments. They are not marked
>   `#[ignore]` so they run normally in interactive shells with a PTY device.
>   A follow-up workstream should add `#[cfg_attr(not(feature = "pty-tests"),
>   ignore)]` so the exclusion is configuration-driven rather than implicit.
> ```

---

### D2 — 35 `rust,ignore`'d doctests without a follow-up workstream

**Proposed clarification to AGENTS.md, appended after the gate-bypass prohibition
paragraph:**

> ```
> Per-item `rust,ignore` or `#[ignore]` markers are the sanctioned source-level path for
> tests that cannot currently run (e.g., requiring a live database, network endpoint, or
> PTY device unavailable in CI). Their use is legitimate PROVIDED the session either:
>   (a) makes at least one test in the category live so the suite is not entirely dead;
>       AND
>   (b) creates a tracked follow-up workstream (`dev-docs/workstreams/<name>/README.md`)
>       documenting which tests remain ignored and what is required to bring them live.
>
> Marking tests ignored without (b) is a deferred deferral — it satisfies the letter
> of "source-level per-item attribute" while violating the spirit of "clean ledger."
> ```

Note: criterion (a) IS met in this branch — 3 doctests are live. Criterion (b) is NOT.

---

### D3 — `create_test_pool_with_migrations()` as the standard

**Proposed addition to AGENTS.md under a "Testing Standards" subsection:**

> ```
> When a test requires a populated SQLite database, use one of:
>   - `db::test_utils::create_test_pool()` — fast template-copy approach; prefer for
>     the majority of tests (~90% faster than running migrations per test)
>   - `db::test_utils::create_test_pool_with_migrations()` — fresh migrations per test;
>     use only when the test exercises migration behavior itself
>
> Never manually duplicate `CREATE TABLE` SQL in test helpers (e.g., setup_db()-style
> functions). Schema defined outside `crates/db/migrations/` will drift from production
> and produce false-green tests that mask real regressions.
> ```

---

## 4. Unneeded divergences — proposed remediations

### D4 — `extract_project_name` test addition

No remediation required. The 6 tests at `session.rs:5596–5634` are pure unit assertions
for a private function that was already correctly `rust,ignore`'d in its doctest. They
introduce no regressions and correctly cover URL parsing, Windows paths, trailing slashes,
empty strings, and no-separator cases. The only characterisation warranted is "benign
scope creep" prompted by tournament finding F005.

---

## 5. Overall assessment

The three-commit sequence on `fix/preexisting-gate-failures` demonstrates a genuinely
responsive remediation effort. The initial `doctest = false` gate bypass was a rule
violation; it was caught by Gemini round-1 and fully corrected in commit `7fc7955e` —
exactly the intended session-level feedback loop. Tournament findings were applied
precisely: non-standard `` `,ignore `` syntax was standardised to `rust,ignore`, three
zero-I/O doctests were correctly promoted to live with verified public import paths, and
the schema-duplication debt in `electric_task_sync.rs` was cleanly resolved by adopting
the existing `create_test_pool_with_migrations()` helper.

Two residual concerns prevent a full "FOLLOWED" verdict. First, the `terminal_session`
PTY test exclusion is accomplished via `--skip` at invocation time rather than per-item
`#[ignore]` at source level, which is the form the new AGENTS.md paragraph mandates;
this carry-forward was acknowledged in the commit message ("known PTY hang") but not
resolved. Second, 35 `rust,ignore`'d doctests remain in `remote` and `services` without
a corresponding tracked workstream — technically using a sanctioned per-item attribute,
but leaving the underlying debt invisible to future sessions. Both issues require a small
documentation fix or source-level annotation rather than structural rework, and neither
introduces correctness regressions.
```