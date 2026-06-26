---
id: "301"
phase: 3
title: Executor resume-capability audit (capability map)
status: ready
depends_on: []
parallel: false
conflicts_with: []
files:
  - docs/plans/vk-swarm-node-foundations/notes/301-executor-resume-capability.md
irreversible: false
scope_test: "N/A"
allowed_change: create
covers_criteria: [SC1]
---
## Failing test (write first)
N/A — analysis task producing a capability-map artifact. Verified via the Manual verification section.

## Change
- **File:** `docs/plans/vk-swarm-node-foundations/notes/301-executor-resume-capability.md` (NEW)
- **Anchor:** new capability-map note.
- **Content:** A TABLE with one row per `CodingAgent`/`BaseCodingAgent` variant in this fork (enumerate
  from `crates/executors/src/executors/mod.rs` `enum CodingAgent` and `profile.rs` `BaseCodingAgent`):
  ClaudeCode, Amp, Gemini, QwenCode, Droid, Codex, CursorAgent, Opencode, Copilot, QaMock (+ any
  others present). Columns:
  | variant | supports `spawn_follow_up(--resume session_id)`? | how (flag/evidence: cite the executor
  file:line, e.g. `claude.rs:252`) | on crash-resume → **resume** / **cold-respawn from
  executor_action** / **mark-failed** | resume-prompt semantics (replay vs continue, if known) | notes |
- For each variant, OPEN its executor file and check whether it implements a real `spawn_follow_up` with
  a `--resume`/session mechanism (Claude does — `crates/executors/src/executors/claude.rs:252`), or
  whether follow-up starts fresh. Classify each into the recovery branch task 304 will take:
  - **resume**: has session resume → rebuild a follow-up action with the recovered `session_id`.
  - **cold-respawn**: no session resume but safe to re-run the original `executor_action` from scratch
    in the worktree (idempotent-ish) → re-spawn the original action.
  - **mark-failed**: neither safe → mark failed and surface (the last resort, SC8).
- **Resolve the open resume-prompt question** (see ledger): for the **resume** class, state per-executor
  what `prompt` to pass to `spawn_follow_up` on a crash (no new user input). Recommend a default
  (minimal continuation prompt vs re-sending the original prompt from `executor_sessions.prompt`) with
  the reasoning. This decision feeds task 303.

## Allowed moves
ONLY create the capability-map note. No source changes. This is the analysis that makes 304's branch
selection and 303's prompt choice non-arbitrary.

## STOP triggers
- An executor's follow-up/resume behaviour cannot be determined from its source → mark it
  **mark-failed** (conservative default) and record the uncertainty; do not guess "resume".
- The audit reveals NO executor besides Claude supports resume AND cold-respawn is unsafe for all →
  STOP and surface to the user: crash-resume would be Claude-only in practice, which the spec's
  SC1-fallback anticipates but the user should confirm is acceptable for the initial fleet.

## Manual verification (record in decisions-ledger)
- `test -f docs/plans/vk-swarm-node-foundations/notes/301-executor-resume-capability.md` → exists.
- The table has a row for EVERY variant in `enum CodingAgent` (cross-check count against
  `grep -c '(' ` the enum body) → record both counts; they must match.
- Every row's "how" column cites a real `file:line` in `crates/executors/src/` → spot-check 3.
- The resume-prompt recommendation is stated → confirm present; record the chosen default in the ledger
  so 303 consumes it.

## Done when
The capability-map note exists, covers every variant with cited evidence, classifies each into a
recovery branch, and states the resume-prompt default — all confirmed and recorded in the
decisions-ledger. (Manual-verification task; no gate script.)
