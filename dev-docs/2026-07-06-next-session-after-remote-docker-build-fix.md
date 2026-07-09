# Next Session — remote-docker-build-fix

**Status: IN-FLIGHT** — PR #458 needs `/wai:ship` to merge. Round-4 adversarial findings unreviewed.

## Current State

- **Workstream:** `remote-docker-build-fix`
- **PR:** #458 (branch `opencode/clever-forest`), 22 files, CI pending
- **Phase:** Post-close — `/wai:close` completed, specs still `status: active`. MASTER.md regenerated. Staging pointers recorded.
- **Gate status:** Lint clean, tsc clean, clippy clean, tests passing. Merge conflicts resolved (ours on root-cause spec).
- **Code-review:** 2 rounds converged (`Actionable: []`). Trailing newline fixed.
- **Adversarial round-4:** 3 panels returned (Codex 5 findings, Gemini 6 findings, Opus 1 finding). Reports untracked at `.agents/reports/2026-07-06-round-4-*-post-fix.md`. Findings JSON at `docs/plans/remote-docker-build-fix/tournament/round-4/`. NOT reviewed or remediated.
- **PR feedback:** 7/7 inline resolved, 22/22 comments replied. Completeness ok.
- **Unpushed:** None (branch is `opencode/clever-forest`, pushed to origin).

## Immediate Next — /wai:ship

Run `/wai:ship remote-docker-build-fix` to:
1. Flip spec frontmatter to `status: shipped`
2. Merge PR #458 to main
3. Graduate staging docs into `dev-docs/`
4. Clean up branch + worktree

## Open Question — Adversarial Round 4 Findings

Before shipping, decide whether to remediate round-4 findings. The 3 panels (Codex, Gemini, Opus) ran on the post-fix state and returned findings. Quick assessment:

**Codex (5 findings, .agents/reports/2026-07-06-round-4-codex-post-fix.md):**
- F001: CI doesn't run rebuild.sh end-to-end (docker compose build in subshell, pipefail propagation)
- F002: Healthcheck `|| echo` swallows curl failure exit codes
- F003: Dockerfile ARG defaults not gated in CI (already addressed in 6ca7e604 via grep check)
- F004: Unrelated pnpm-lock.yaml drift in diff
- F005: E2E verification was partial pass (build timed out, healthcheck didn't reach 200)

**Gemini (6 findings, .agents/reports/2026-07-06-round-4-gemini-post-fix.md):**
- F001-F006: All appear to be parsing/corruption artifacts (space in `pnpm @`, truncated jq commands, broken actions/checkout paths). Likely Gemini panel read corrupted diff — **dismiss** unless verified against real code.

**Opus (from round-4 logs, if available):**
- Check `docs/plans/remote-docker-build-fix/tournament/round-4/` for claude-panel.json or findings-opus.json.

**Recommendation:** Glance at Codex F001 and F002 — both are low-confidence quality items. F003 already fixed. F004 is expected (pnpm-lock.yaml always regenerates). F005 is documented in the decisions ledger. Shipable as-is OR fix F001/F002 in a follow-up workstream.

## Three World-Class Improvements

These are NOT required to ship. They elevate the fix from "works" to "provably-correct, monitored, auditable" — the difference between done and best-of-breed.

### 1. Drift Prevention — Pre-Commit Hook Cascade

**Problem:** The fix stops the BLEED (build passes) but doesn't prevent the DRIFT (someone changes one Dockerfile's FROM line or PNPM_VERSION without remembering the other, and doesn't run CI before pushing a branch that doesn't hit the path globs).

**Solution:** Wire `scripts/assert-dockerfile-node-match.sh` into a Husky pre-commit hook (`frontend/.husky/pre-commit` already exists — add a `bash scripts/assert-dockerfile-node-match.sh` check). Add a second assertion: a script that compares `package.json::packageManager`, both `Dockerfile::ARG PNPM_VERSION=`, and `docker-compose.yml::PNPM_VERSION` fallback — exits 1 on ANY mismatch. Both scripts run in < 50ms; zero friction.

**Files:** `scripts/assert-pnpm-version-match.sh`, `.husky/pre-commit` (append), `package.json` (if hook wiring needed)

