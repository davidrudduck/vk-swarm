---
id: "005"
phase: 2
title: Create .github/workflows/remote-hive-build.yml CI job
status: passed
depends_on: ["004"]
parallel: false
conflicts_with: ["004", "006"]
files:
  - .github/workflows/remote-hive-build.yml
  - .github/workflows/pre-release.yml
irreversible: false
scope_test: "N/A"
allowed_change: create
covers_criteria: [SC6]
covers_tests: [TS6]
---
## Sibling alignment: `.github/workflows/pre-release.yml`

Listed in `files:` per `wai-plan-lint.sh` advisory. `pre-release.yml` is a
`workflow_dispatch`-only release pipeline with `concurrency`, `permissions: contents: write`,
`env:` block pinning NODE_VERSION=22 and PNPM_VERSION=10.13.1. We adopt its conventions
(`runs-on: ubuntu-latest`, `concurrency`, `actions/checkout@v4`) in
`remote-hive-build.yml`. One divergence to record: `pre-release.yml` uses
`permissions: contents: write` (it publishes releases); `remote-hive-build.yml` uses
`permissions: contents: read` (it's a CI check, no write needed). This is intentional and
correct per the GitHub Actions least-privilege principle. Record this justification in the
decisions ledger.

## Failing test (write first)
N/A — new file, no existing test. The CI job is its own validation: when it is committed
and a PR is opened touching the path globs, the `remote-hive-build` check appears in the
PR checks list. If the Dockerfile build is broken, the check is red.

## Change

### File: `.github/workflows/remote-hive-build.yml` (NEW)

Create the file with the following content. Follows the existing CI conventions in this
repo: `runs-on: ubuntu-latest`, `permissions: contents: read`, `actions/checkout@v4`.

```yaml
name: Remote Hive Build

on:
  pull_request:
    paths:
      - 'crates/remote/**'
      - 'Dockerfile'
      - 'package.json'
      - 'pnpm-lock.yaml'
      - 'pnpm-workspace.yaml'
      - 'scripts/assert-dockerfile-node-match.sh'
      - '.github/workflows/remote-hive-build.yml'
  workflow_dispatch:

concurrency:
  group: remote-hive-build-${{ github.ref }}
  cancel-in-progress: true

permissions:
  contents: read

jobs:
  assert-dockerfile-match:
    name: Assert Dockerfile node version match
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: bash scripts/assert-dockerfile-node-match.sh

  build-remote:
    name: Build remote-server (simulated)
    runs-on: ubuntu-latest
    needs: assert-dockerfile-match
    steps:
      - uses: actions/checkout@v4
      - name: Verify rebuild.sh syntax
        run: |
          bash -n crates/remote/rebuild.sh
          echo "rebuild.sh syntax OK"
      - name: Verify PNPM_VERSION plumbing
        run: |
          EXPECTED_VERSION=$(jq -r .packageManager package.json | sed 's/pnpm@//')
          echo "Expected pnpm version from package.json: $EXPECTED_VERSION"
          COMPOSE_VERSION=$(grep -oP 'PNPM_VERSION: .*\{\w+:-([\d.]+)\}' crates/remote/docker-compose.yml | grep -oP '[\d.]+' | tail -1)
          echo "Compose fallback version: $COMPOSE_VERSION"
          [ "$COMPOSE_VERSION" = "$EXPECTED_VERSION" ] || {
            echo "MISMATCH: compose fallback ($COMPOSE_VERSION) != package.json ($EXPECTED_VERSION)"
            exit 1
          }
          echo "PNPM_VERSION plumbing OK"
      - name: Verify engines.node constraint
        run: |
          ENGINE=$(jq -r '.engines.node' package.json)
          echo "engines.node: $ENGINE"
          MAJOR=$(echo "$ENGINE" | grep -oP '\d+' | head -1)
          [ "$MAJOR" -ge 22 ] || { echo "engines.node too permissive: $ENGINE"; exit 1; }
      - name: Verify cross-file coupling comments exist
        run: |
          grep -q "keep in sync" Dockerfile || { echo "Missing cross-file comment in root Dockerfile"; exit 1; }
          grep -q "keep in sync" crates/remote/Dockerfile || { echo "Missing cross-file comment in crate/remote Dockerfile"; exit 1; }
```

**Notes:**
- Two jobs: `assert-dockerfile-match` (runs the script from Task 004) and `build-remote`
  (verifies the non-Docker parts of the fix: rebuild.sh syntax, PNPM_VERSION plumbing,
  engines.node constraint, cross-file coupling comments).
- The `build-remote` job uses `needs: assert-dockerfile-match` so the FROM-line check runs
  first and fails fast, saving CI minutes.
- The build itself is NOT run in CI (that would require a Docker daemon, which increases
  CI minutes significantly). The CI job validates the *plumbing* (version consistency,
  script syntax, engine constraints). The actual end-to-end smoke test is in Task 006
  (manual verification) because it requires a running Docker daemon and takes several
  minutes — better suited to a pre-commit hook or a scheduled job than a per-PR check.
- `workflow_dispatch` allows manual triggering for debugging.
- The concurrency group cancels in-progress runs on the same ref (avoids CI queue buildup).

## Allowed moves
- Create EXACTLY one file: `.github/workflows/remote-hive-build.yml`.
- Do NOT create, modify, or delete any other `.github/workflows/` file.
- Do NOT modify any existing workflow to reference this one.

## STOP triggers
- If file `.github/workflows/remote-hive-build.yml` already exists, halt. Verify its
  content matches the above and skip to Done when; do NOT overwrite.
- If the `scripts/assert-dockerfile-node-match.sh` from Task 004 does not exist at the
  expected path, halt. Task 004 must be completed first.
- If `jq` is not detected in the CI runner env, halt — the CI job depends on `jq` being
  available in the `ubuntu-latest` image (it is pre-installed).

## Manual verification (record in decisions-ledger)

1. Verify the workflow YAML parses:
   ```bash
   python3 -c "import yaml; yaml.safe_load(open('.github/workflows/remote-hive-build.yml'))" && echo "VALID"
   ```
   Expected: VALID.

2. Verify the path triggers cover the documented files:
   ```bash
   grep -A 10 "pull_request:" .github/workflows/remote-hive-build.yml | grep -E "crates/remote|Dockerfile|package.json|pnpm-lock|pnpm-workspace|assert-dockerfile|remote-hive-build"
   ```
   Expected: at least 7 matching lines (one per path glob + `pull_request:` itself).

3. Verify the concurrency group uses `cancel-in-progress: true`:
   ```bash
   grep "cancel-in-progress: true" .github/workflows/remote-hive-build.yml
   ```
   Expected: one match.

4. Run the CI checks locally (simulate what GitHub Actions would do):
   ```bash
   bash scripts/assert-dockerfile-node-match.sh
   bash -n crates/remote/rebuild.sh && echo "syntax OK"
   EXPECTED_VERSION=$(jq -r .packageManager package.json | sed 's/pnpm@//')
   COMPOSE_VERSION=$(grep -oP 'PNPM_VERSION: .*\{\w+:-([\d.]+)\}' crates/remote/docker-compose.yml | grep -oP '[\d.]+' | tail -1)
   [ "$COMPOSE_VERSION" = "$EXPECTED_VERSION" ] && echo "PNPM_VERSION OK" || echo "MISMATCH"
   ENGINE=$(jq -r '.engines.node' package.json)
   MAJOR=$(echo "$ENGINE" | grep -oP '\d+' | head -1)
   [ "$MAJOR" -ge 22 ] && echo "engines.node OK" || echo "engines.node FAIL"
   grep -q "keep in sync" Dockerfile && echo "root comment OK"
   grep -q "keep in sync" crates/remote/Dockerfile && echo "remote comment OK"
   ```
   Expected: all lines print `... OK` (no FAIL, no MISMATCH).

## Done when
`WAI_TYPECHECK_CMD="" WAI_TEST_CMD="" bash ~/.claude/wai/scripts/task-gate.sh remote-docker-build-fix 005` exits 0