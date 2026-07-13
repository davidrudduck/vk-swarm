ADVERSARIAL TOURNAMENT — FIND + REMEDIATE. You are ONE competitor against N peers. Find every way
this breakdown will FAIL an implementer, and for EACH finding propose a concrete, applicable fix.
Scoring: +1 per REAL cited problem, +1 per correct fix — BUT every finding is judged by a PEER (not
you); a finding the peer rules not-real scores 0, a hand-wavy fix scores 0. Quality beats quantity:
a padded/pedantic nit LOSES points. An honest `FINDINGS: 0` beats a rejected nit.

SPEC: docs/superpowers/specs/2026-07-10-error-handling-and-dialog-a11y.md
PLAN: docs/plans/error-handling-and-dialog-a11y/plan.md
TASK FILES: docs/plans/error-handling-and-dialog-a11y/phase-{1,2,3,4}/*.md

Attack axes (cite task id + the contradicting repo file:line):
1. Not bite-sized / two concerns in one task. 2. Wrong/non-existent anchor/symbol/Before-text
   (VERIFY against the repo). 3. Ambiguous instruction. 4. allowed_change mismatch. 5. Dependency/
   conflict error or cycle. 6. Unmarked irreversible. 7. Untestable or HOLLOW test (passes without
   the implementation). 8. CONTROL-FLOW GROUNDING: open the real code; a plausible-but-inverted call
   path is a finding — symbol existence ≠ control-flow correctness. 9. Fidelity: an SC/TS clause no
   task truly delivers (covered-but-hollow); walk EACH clause sub-id (SC2a/SC2b…) to a task.

Output one Markdown row per finding (severity | task | file:line | issue | remediation), then
`FINDINGS: <n>` and a one-line self-assessment of why they survive peer review.
