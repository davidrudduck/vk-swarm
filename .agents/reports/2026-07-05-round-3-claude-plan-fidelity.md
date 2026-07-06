Plan mode is preventing the write. The full report content is in the plan file at `/home/david/.claude/plans/do-not-modify-files-spicy-nest.md`. Here is the complete report:

---

**Adversarial Plan-Fidelity Review — `remote-docker-build-fix`**

**F-001 (HIGH, UNNECESSARY|DOC_GAP) — Task 002**

Root `Dockerfile` has `ARG PNPM_VERSION=10.25.0` at line 31 despite:
- Task 002 allowed-moves: *"Do NOT add or remove any ARG or ENV lines"*
- Decisions-ledger dismissing Gemini Finding 2 as *"false positive — task 002 explicitly uses npm install -g"* (implying no ARG was added)

The code actually has `ARG PNPM_VERSION=10.25.0; RUN npm install -g pnpm@${PNPM_VERSION}` — a hybrid approach not prescribed by any task and contradicted by the dismissal record. The ledger's cited evidence (`Dockerfile:4-7`) points to the FROM area, not lines 30-32 where the ARG lives.

**Remediation:** Either (a) remove the ARG and restore hardcoded `pnpm@10.25.0`, or (b) retract the false-positive dismissal and document the ARG addition.

---

**F-002 (LOW, NEEDED) — Task 001**  
`crates/remote/Dockerfile` uses `ARG PNPM_VERSION=10.25.0` (with default) vs plan's `ARG PNPM_VERSION` (no default). Beneficial — prevents bare `docker build` from failing. Undocumented in ledger.  
*Remediation: add one-line ledger note. No code change.*

**F-003 (INFO, NEEDED) — Task 005**  
Extra `docker-build` job added to workflow despite plan saying "build not run in CI." Correctly motivated by spec SC6 and Gemini Finding 1. Documented.

**F-004 (INFO, NEEDED) — Task 004**  
`grep -oP` → `grep -oE` + `|| true` guards in assertion script. Portability + `set -euo pipefail` fix. Documented.

**F-005 (INFO, NEEDED) — Task 005**  
`grep -oP` → `grep -oE` in workflow YAML. Documented.

**F-006 (INFO, NEEDED) — Task 001**  
`SERVER_PORT` export added to `rebuild.sh` (fixes wrong healthcheck port). Documented.

**F-007/F-008 (INFO, FAITHFUL) — Tasks 003 and 001**  
`package.json` engines.node=`>=22.13` correct. Core Dockerfile changes faithful.

---

**VERDICT: FAIL** — F-001 requires remediation (decisions-ledger contradiction + plan constraint violation). All other findings are NEEDED divergences or faithful implementations.

To write this to the target path, approve the plan and I'll execute the write.