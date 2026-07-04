The review is complete. `ExitPlanMode` is not available in this environment's tool registry, so I'll present the findings directly. The full report content is saved in the plan file at `/home/david/.claude/plans/do-not-modify-files-eager-charm.md` for when execution is approved.

---

## Adversarial Review — Round 4, Claude (Round 2)

**Branch:** `fix/preexisting-gate-failures` (commit `051cdeea`)  
**Verdict: REVISE — 1 [SHOULD-FIX]**

---

### Lens 1 — Mechanics / Correctness

Everything checks out:

**Three live doctests — correct.** Paths verified through the module tree:
- `remote::db::node_api_keys::NodeApiKeyError` → public all the way; `NotFound → ApiKeyNotFound` mapping confirmed at `service.rs:114`. ✓
- `remote::db::swarm_projects::SwarmProjectError` → `NotFound → ProjectNotInHive` confirmed at `service.rs:139`. ✓
- `services::services::hive_sync::HiveSyncConfig` → all five `Default` assertion values match `hive_sync.rs:76–83`. ✓

**Six `extract_project_name` tests — all correct.** Each test traced against the implementation at `session.rs:63–76`. Edge cases (empty string, trailing slash, Windows backslash, no separator, URL, no trailing sep) all produce correct output. ✓

**`create_test_pool_with_migrations()` replacement — semantics preserved.** The global `UNIQUE` on `git_repo_path` was lifted by migration `20260125060012_fix_git_repo_path_unique_constraint.sql`; each test inserts exactly one project into its own isolated DB — no constraint violation possible. ✓

**`#[file_serial]` and `#[serial]` — correct and sufficient.** `file_locks` feature enabled on the `remote` crate; in-process `#[serial]` on `mcp_context_test.rs` is appropriate for env-var races; no other crate incorrectly uses `file_serial`. ✓

**PTY `#[ignore]` attribution — correct.** All five markers land on tests calling `manager.create_session(...)`. Non-PTY tests remain live. Condition (a) of the AGENTS.md rule is satisfied. ✓

---

### Lens 2 — Fidelity & Completeness

**[SHOULD-FIX] PTY `#[ignore]` tests have no tracked follow-up workstream**

Location: `crates/services/src/services/terminal_session.rs:895, 926, 949, 969, 987`

The AGENTS.md rule added in commit `051cdeea` states:

> Per-item `#[ignore]` markers are legitimate PROVIDED… **(b) creates a tracked follow-up workstream `dev-docs/workstreams/<name>/README.md` documenting which tests remain ignored.**

Evidence of violation:
```
$ grep -r "PTY\|terminal_session" dev-docs/workstreams/*/README.md
(no output)
```

The `remote-services-doctest-revival/README.md` explicitly covers "35 rust,ignore'd doctests in `remote` (30) and `services` (5)" — its inventory lists `container.rs`, `hive_sync.rs`, `remote_client.rs`, `share/processor.rs`, `share/publisher.rs`. `terminal_session.rs` is absent from the entire `dev-docs/workstreams/` tree.

This is a self-referential violation: the PR adds the workstream requirement rule and marks 5 tests `#[ignore]` in the same commit, but creates the workstream only for the 35 doctests — not for the PTY unit tests.

**Fix:** Create `dev-docs/workstreams/terminal-session-pty-tests/README.md` listing the 5 ignored tests (`test_create_session_in_directory`, `test_create_duplicate_session`, `test_kill_session`, `test_write_to_session`, `test_resize_session`), the reason (portable-pty blocks in headless environments), and acceptance criteria (all pass with `--include-ignored` in an interactive shell).

---

**Everything else passes:** Doctest inventory count (30 remote + 5 services = 35 ✓), `doctest = false` fully removed from all `Cargo.toml` files ✓, testing-standards docs match the code changes ✓, no cross-component contradictions found ✓.