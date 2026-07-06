---
id: "004"
phase: 2
title: Create FROM-line assertion script (grep-based node version match)
status: ready
depends_on: []
parallel: false
conflicts_with: ["001", "002", "005"]
files:
  - scripts/assert-dockerfile-node-match.sh
  - scripts/check-i18n.sh
  - scripts/clean-cargo-cache.sh
irreversible: false
scope_test: "N/A"
allowed_change: create
covers_criteria: [SC3]
covers_tests: [TS3]
---
## Sibling alignment: `scripts/check-i18n.sh`

Listed in `files:` per `wai-plan-lint.sh` advisory. `scripts/check-i18n.sh` is an ESLint-based
i18n regression checker — it does a shallow git clone, diff-against-baseline, JSON key consistency
checks. `scripts/clean-cargo-cache.sh` is a cargo cache sweeper. Neither shares any structure
(guards, exclusions, error-handling patterns) with `scripts/assert-dockerfile-node-match.sh`.
Justification recorded in decisions ledger.

## Failing test (write first)

The following shell snippet FAILS on the current codebase (exits non-zero — the file does
not exist yet). After this task, it passes.

```bash
# FAILS NOW: assertion script does not exist yet
test -f scripts/assert-dockerfile-node-match.sh || { echo "FAIL: script not created"; exit 1; }
# FAILS NOW: cannot execute absent script
bash scripts/assert-dockerfile-node-match.sh && echo "PASS: FROM lines match" || {
  echo "FAIL: script returned non-zero (either absent or mismatch)"
  exit 1
}
```

## Change

### File: `scripts/assert-dockerfile-node-match.sh` (NEW)

Create the file with the following content. This is a deterministic shell script with
no dependencies beyond `sed`, `grep`, and `bash`.

```bash
#!/usr/bin/env bash
# Assert the two fe-builder stages in Dockerfile and crates/remote/Dockerfile
# are on the same node base image major version. Catches drift between the two
# Dockerfiles (the bug that caused the ERR_UNKNOWN_BUILTIN_MODULE failure).
# Usage: bash scripts/assert-dockerfile-node-match.sh
# Exit 0 if they match, 1 with a diff if they don't.
set -euo pipefail

ROOT_DOCKERFILE="${1:-Dockerfile}"
REMOTE_DOCKERFILE="${2:-crates/remote/Dockerfile}"

ROOT_NODE=$(grep -oP 'FROM node:\d+-alpine AS builder' "$ROOT_DOCKERFILE" | sed 's/FROM //;s/ AS builder//')
REMOTE_NODE=$(grep -oP 'FROM node:\d+-alpine AS fe-builder' "$REMOTE_DOCKERFILE" | sed 's/FROM //;s/ AS fe-builder//')

if [ -z "$ROOT_NODE" ]; then
    echo "ERROR: could not find 'FROM node:<N>-alpine AS builder' in $ROOT_DOCKERFILE" >&2
    exit 2
fi

if [ -z "$REMOTE_NODE" ]; then
    echo "ERROR: could not find 'FROM node:<N>-alpine AS fe-builder' in $REMOTE_DOCKERFILE" >&2
    exit 2
fi

if [ "$ROOT_NODE" != "$REMOTE_NODE" ]; then
    echo "MISMATCH: root Dockerfile uses $ROOT_NODE but crates/remote/Dockerfile uses $REMOTE_NODE" >&2
    echo "Fix: update the older one to match." >&2
    exit 1
fi

echo "OK: both Dockerfiles use ${ROOT_NODE}"
exit 0
```

**Notes:**
- `grep -oP 'FROM node:\d+-alpine AS ...'` extracts the exact `node:<N>-alpine` image name for
  each Dockerfile's frontend-builder stage.
- `sed 's/FROM //;s/ AS ...//'` strips the `FROM ` prefix and ` AS <name>` suffix, leaving only
  the image tag (e.g. `node:24-alpine`).
- The script uses exact stage name matching (`AS builder` for root, `AS fe-builder` for remote)
  so it extracts the correct `FROM` line even if other stages use `FROM node:` lines in the future.
- On mismatch, exit 1 with a human-readable diff message. On structural error (line not found),
  exit 2 so CI can distinguish "please fix" from "the script itself broke."
- The `set -euo pipefail` makes the script fail on any unbound variable or failed command.
- The `"${1:-Dockerfile}"` syntax allows overriding paths for testing but defaults to the
  repo-relative paths in CI.

## Allowed moves
- Create EXACTLY one file: `scripts/assert-dockerfile-node-match.sh`.
- Do NOT create, modify, or delete any other file.
- Do NOT change any Dockerfile to make this script pass — the script diagnoses drift; it
  does not fix it.

## STOP triggers
- If file `scripts/assert-dockerfile-node-match.sh` already exists, halt. It means a prior
  session already created this script. Verify its content matches the above and skip to
  Done when; do NOT overwrite.
- If `grep "FROM node:" Dockerfile | wc -l` returns 0, halt — the root Dockerfile has no
  `FROM node:` line (unexpected).
- If `grep "FROM node:" crates/remote/Dockerfile | wc -l` returns 0, halt — the remote
  Dockerfile has no `FROM node:` line (unexpected; Task 001 should have fixed this).

## Manual verification (record in decisions-ledger)

1. Make the script executable:
   ```bash
   chmod +x scripts/assert-dockerfile-node-match.sh
   ```

2. Run it — should pass if Task 001 was applied correctly:
   ```bash
   bash scripts/assert-dockerfile-node-match.sh
   ```
   Expected: `OK: both Dockerfiles use node:24-alpine`, exit 0.

3. Verify it catches a drift (temporarily break it):
   ```bash
   # Temporarily report a different value, then restore
   bash scripts/assert-dockerfile-node-match.sh /dev/stdin crates/remote/Dockerfile <<< "FROM node:20-alpine AS builder" || echo "CAUGHT_MISMATCH"
   ```
   Expected: `CAUGHT_MISMATCH` (script exits 1 on mismatch).

4. Verify it reports structural errors gracefully:
   ```bash
   bash scripts/assert-dockerfile-node-match.sh /dev/null crates/remote/Dockerfile || echo "CAUGHT_ERROR"
   ```
   Expected: `CAUGHT_ERROR` (script exits 2).

## Done when
`WAI_TYPECHECK_CMD="" WAI_TEST_CMD="" bash ~/.claude/wai/scripts/task-gate.sh remote-docker-build-fix 004` exits 0