---
review: code-review-round-3
topic: foundations-followup1
target: branch worktree-bridge-cse_01Xf9p3eZr6VxJMaXNEheyyW vs merge-base a4ff0b89
date: 2026-06-28
effort: high
---

# Code Review — Round 3 (CodeRabbit-fix delta)

**Target:** branch worktree-bridge-cse_01Xf9p3eZr6VxJMaXNEheyyW   **Range:** `a4ff0b89..683fd5e1`   **Effort:** high

## Scope

Incremental review covering commit `683fd5e1` ("fix(foundations-followup1): address coderabbit
review findings"), which was applied after Round 2 converged. Round 2 already cleared the full
branch diff; this pass reviews only the three code deltas in `683fd5e1`:

1. `crates/local-deployment/src/container.rs` — `let _ = tx.send(...)` → `.expect("...")`
2. `crates/services/src/services/container.rs:380` — `let _ = set_resume_state(...).await` → `.await?`
3. `crates/services/src/services/container.rs:429-444` — `unwrap_or(None)` → `match ... { Err(e) => { log; continue; } }`

Plus plan doc wording fix (documentation only — not code-reviewed).

## Method

Two parallel finder subagents (general-purpose, high effort), plus direct adjudication of the
disputed finding:

- **Finder A** — `services/container.rs`: error-handling consistency of `await?` vs `continue`.
- **Finder B** — `local-deployment/container.rs` + `services/container.rs` full delta scan;
  also verified `NotOurProcess` arm asymmetry.

The two finders disagreed on whether `await?` at line 380 is a defect. Adjudicated inline below.

## Findings

| # | File:line | Severity | Category | Finding | Confidence | Actionable? |
|---|-----------|----------|----------|---------|-----------|-------------|

*(No rows — see verdict below.)*

## Non-actionable

| # | File:line | Severity | Category | Finding | Confidence | Why non-actionable |
|---|-----------|----------|----------|---------|-----------|---------------------|
| R3-N1 | `container.rs:380` | low | quality | Finder A flagged `await?` vs `continue` inconsistency: `set_resume_state('pending')` propagates on failure while counter reads immediately below use `continue`. | high | **Intentional asymmetry — not a defect.** The `pending` write is a safety guard for the blanket `mark_orphaned_as_failed` call at line 519 (runs AFTER the loop). If `set_resume_state` fails and we `continue`, line 519 would mark a live D-state process as `failed` — data corruption. With `await?`, the function exits before line 519 runs; the stuck process stays `running` and is retried on the next boot. The counter reads and session lookup at lines 381-444 all execute after `pending` is safely written, so `continue` is correct there. The `await?` vs `continue` split is the correct and intentional design. |
| R3-N2 | `container.rs:361` | low | quality | `let _ = set_resume_state(pool, process.id, "abandoned")` in `NotOurProcess` arm uses `let _ =` (vs `await?` in `CouldNotKill` at line 380). | medium | **Intentional asymmetry — pre-existing, out of scope.** `NotOurProcess` means the PID was reused; the process is already gone. Writing `abandoned` is best-effort bookkeeping only: `mark_process_failed_with_task_update` at line 367 still runs and correctly updates task state regardless of whether the state write succeeded. The `let _ =` is safe here; `await?` would be needlessly strict for a bookkeeping-only write. Pre-existing behavior, not introduced by this diff. |

## Adjudication record (finder split on R3-N1)

Finder A called `await?` at line 380 a real defect and proposed replacing it with `match ... { Err(e) => { log; continue; } }`. Finder B called it correct and intentional. Adjudication:

The key fact is that `mark_orphaned_as_failed` lives at line **519** — AFTER the `for` loop closes at line ~426. If `set_resume_state('pending')` fails and execution `continue`s:
- The process row has no `pending` state.
- The loop finishes; line 519 runs `mark_orphaned_as_failed`.
- `mark_orphaned_as_failed` excludes only rows with `resume_state IN ('pending','resumed')`.
- This stuck-alive process lacks that protection → it gets incorrectly marked `failed`.
- This is data corruption (a live D-state process, which the OS is still running, is marked failed).

With `await?`:
- The function returns `Err` before line 519.
- `mark_orphaned_as_failed` never runs in this boot cycle.
- The caller (`main.rs:129-134`) logs a warning.
- The stuck process stays `running` in the DB, safely deferred to the next boot's cleanup.
- No other processes from this boot's candidate list get processed either, but none of them were at risk of being incorrectly marked failed by the blanket sweep for THIS one DB error.

**Verdict:** `await?` is correct. Finder A's proposed `continue` would introduce data corruption. Finding closed as non-actionable.

## Verdict: Approve

Loop converged at Round 3. No actionable findings.

Actionable: []
