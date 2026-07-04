# Adversarial review round 2 — preexisting-gate-failures

**Date:** 2026-07-04
**Target:** full 4-commit branch diff (`6b5c9adb`→`051cdeea`, 65 files, +2820/-189) on
`fix/preexisting-gate-failures` (PR #452, `davidrudduck/vk-swarm`).
**Governing intent:** AGENTS.md + CLAUDE.md rules ("Finish What We Start", "No Deferred
Remediation", the gate-bypass prohibition, per-item ignore legitimacy, testing standards).

## Resilience record (ADR 0010 — all three primary executors ran)

| Panelist | CLI | Version | Task | Safety | Effort | Status |
|----------|-----|---------|------|--------|--------|--------|
| Claude   | claude-cli-panel | claude 2.1.200 | codebase-review | read-only | high | ok |
| Codex    | codex-cli-panel  | codex-cli 0.142.5 | codebase-review | read-only | high | ok |
| Gemini   | gemini-cli-panel | gemini 0.49.0 | codebase-review | read-only | high | ok |

No fallback to OpenCode/GLM was required (no disputed findings to adjudicate).

Reports: `.agents/reports/2026-07-04-adversarial-round-4-{claude,codex,gemini}-round-2.md`.

## Consolidated findings

| # | Issue | Tag | Source | Accepted? | Remediation |
|---|-------|-----|--------|-----------|-------------|
| 1 | 5 PTY `#[ignore]` tests have no tracked follow-up workstream | SHOULD-FIX / BLOCKING | Claude + Codex (unanimous) | Yes | Created `dev-docs/workstreams/terminal-session-pty-tests/README.md` with inventory of 5 tests, acceptance criteria, 3 approach options |
| 2 | 35 ignored doctests tracked in README but no decisions-ledger entry | BLOCKING | Codex | Yes | Created `docs/plans/preexisting-gate-failures/decisions-ledger.md` documenting both scope splits (`remote-services-doctest-revival` + `terminal-session-pty-tests`) with reasoning, source-level markers, acceptance criteria, and the dismissed false positive |
| 3 | `service.rs:804` live doctest breaks `cargo test --workspace` | BLOCKING | Codex | No — false positive | Dismissed with evidence: `cargo test --doc -p remote` reports `line 804 ... ok`. The doctest defines `async fn unlink_example()` but never calls it; `crate::` resolves in rustdoc external-crate context because `NodeServiceImpl` is `pub` |
| 4 | Trailing whitespace in review artifacts | INFO | Codex | Noted | All in generated artifacts (`.agents/reports/*.md`, `target.diff`); non-blocking hygiene debt |

## Verdicts

- **Claude:** REVISE → fixed (workstream created).
- **Codex:** REVISE → fixed (workstream + decisions-ledger created; F3 dismissed as false positive).
- **Gemini:** APPROVE (0 findings).

## Lessons learned

1. **The workstream-requirement rule is self-referential.** This PR adds the rule requiring tracked
   workstreams for `#[ignore]` markers AND marks tests `#[ignore]` in the same PR. The first round
   created the workstream for doctests but missed the PTY tests — a gap caught only by the
   cross-family review. Future rule-codification PRs must audit EVERY `#[ignore]`/`rust,ignore`
   marker in the diff and confirm a workstream exists for each.
2. **The decisions-ledger requirement is easy to miss.** AGENTS.md requires scope splits to be
   documented in the decisions-ledger before PR submission, but no decisions-ledger existed for this
   branch (it was a gate-fix, not a WAI workstream). Codex caught this; the ledger is now created at
   `docs/plans/preexisting-gate-failures/decisions-ledger.md`. Consider surfacing the
   decisions-ledger requirement more prominently in AGENTS.md.
3. **False positives still require evidence.** Codex's F3 (live doctest "breaks the gate") was a
   confident BLOCKING claim that would have wasted remediation effort if accepted uncritically.
   `cargo test --doc -p remote` was the one-line verification that dismissed it. Every finding must
   be verified against the real repo before acceptance.
4. **Three-family review converges.** Gemini's clean APPROVE + Claude/Codex's targeted SHOULD-FIX
   findings converged on a small, correct remediation set. No disputes required the OpenCode/GLM
   decider — the three primary executors were sufficient.

## Gate verification (all green on `051cdeea`, the reviewed state)

- `cargo clippy --all --all-targets --all-features -- -D warnings`: clean
- `cargo test --workspace`: 0 failed (no `--skip` flags; PTY tests `#[ignore]`'d at source)
- `cd frontend && npm run lint`: clean
- `cd frontend && npx tsc --noEmit`: exit 0

Round-2 remediation (this commit) adds documentation only — no code changes, no gate impact.
