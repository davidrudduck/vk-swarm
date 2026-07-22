---
id: "001"
phase: 1
title: crates/remote/Dockerfile fe-builder: node:24-alpine + ARG PNPM_VERSION + corepack prepare; +compose/rebuild plumbing
status: passed
depends_on: []
parallel: false
conflicts_with: ["002", "004"]
files:
  - crates/remote/Dockerfile
  - crates/remote/docker-compose.yml
  - crates/remote/rebuild.sh
irreversible: false
scope_test: "N/A"
allowed_change: edit
covers_criteria: [SC3, SC4]
covers_tests: [TS4]
---
## Failing test (write first)

The following shell snippet FAILS on the current codebase (exits non-zero) and PASSES
after this task is applied. Run it BEFORE making any change; record the failure. Run it
again AFTER; record the pass.

```bash
# FAILS NOW: remote Dockerfile still uses node:20-alpine (no sync comment, no node:24)
grep -q "FROM node:24-alpine" crates/remote/Dockerfile || { echo "FAIL: remote Dockerfile still on old node version (expected node:24-alpine)"; exit 1; }
# FAILS NOW: no corepack prepare line exists
grep -q 'corepack prepare.*pnpm' crates/remote/Dockerfile || { echo "FAIL: no corepack prepare line in remote Dockerfile"; exit 1; }
# FAILS NOW: no PNPM_VERSION in compose build args
grep -q "PNPM_VERSION" crates/remote/docker-compose.yml || { echo "FAIL: no PNPM_VERSION in docker-compose.yml"; exit 1; }
# FAILS NOW: no PNPM_VERSION export in rebuild.sh
grep -q "PNPM_VERSION" crates/remote/rebuild.sh || { echo "FAIL: no PNPM_VERSION export in rebuild.sh"; exit 1; }
echo "PASS: all guards applied"
```

## Change

### File 1: `crates/remote/Dockerfile`

**Anchor 1 — the `FROM` line at line 5.** Change the `fe-builder` stage base to `node:24-alpine`
and add a cross-file coupling comment.

Before:
```dockerfile
FROM node:20-alpine AS fe-builder
```

After:
```dockerfile
# keep in sync with root Dockerfile's builder stage (both node:24-alpine)
FROM node:24-alpine AS fe-builder
```

**Anchor 2 — the `RUN corepack enable` at line 8.** Add `ARG PNPM_VERSION` before it and
`corepack prepare` after it, so the `package.json` pin is honoured.

Before:
```dockerfile
RUN corepack enable
```

After:
```dockerfile
ARG PNPM_VERSION
RUN corepack enable \
 && corepack prepare "pnpm@${PNPM_VERSION}" --activate
```

**Notes:**
- `ARG PNPM_VERSION` can go anywhere in the top scope of the `fe-builder` stage, but placing it
  immediately before the `RUN corepack enable` keeps the two coupled lines together for readability.
- Do NOT add ARG PNPM_VERSION in the builder or runtime stages — their `ARG APP_NAME` is already
  correct and they don't need this variable.
- Do NOT change any other line in this file. The `COPY`, `RUN pnpm install`, `RUN pnpm build`
  lines stay exactly as-is.
- The `--activate` flag is critical — without it `corepack prepare` downloads but does not
  wire the shim.

### File 2: `crates/remote/docker-compose.yml`

**Anchor — the `remote-server:` service's `build.args:` block at lines 121–127.**
Add `PNPM_VERSION` as the first build arg.

Before:
```yaml
      args:
        # SQLx needs DATABASE_URL at compile time for query validation
        # Use offline mode with sqlx-data.json, or provide a dummy URL
        SQLX_OFFLINE: "true"
        # Git info for build identification (passed from host)
        VK_GIT_COMMIT: ${VK_GIT_COMMIT:-unknown}
        VK_GIT_BRANCH: ${VK_GIT_BRANCH:-unknown}
```

After:
```yaml
      args:
        # pnpm version — sourced from package.json packageManager field by rebuild.sh
        PNPM_VERSION: ${PNPM_VERSION:-10.25.0}
        # SQLx needs DATABASE_URL at compile time for query validation
        # Use offline mode with sqlx-data.json, or provide a dummy URL
        SQLX_OFFLINE: "true"
        # Git info for build identification (passed from host)
        VK_GIT_COMMIT: ${VK_GIT_COMMIT:-unknown}
        VK_GIT_BRANCH: ${VK_GIT_BRANCH:-unknown}
```

