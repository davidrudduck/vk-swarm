---
workstream: terminal-session-pty-tests
doc_type: readme
status: active
title: "Bring 5 #[ignore]'d PTY-spawning tests in terminal_session back to live"
originated_in: fix/preexisting-gate-failures
originated_commit: 051cdeea
adrs: []
staging_pointers: []
---

# terminal-session-pty-tests

## Context

The `fix/preexisting-gate-failures` branch remediated all pre-existing gate failures so the
mandatory AGENTS.md gate passes. During that work, 5 PTY-spawning unit tests in
`crates/services/src/services/terminal_session.rs` were marked `#[ignore]` at the source level
because they require a live PTY device that is unavailable in CI and most non-interactive shells.

This workstream tracks the debt so it is invisible to no future session. It was created in
response to the adversarial review round 2 finding (Claude SHOULD-FIX + Codex BLOCKING #2) that
the `#[ignore]` markers were a self-referential violation of the AGENTS.md rule requiring a tracked
follow-up workstream — the PR added the rule and marked the tests `#[ignore]` in the same commit
but only created a workstream for the 35 ignored doctests, not for the 5 PTY tests.

## Inventory

| File | Line | Test name | Why ignored | Path to live |
|------|------|-----------|-------------|---------------|
| `crates/services/src/services/terminal_session.rs` | 895 | `test_create_session_in_directory` | Requires live PTY device | Run under `--include-ignored` in interactive shells, or extract PTY logic behind a trait and mock it |
| `crates/services/src/services/terminal_session.rs` | 926 | `test_create_duplicate_session` | Requires live PTY device | Same as above |
| `crates/services/src/services/terminal_session.rs` | 949 | `test_kill_session` | Requires live PTY device | Same as above |
| `crates/services/src/services/terminal_session.rs` | 969 | `test_write_to_session` | Requires live PTY device | Same as above |
| `crates/services/src/services/terminal_session.rs` | 987 | `test_resize_session` | Requires live PTY device | Same as above |

## Acceptance criteria

- [ ] All 5 tests either run in CI (via trait mocking or a PTY test harness) or are explicitly
  documented as requiring `--include-ignored` and at least one test in the `terminal_session`
  module is live (satisfying the AGENTS.md "at least one test in the category remains live" rule).
- [ ] `cargo test --workspace` passes without `--skip terminal_session`.
- [ ] No regression in the mandatory gate (clippy, test, lint, tsc).

## Approach

1. **Trait extraction (preferred):** Extract the PTY-spawning logic behind a `PtyBackend` trait.
   The 5 tests mock the backend and run in CI. One integration test uses the real backend under
   `#[cfg(test)]` with `--include-ignored` for interactive verification.
2. **Alternative — test harness:** Use a crate like `portable-pty` to provide a PTY in CI. Heavier
   dependency, but avoids trait refactoring.
3. **Minimum viable:** If the above are too heavy, convert the 5 tests to `#[ignore = "requires live PTY; ..."]`
   (already done) and add at least one `#[test]` in the module that exercises the non-PTY logic
   (e.g. `Session::id()`, `Session::name()`, config parsing) so the category is not entirely dead.

## Status

Active — created in the `fix/preexisting-gate-failures` session per the No-Deferred-Remediation
rule. The debt was made visible (source-level `#[ignore]` + this workstream) rather than hidden
(invocation-time `--skip`).
