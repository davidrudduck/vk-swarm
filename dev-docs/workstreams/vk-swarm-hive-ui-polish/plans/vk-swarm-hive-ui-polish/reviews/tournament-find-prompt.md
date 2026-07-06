ADVERSARIAL TOURNAMENT — FIND + REMEDIATE. You are ONE competitor against N peers. Find every way this breakdown will FAIL an implementer, and for EACH finding propose a concrete, applicable fix.
Scoring: +1 per REAL cited problem, +1 per correct fix — BUT every finding is judged by a PEER (not you); a finding the peer rules not-real scores 0, a hand-wavy fix scores 0. Quality beats quantity: a padded/pedantic nit LOSES points. An honest `FINDINGS: 0` beats a rejected nit.

SPEC: docs/superpowers/specs/2026-07-05-vk-swarm-hive-ui-polish.md
PLAN: docs/plans/vk-swarm-hive-ui-polish/plan.md
PHASE-1 TASKS: docs/plans/vk-swarm-hive-ui-polish/phase-1/*.md
PHASE-2 TASKS: docs/plans/vk-swarm-hive-ui-polish/phase-2/*.md
PHASE-3 TASKS: docs/plans/vk-swarm-hive-ui-polish/phase-3/*.md
DECISIONS: docs/plans/vk-swarm-hive-ui-polish/decisions-ledger.md

WORKING DIR: The cwd is the repo root. All paths above are relative to it.

Attack axes (cite task id + the contradicting repo file:line for every finding):
1. Not bite-sized / two concerns in one task. 2. Wrong/non-existent anchor/symbol/Before-text (VERIFY against the real repo code — `git grep` and `cat` the referenced files). 3. Ambiguous instruction. 4. allowed_change mismatch. 5. Dependency/conflict error or cycle. 6. Unmarked irreversible. 7. Untestable or HOLLOW test (passes without the implementation — e.g. checking for a string in source code rather than testing behavior). 8. CONTROL-FLOW GROUNDING: open the real code; a plausible-but-inverted call path is a finding — symbol existence ≠ control-flow correctness. 9. Fidelity: an SC/TS clause no task truly delivers (covered-but-hollow); walk EACH clause sub-id (SC2a/SC2b…) to a task.

ADDITIONAL ATTACK AXES:
10. Missing cross-reference: a task's Before anchor references code that doesn't exist in the repo at those exact file:lines.
11. Test environment mismatch: the task's vitest test uses browser APIs in a node test or vice versa.
12. Dependency mismatch: a task depends_on a task that doesn't exist or forgot a real dependency.
13. Spec-SC gap: an SC in the spec is NOT covered by ANY task's covers_criteria — check every SC1-SC16.
14. Uncovered user story: a US in the spec has no task that addresses it.

Output one Markdown row per finding:

| Severity | Task | File:line | Issue | Remediation |

Then `FINDINGS: <n>` and a one-line self-assessment of why they survive peer review.