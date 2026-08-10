ADVERSARIAL TOURNAMENT — FIND + REMEDIATE. You are ONE competitor against 2 peers. Find every way
this breakdown will FAIL an implementer, and for EACH finding propose a concrete, applicable fix.
Scoring: +1 per REAL cited problem, +1 per correct fix — BUT every finding is judged by a PEER (not
you); a finding the peer rules not-real scores 0, a hand-wavy fix scores 0. Quality beats quantity:
a padded/pedantic nit LOSES points. An honest `FINDINGS: 0` beats a rejected nit.

SPEC: docs/superpowers/specs/2026-08-07-vk-swarm-task-breakdown.md
PLAN: docs/plans/vk-swarm-task-breakdown/plan.md
PHASE FILES: docs/plans/vk-swarm-task-breakdown/phase-*.md
TASK FILES: docs/plans/vk-swarm-task-breakdown/phase-*/*.md
(All paths relative to your working directory = the repo root.)

Attack axes (cite task id + the contradicting repo file:line):
1. Not bite-sized / two concerns in one task. 2. Wrong/non-existent anchor/symbol/Before-text
   (VERIFY against the repo). 3. Ambiguous instruction. 4. allowed_change mismatch. 5. Dependency/
   conflict error or cycle. 6. Unmarked irreversible. 7. Untestable or HOLLOW test (passes without
   the implementation). 8. CONTROL-FLOW GROUNDING: open the real code; a plausible-but-inverted call
   path is a finding — symbol existence ≠ control-flow correctness. 9. Fidelity: an SC/TS clause no
   task truly delivers (covered-but-hollow); walk EACH SC id (SC1..SC7) and TS id (TS1..TS6) to a
   task and verify the claim is real.

High-value areas to verify against this repo specifically:
- Task 102's accept-transaction claim that it can mirror Task::create + enqueue_task_upsert_op
  inside one transaction (crates/db/src/models/task/queries.rs:262-294, :339+).
- Task 203's anchor: the exit-monitor block in crates/local-deployment/src/container.rs (~711-782),
  the `success` binding at ~730, and the `if success || cleanup_done` condition.
- Task 301's assumption that a route handler can build attempt + start_attempt the way
  create_task_and_start does (crates/server/src/routes/tasks/handlers/core.rs:305-452) with a
  custom prompt and run_reason — check ContainerService::start_attempt's real signature
  (crates/services/src/services/container.rs:1193) for whether a caller can inject a prompt at all.
- Task 401's rmcp #[tool_router] append pattern (crates/server/src/mcp/task_server.rs).
- Task 503's claim that actions-dropdown.tsx has two branches needing parallel edits.
- Task 601's claim that parallel_setup_script threading is confined to project/mod.rs + queries.rs.

TOURNAMENT RULES (non-negotiable):
- You INSPECT and REPORT; you never mutate the repo. NEVER revert or discard
  working-tree state: no git checkout/restore/stash/reset/clean in ANY form, with
  or without a path argument. Parts of the decomposition may be UNCOMMITTED —
  a single `git checkout docs/plans/...` destroys the breakdown with no trace.
- Propose fixes as TEXT in your findings. Do not apply them yourself.

Output one Markdown row per finding (severity | task | file:line | issue | remediation), then
`FINDINGS: <n>` and a one-line self-assessment of why they survive peer review.