**Notes:**
- `${PNPM_VERSION:-10.25.0}` provides a default so a bare `docker compose build` without
  `rebuild.sh` still works (defaults to the same version in `package.json`).
- Do NOT change any other build arg, env var, or port in the compose file.

### File 3: `crates/remote/rebuild.sh`

**Anchor — after the `export VK_GIT_BRANCH` line (currently line ~4).** Add an export that
reads `packageManager` from `package.json`.

Before:
```bash
export VK_GIT_BRANCH=$(git branch --show-current)
```

After:
```bash
export VK_GIT_BRANCH=$(git branch --show-current)
export PNPM_VERSION=$(jq -r .packageManager package.json | sed 's/pnpm@//')
```

**Notes:**
- `jq` must be installed on the host (it is a standard tool on dev machines; the Docker
  container does not need it — the value is passed as a build arg before the container runs).
- The `cd "$(dirname "$0")"` at the start means `rebuild.sh` runs from `crates/remote/`.
  But `package.json` is at the repo root. The one-liner must account for this: path is
  `../..` relative to `crates/remote/`. But looking at the script, it's `cd "$(dirname "$0")"`
  which puts the cwd at `crates/remote/`. So `package.json` is at `../../package.json`.
  However, `jq -r .packageManager package.json` assumes cwd is the repo root.
  **Decision:** change the path to `../../package.json`:

```bash
export PNPM_VERSION=$(jq -r .packageManager ../../package.json | sed 's/pnpm@//')
```

This is the final `After` text for the `rebuild.sh` edit.

## Allowed moves
- Edit only the blocks described in the Change section above.
- Do NOT change: `FROM rust:1.89-slim-bookworm`, `FROM debian:bookworm-slim`, any `COPY`,
  `RUN apt-get`, `ENV`, `EXPOSE`, `HEALTHCHECK`, `ENTRYPOINT`, or `USER` lines.
- Do NOT remove or re-order any line.
- The `docker-compose.yml` edit is ADDITIVE only — add one block; change nothing else.
- The `rebuild.sh` edit is ADDITIVE only — add one export line; change nothing else.

## STOP triggers
- If `FROM node:20-alpine AS fe-builder` is NOT on line 5 of `crates/remote/Dockerfile`, halt
  and report the actual line content.
- If `RUN corepack enable` is NOT on line 8 (or the adjacent line after the ARG), halt.
- If `jq` is not installed (`which jq` exits non-zero), halt. Install `jq` first.
- If `package.json` does NOT contain `"packageManager": "pnpm@10.25.0"`, halt and report
  the actual value. Update the fallback in `docker-compose.yml` to match.
- If any file outside `files:` changes, halt.

## Manual verification (record in decisions-ledger)

1. Verify the `rebuild.sh` export works:
   ```bash
   cd crates/remote
   source <(grep PNPM_VERSION rebuild.sh)
   echo "PNPM_VERSION=$PNPM_VERSION"
   ```
   Expected: `PNPM_VERSION=10.25.0`

2. Verify docker-compose.yml parses:
   ```bash
   docker compose --env-file .env.remote config 2>&1 | grep -q PNPM_VERSION
   ```
   Expected: exit 0 (PNPM_VERSION appears in the expanded config).

3. Verify the Dockerfile ARG is reachable:
   ```bash
   PNPM_VERSION=10.25.0 docker compose --env-file .env.remote build --no-cache remote-server 2>&1 | tee /tmp/build-001.log
   ```
   Expected: exit 0 (build succeeds). Grep the log:
   ```bash
   grep -q "pnpm@10.25.0" /tmp/build-001.log
   ```
   Expected: exit 0 (corepack downloaded and prepared pnpm 10.25.0, NOT 11.x). Also:
   ```bash
   grep "ERR_UNKNOWN_BUILTIN_MODULE" /tmp/build-001.log
   ```
   Expected: exit 1 (no hits — the error is gone).

## Done when
`WAI_TYPECHECK_CMD="" WAI_TEST_CMD="" bash ~/.claude/wai/scripts/task-gate.sh remote-docker-build-fix 001` exits 0