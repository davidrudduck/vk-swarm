---
review: adversarial-review-round-1
topic: foundations-followup1
target: branch worktree-bridge-cse_01Xf9p3eZr6VxJMaXNEheyyW (PR #448)
head_at_dispatch: caed109d
date: 2026-06-28
---

# Adversarial Review — Round 1

## Models run

| Model | How | Status |
|---|---|---|
| Opus | Local checkout via Agent tool (general-purpose, model: opus) | ✓ Ran |
| Gemini | Local filesystem via cc-gemini-plugin:gemini-agent | ✓ Ran |
| Codex | Local checkout via codex:codex-rescue | ✓ Ran |

All three ran successfully. 3-of-3 pass (no fallback needed).

## Pre-flight finding (fixed before dispatch)

**BLOCKING — `Cargo.lock` and `crates/services/Cargo.toml` uncommitted (task 202)**

Found during pre-flight `git status -sb`: the `tracing-test = "0.2"` dev-dependency and its
`Cargo.lock` entries were present in the working tree but never committed. The branch would not
compile from a clean checkout because `#[traced_test]` on
`test_cleanup_orphan_executions_stubborn_pid_escalation` requires the crate. Fixed in `caed109d`
before challenger dispatch.

## Consolidated findings

| # | Issue (cited) | Tag | Accepted? | Impact if shipped | Remediation |
|---|---|---|---|---|---|
| 1 | `crates/services/Cargo.toml` + `Cargo.lock` not committed; `tracing-test` crate missing from committed state | [BLOCKING] | yes | `cargo build` fails from clean checkout; test with `#[traced_test]` cannot compile | Committed in `caed109d` (pre-review) |
| 2 | `crates/services/src/services/container.rs:387` — `get_fence_attempt_count` `.unwrap_or(0)` silently suppresses escalation warn on transient DB read error | [SHOULD-FIX] | yes | A persistent DB read error after a successful increment produces `count=0` every restart, hiding both the error and the escalation warn indefinitely | Replaced with explicit `match` → `tracing::error!` + `continue`; committed in `2bf87093` |
| 3 | `docs/superpowers/specs/…md:84` — SC2c says "configurable threshold" but code uses `const FENCE_ESCALATION_THRESHOLD: i64 = 5` (spec self-contradiction) | [SHOULD-FIX] | yes | Spec-vs-code divergence; a future implementer adding a `VK_*` env var for this would not know the decision was deliberate | Amended SC2c to remove "configurable"; added implementation note explaining why `const` is correct; committed in `2aedcac9` |
| 4 | `docs/plans/…/decisions-ledger.md:116-140` — reachability-gate line cites drifted from code (317/354/381/414 vs originally stated 316/351/382/401) | [INFO] | yes | Wrong line numbers in a governing artifact; a verifier checking cited lines finds the right code ±1-2 lines, not a defect | Corrected to live line numbers in `2aedcac9` |
| 5 | `crates/services/src/services/container.rs` — escalation warn uses `>= threshold` so re-fires on cycles 6, 7, … not only at first crossing | [INFO] | yes (intentional) | None — repeated warnings are the correct operator signal for a permanently stuck process | No code change; recorded as intentional in decisions-ledger |
| 6 | `CLAUDE.md` / `AGENTS.md` governance additions are out-of-task-scope per the spec's stated constraints | [INFO] | yes (documented) | None — already disclosed in decisions-ledger under "Post-execution review decisions" | No change; accepted scope addition with rationale recorded |
| 7 | `cargo fmt --all -- --check` fails on workstream-authored code (and pre-existing violations on base) | [SHOULD-FIX] | yes | `cargo fmt` gate fails; CLAUDE.md "Finish What We Start" violated | `cargo fmt --all` applied to full workspace in `2bf87093` |

## Final gate state (HEAD: `2aedcac9`)

| Gate | Result |
|---|---|
| `cargo fmt --all -- --check` | ✓ PASS (exit 0; nightly-feature warnings are non-fatal) |
| `cargo clippy --all --all-targets --all-features -- -D warnings` | ✓ PASS |
| `cargo check -p services -p local-deployment -p db` | ✓ PASS |
| All three new tests compile and referenced | ✓ PASS (verified by Codex + Gemini) |

## Lessons learned

**What the cross-family pass caught that a single reviewer would have rationalised past:**

1. **The uncommitted Cargo files** — A single reviewer reading only source files would miss that
   `.unwrap_or(0)` in a `match` arm is semantically correct but a second DB call failing silently
   is not. Codex flagged it as [SHOULD-FIX] while Gemini rated it [INFO]; the stronger tag won.
   The fix (log + continue) is strictly better.

2. **The fmt gate** — Gemini actually ran `cargo fmt --all -- --check` and counted violations
   before/after the workstream. Neither Opus nor Codex ran the formatter gate explicitly. A single
   reviewer doing only a code read would have missed that the base was already failing and the
   workstream worsened it.

3. **The spec self-contradiction** — Opus caught "configurable" in SC2c as a [SHOULD-FIX] while
   Gemini only rated it [INFO] (noting the spec's own Implementation section prescribed `const`).
   The cross-check forced a resolution rather than letting the ambiguity persist.

**Standing debt accepted (none):**

All [BLOCKING] and [SHOULD-FIX] findings were remediated in-session. The three [INFO] items
(stale line numbers, re-fire behavior, governance scope drift) are all corrected or documented in
the decisions-ledger. No debt is carried forward.
