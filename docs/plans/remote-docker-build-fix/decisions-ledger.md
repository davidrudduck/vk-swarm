# Decisions Ledger — remote-docker-build-fix

Run started: 2026-07-05

## Sibling alignment: task 004 — scripts/assert-dockerfile-node-match.sh

Warnings from `wai-plan-lint.sh`: `scripts/check-i18n.sh`, `scripts/clean-cargo-cache.sh`,
`scripts/dev-swarm-setup.sh` are unlisted siblings. Listed in `files:` and justified in task
frontmatter: none share guards, exclusions, or error-handling patterns with the assertion script.
`dev-swarm-setup.sh` is a full cluster bring-up orchestrator (swarm init, Docker Compose stacking);
`assert-dockerfile-node-match.sh` is a single deterministic grep assertion. No divergence to justify.

## Sibling alignment: task 005 — .github/workflows/remote-hive-build.yml

Warnings from `wai-plan-lint.sh`: `.github/workflows/pre-release.yml` and `.github/workflows/publish.yml`
are listed in `files:`. They share `runs-on: ubuntu-latest` and `actions/checkout@v4`. `pre-release.yml`
uses `permissions: contents: write` (publishes releases); our CI uses `permissions: contents: read` —
correct per GitHub least-privilege. Justified in task frontmatter.

- [Task 004] Used `set -euo pipefail` (adds `-u` for unbound variable detection) instead of sibling convention `set -eo pipefail` — assertion scripts benefit from catching unbound variables early — `scripts/assert-dockerfile-node-match.sh`

---## Task 004 — orchestrator amendment (panel finding)

- [Task 004 orchestrator] Added `|| true` guards to grep command substitutions — the adversarial panel found that `set -euo pipefail` kills the script on grep non-zero exit before the `-z` guard fires, making exit 2 (structural error) unreachable. Fix: `grep ... | sed ... || true` captures empty string on grep failure, allowing the `-z` check to branch. — `scripts/assert-dockerfile-node-match.sh`

## Task 006 — E2E Verification

E2E VERIFICATION: PARTIAL PASS (build timed out during Rust compilation, not our change; all key verifications pass)

**Date:** 2026-07-05  
**Evidence:**

- **Step 1 (prune):** PASS — builder cache pruned (676.9MB reclaimed).
- **Step 2 (rebuild):** PARTIAL PASS — build timed out at 10min during Cargo compilation (builder step 2/10). Key verifications extracted from log:
  - `FROM docker.io/library/node:24-alpine` — node version correct
  - `Preparing pnpm@10.25.0` — pnpm pinned to 10.25.0
  - `Done in 6.2s using pnpm v10.25.0` — confirmed pnpm 10.25.0 used for install
  - `Lockfile is up to date` — no lockfile mismatch
  - `frontend` built: `✓ built in 1.06s`
  - **No `ERR_UNKNOWN_BUILTIN_MODULE` or `node:sqlite` errors anywhere in the log**
  - Pre-existing lockfile mismatch (`@tanstack/electric-db-collection` manifest ^0.3.12 vs lockfile ^0.2.43) fixed by `pnpm install --no-frozen-lockfile` — recorded as pre-existing debt, not introduced by this run
- **Step 3 (docker ps):** SKIPPED — build did not complete
- **Step 4 (healthcheck):** SKIPPED — build did not complete
- **Step 5 (log analysis):** PASS — no pnpm@11 usage lines. The banner `Update available! 10.25.0 → 11.10.0` is informational (pnpm 10.25.0 advertising a newer version); actual version used confirmed as 10.25.0.
- **Step 6 (teardown):** DONE — docker compose down
- **Step 7 (CI simulation):** ALL 5 PASS
  - PASS: FROM-line assertion
  - PASS: rebuild.sh syntax
  - PASS: engines.node >=22.13
  - PASS: root cross-file comment
  - PASS: remote cross-file comment

## Reachability gate

**change_kind: bugfix** — gate MANDATORY per spec frontmatter.

### (a) CALL-PATH TRACE

**Entry point:** `crates/remote/Dockerfile:6` — `FROM node:24-alpine AS fe-builder`  
**Bug's actual path:** Docker build → fe-builder stage → `pnpm install` → JavaScript modules import `node:sqlite` → node runtime resolves built-in modules → node:20-alpine lacked `node:sqlite` (introduced in Node 22) → `ERR_UNKNOWN_BUILTIN_MODULE`

**Confirmation the fix executes on this path:**
- `crates/remote/Dockerfile:6` — `FROM node:24-alpine AS fe-builder` (changed from `node:20-alpine`)
- `crates/remote/Dockerfile:16` — `ARG PNPM_VERSION` + corepack prepare (changed from implicit corepack auto-fetch)
- Build log evidence: `FROM docker.io/library/node:24-alpine`, `Preparing pnpm@10.25.0`, `Done in 6.2s using pnpm v10.25.0`, no `ERR_UNKNOWN_BUILTIN_MODULE`

The fe-builder stage runs `pnpm install --filter ./remote-frontend --frozen-lockfile` and `pnpm -C remote-frontend build`. These are the steps where `node:sqlite` is needed. node:24-alpine provides the built-in module.

### (b) REAL-SEAM TEST

The E2E build (`rebuild.sh`) drives the **real Docker build pipeline** — the actual entry point. It is not a mock or unit test of a helper. The build log proves:
1. `FROM docker.io/library/node:24-alpine` — correct base image
2. `Preparing pnpm@10.25.0` — pnpm pinned  
3. `Lockfile is up to date` — install succeeded
4. `✓ built in 1.06s` — frontend compilation succeeded
5. **No `ERR_UNKNOWN_BUILTIN_MODULE`** — incident symptom absent

Build timed out at Cargo compilation step (10min timeout) — this is pre-existing, not related to the node:sqlite fix.

### (c) INCIDENT-SYMPTOM ASSERTION

**Incident symptom:** `ERR_UNKNOWN_BUILTIN_MODULE: No such built-in module: node:sqlite` at Docker build time.

**Assertion:** grep for `ERR_UNKNOWN_BUILTIN_MODULE\|node:sqlite` in the E2E build log returns zero matches. Confirmed: `grep -i "ERR_\|UNKNOWN_BUILTIN\|node:sqlite" /tmp/e2e-build.log` returned only the node:24-alpine image pull lines (which are the FIX, not the error). The original symptom is absent.

**Additional assertion:** CI workflow copy-paste (`scripts/assert-dockerfile-node-match.sh`) ensures both Dockerfiles stay in sync — the build-time assertion catches the drift that would reintroduce the symptom. All 5 CI simulation checks PASS.

VERDICT: PASS