**Spec anchors:** `SC6` (CI plumbing) — extends it to pre-commit plumbing.

### 2. Observability — Scheduled CI Build + Healthcheck Loop

**Problem:** The PR-triggered CI job validates plumbing but only runs on path-matched PRs. A silent drift (e.g., Node 24 → 26, pnpm 10.25.0 → 11.x via renovate bumping `packageManager` without updating the hardcoded Dockerfile ARG) could go undetected for weeks if no PR touches the watched paths.

**Solution:** Add a `schedule:` trigger to `remote-hive-build.yml` (e.g., `cron: '0 6 * * 1'` — Monday mornings). The scheduled run does the FULL pipeline: `docker compose build --no-cache && docker compose up -d && sleep 5 && curl -f http://localhost:9000/v1/health`. Add a Slack webhook notification on failure (via `slackapi/slack-github-action@v1` or `rtCamp/action-slack-notify@v2`). The `LOOPS_EMAIL_API_KEY=ci-placeholder` workaround works because the healthcheck doesn't call Loops — it just checks server liveness.

**Files:** `.github/workflows/remote-hive-build.yml` (add schedule + healthcheck step + notify), optional `.github/workflows/slack-notify.yml`

**Spec anchors:** `SC5` (rebuild.sh + compose rebuild works) — proves it continuously, not just once.

### 3. Provenance — OCI Labels + Build Metadata

**Problem:** When a production image breaks, you need to know which commit built it, which pnpm version was used, and which Node version it runs. Currently: zero labels.

**Solution:** Add OCI-standard labels to both Dockerfiles:
```dockerfile
LABEL org.opencontainers.image.version="${VK_GIT_BRANCH}-${VK_GIT_COMMIT}"
LABEL org.opencontainers.image.revision="${VK_GIT_COMMIT}"
LABEL org.opencontainers.image.created="$(date -u +'%Y-%m-%dT%H:%M:%SZ')"
LABEL com.clever-forest.pnpm-version="${PNPM_VERSION}"
LABEL com.clever-forest.node-version="24-alpine"
```
Source `VK_GIT_COMMIT` and `VK_GIT_BRANCH` from `rebuild.sh` (already exported as env vars — pass them as build args). The labels appear in `docker inspect` and in any registry UI. Combined with the healthcheck, this gives you a full provenance chain: commit → pnpm version → node version → build timestamp → healthcheck status.

**Files:** `Dockerfile`, `crates/remote/Dockerfile`, `crates/remote/rebuild.sh` (add build args), `crates/remote/docker-compose.yml` (add build args)

**Spec anchors:** None — net-new capability. Extends the workstream's "reproducible build" intent to "auditable build."

---

## Handoff Prompt for Next Session

```
You are picking up the remote-docker-build-fix workstream after `/wai:close` succeeded.

CONTEXT:
- PR #458 is open on branch `opencode/clever-forest`, merged with origin/main, all gates green.
- Adversarial round-4 findings exist but are untracked (`.agents/reports/2026-07-06-round-4-*.md` and `docs/plans/remote-docker-build-fix/tournament/round-4/`).
- Code review converged (2 rounds, Actionable:[]).
- The close committed and pushed (`ca14235b`).

TASK 1 — SHIP IT:
Run `/wai:ship remote-docker-build-fix`. This flips specs to shipped, merges PR, graduates docs, and cleans up the branch.

TASK 2 (OPTIONAL) — ROUND-4 REMEDIATION:
Review the adversarial round-4 findings. Gemini's 6 findings appear to be parsing artifacts (space-injected regex patterns) — likely dismissable after verifying against real code. Codex F001/F002 are the only substantive ones (pipefail subshell exit + healthcheck error masking). Decide whether to fix them in a follow-up workstream or dismiss them in the decisions ledger before shipping.

TASK 3 (OPTIONAL, WORLD-CLASS) — PICK ONE:
Read `dev-docs/2026-07-06-next-session-after-remote-docker-build-fix.md` (this file). Choose one of the three world-class improvements and implement it as a follow-up workstream (use `/wai:prd-new` to capture intent, then `/wai:spec` + `/wai:precheck` + `/wai:decompose`). Recommendation: start with #1 (pre-commit hook) — highest ROI, lowest complexity, zero CI minutes.
```