---
review: code-review-round-1
topic: foundations-followup1
target: branch worktree-bridge-cse_01Xf9p3eZr6VxJMaXNEheyyW vs merge-base a4ff0b89
date: 2026-06-28
effort: high
---

# Code Review — Round 1

## Scope

Full branch diff `a4ff0b89..HEAD` (39 files, ~2862 insertions). Substantive logic isolated to:
`crates/services/src/services/container.rs`, `crates/local-deployment/src/container.rs`,
`crates/db/src/models/execution_process/queries.rs`, the `fence_attempt_count` migration, and the
process-recovery documentation authored this session. The remaining 13 files in commit `2bf87093`
are `cargo fmt --all` output.

## Method

Three parallel finder subagents (general-purpose, high effort):
- **Finder A** — `services/container.rs`: make_process_inspector hook, CouldNotKill escalation,
  TestContainerService, two new tests.
- **Finder B** — `local-deployment/container.rs`: drain spy field/cfg-gating, constructors,
  boot-drain wiring, test.
- **Finder C** — verified all 13 `cargo fmt` files are behavior-preserving (no logic in a fmt commit).

Plus direct review of `process_fence.rs`, `queries.rs`, the migration, and the `.mdx` docs against
the real boot code (`crates/server/src/main.rs`).

## Findings

| # | Issue (cited) | Sev | Conf | Actionable? | Remediation |
|---|---|---|---|---|---|
| R1-01 | `docs/architecture/process-management.mdx` + `docs/features/process-recovery.mdx` documented the boot sequence as **drain → cleanup**, blocking before HTTP serving. Real code (`main.rs:128-146`) is **cleanup → drain**, sequenced in one background `tokio::spawn`, concurrent with serving. Docs authored this session were factually inverted. | high | high | yes | FIXED — rewrote "Boot-Drain Ordering" + "Crash Recovery" intro in process-management.mdx; corrected "How Recovery Works" intro, "Queued Message Drain", and `RUST_LOG` phrasing in process-recovery.mdx |

## Non-actionable

- **Duplicated `warn!` arms (if/else)** in `container.rs` CouldNotKill — pure style; explicitly out of scope (Finder A).
- **`let _ = set_resume_state('pending')`** ignores error — pre-existing before this diff, not introduced (Finder A).
- **`cargo fmt --all` touched 13 out-of-workstream files** — already adjudicated as accepted scope in adversarial-review-round-1 finding #7.
- **Governance additions to CLAUDE.md/AGENTS.md** — already adjudicated in adversarial-review-round-1 finding #6.

## Verifications confirming NO defect

- Threshold `count >= 5` fires exactly at 5 and re-fires every cycle (intentional); test loop `1..=5` with guarded pre-threshold negative assertion is correct.
- All three `continue`s in the CouldNotKill arm correctly skip the resume path; post-loop blanket sweep still runs; `pid_raw` type/value correct.
- `mark_orphaned_as_failed` excludes `pending`/`resumed` (queries.rs:124) — stuck row stays Running, matching test.
- `#[cfg(test)] drain_spy_tx` correctly initialized in the sole struct-literal constructor; `cargo build -p local-deployment` (non-test) compiles — field absent from production builds.
- Boot-drain test calls the real `drain_queued_messages_on_boot` (not `query_drainable`), asserts exactly one drain.
- All 13 fmt files: no changed identifiers, literals, control flow, arg order, or glob/shadowing risk.

## Verdict

One actionable finding (R1-01), remediated in-session. Re-review required to confirm convergence.

Actionable: [R1-01]
