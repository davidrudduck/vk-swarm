# Adversarial Review — Round 1 (Plan-Adherence)

**Target:** `git diff origin/main...HEAD` on `fix/preexisting-gate-failures` (PR #452, davidrudduck/vk-swarm)
**Governing intent:** AGENTS.md "Finish What We Start" + "No Deferred Remediation" + gate-bypass prohibition paragraph
**Date:** 2026-07-04

---

## Resilience record

All three primary executors ran via real CLI harnesses (not in-harness subagents). All returned `status: ok`.

| Executor | CLI version | Status | Report |
|----------|-------------|--------|--------|
| Claude (Opus) | claude-cli 2.1.200 | ok | `.agents/reports/2026-07-04-adversarial-round-3-claude-plan-adherence.md` |
| Codex | codex-cli 0.142.5 | ok | `.agents/reports/2026-07-04-adversarial-round-3-codex-plan-adherence.md` |
| Gemini | gemini 0.49.0 | ok | `.agents/reports/2026-07-04-adversarial-round-3-gemini-plan-adherence.md` |

---

## Consolidated findings (deduped across 3 challengers)

| # | Issue | Tag | Accepted? | Impact if shipped | Remediation |
|---|-------|-----|-----------|-------------------|-------------|
| 1 | `--skip terminal_session` is invocation-time, not source-level `#[ignore]` — violates new AGENTS.md gate-bypass paragraph | SHOULD-FIX | Yes (Claude D1) | Gate-green claim relies on `--skip` flag rather than source-level annotation; contradicts the rule codified in the same branch | Mark 5 PTY-spawning tests `#[ignore]` at source level; `cargo test --workspace` passes without `--skip` |
| 2 | 35 `rust,ignore`'d doctests without tracked follow-up workstream | SHOULD-FIX | Yes (Claude D2, Codex 2, Gemini D3 — unanimous) | Debt invisible to future sessions; violates spirit of "clean ledger" | Created `dev-docs/workstreams/remote-services-doctest-revival/README.md` with full inventory + acceptance criteria |
| 3 | Document `create_test_pool_with_migrations()` as the standard | SHOULD-FIX | Yes (Claude D3, Codex 4, Gemini D1 — unanimous) | Future tests may re-introduce hand-rolled `CREATE TABLE` schema duplication | Added "Testing standards" section to AGENTS.md + condensed version to CLAUDE.md |
| 4 | `unlink_swarm_project` doctest at service.rs:804 likely broken (uses `crate::` path) | INFO | No — false positive (Codex 6) | None — doctest passes | Dismissed: `cargo test --doc -p remote` confirms `line 804 ... ok`; `crate::` resolves in rustdoc external-crate context because `NodeServiceImpl` is pub |
| 5 | `extract_project_name` tests added — scope creep | INFO | Noted (Claude D4, Codex 5, Gemini D2) | None — benign, no regressions, correctly covers edge cases | No remediation needed; prompted by tournament finding F005 |
| 6 | Final gate not evidenced in committed artifacts | SHOULD-FIX | Yes (Codex 7) | Gate-green claim not independently verifiable from artifacts | This report + commit message record the gate evidence |

---

## Remediation applied in-session

### Finding 1 — terminal_session PTY tests

Marked 5 PTY-spawning tests `#[ignore = "requires live PTY device; run with --include-ignored in interactive shells"]` at source level in `crates/services/src/services/terminal_session.rs`:
- `test_create_session_in_directory` (line 895)
- `test_create_duplicate_session` (line 925)
- `test_kill_session` (line 947)
- `test_write_to_session` (line 966)
- `test_resize_session` (line 983)

`cargo test --workspace` now passes **without** `--skip terminal_session`. The 11 non-PTY tests in the same module continue to run normally.

### Finding 2 — tracked follow-up workstream

Created `dev-docs/workstreams/remote-services-doctest-revival/README.md` with:
- Full inventory of all 35 ignored doctests (30 remote + 5 services)
- Per-doctest: file, line, symbol, why ignored, path to live
- Acceptance criteria (0 ignored, no regression)
- Approach (convert to `no_run`, remove redundant, or integration-test)

### Finding 3 — testing standards documentation

Added "Testing standards" section to AGENTS.md after the gate-bypass prohibition paragraph:
- `db::test_utils::create_test_pool()` for fast template-copy
- `db::test_utils::create_test_pool_with_migrations()` for full schema
- Prohibition on hand-written `CREATE TABLE` in test helpers

Condensed version added to CLAUDE.md No-Deferred-Remediation bullet.

### Finding 4 — false positive dismissed

`cargo test --doc -p remote` output: `test crates/remote/src/nodes/service.rs - nodes::service::NodeServiceImpl::unlink_swarm_project (line 804) ... ok`. The `crate::` path resolves correctly because rustdoc compiles doctests as a crate that depends on the tested crate, and `crate::` refers to the `remote` crate. `NodeServiceImpl` is pub.

### Finding 6 — gate evidence

The full mandatory gate was run on the final committed state and recorded in the commit message:
- `cargo clippy --all --all-targets --all-features -- -D warnings` — clean
- `cargo test --workspace` — 0 failed (no `--skip` needed)
- `cd frontend && npm run lint` — clean
- `cd frontend && npx tsc --noEmit` — exit 0

---

## Lessons learned

1. **The rule you write today applies to the work you do today.** The gate-bypass prohibition paragraph added in commit `7fc7955e` should have immediately surfaced the `--skip terminal_session` inconsistency — it took a third adversarial round (Claude) to catch that the new rule was not being applied to the existing `--skip` practice. A single-model reviewer rationalized past this; the cross-family pass caught it.

2. **Source-level attributes are necessary but not sufficient.** Marking 35 doctests `rust,ignore` satisfied the letter of "per-item source-level attribute" but without a tracked workstream the debt was invisible. The three challengers unanimously flagged this as a deferred deferral. The fix (tracked workstream) is cheap; the discipline of creating it is what matters.

3. **`crate::` in doctests is not always broken.** Codex flagged `service.rs:804` as a likely broken doctest based on the `crate::` path pattern that broke 31 other doctests. It passed because `NodeServiceImpl` is pub and the full module path resolves in rustdoc's external-crate compilation model. The distinction: `crate::` breaks when the type is NOT re-exported at the module level or the function is `pub(crate)`; it works when the full path to a pub item is valid. A single model over-generalized the pattern; verification against the actual `cargo test --doc` output dismissed it.

4. **Unanimous findings are the strongest signal.** Findings 2 and 3 were raised independently by all three challengers (Claude, Codex, Gemini) from different angles. This is the highest-confidence signal the multi-family adversarial review produces.
