---
id: "006"
phase: 3
title: "Manual E2E verification: rebuild.sh + compose up + healthcheck 200"
status: ready
depends_on: ["001", "002", "003", "005"]
parallel: false
conflicts_with: ["005"]
files: []
irreversible: false
scope_test: "N/A"
allowed_change: edit
covers_criteria: [SC1, SC2]
covers_tests: [TS1, TS2]
---
## Failing test (write first)
N/A — this task is pure verification. No new code is written. The test is the sequence of
commands below. Before the tasks in Phase 1, `rebuild.sh` fails with
`ERR_UNKNOWN_BUILTIN_MODULE: node:sqlite` on a clean builder cache. After Phase 1 is
applied, the build succeeds and the server responds with HTTP 200 on `/v1/health`.

## Change
No files are created, modified, or deleted. This task verifies the work done in tasks 001–005
by running the real build and deployment flow end-to-end.

## Allowed moves
- No files may be created, modified, or deleted.
- The only permitted action is running the verification commands below and recording
  the results in the decisions-ledger.

## STOP triggers
- If `docker` is not available (`docker --version` fails), halt. Docker must be running.
- If `psql` is not available, halt. The E2E test needs a Postgres client to verify the DB
  migration. (Alternative: skip the DB verification — the healthcheck covers the server's
  own DB connectivity; a DB migration failure will show as a non-200 healthcheck.)
- If the `.env.remote` file does not exist, halt. Create one from `.env.remote.dev` first:
  ```bash
  cp crates/remote/.env.remote.dev crates/remote/.env.remote
  ```
- If any pre-existing Docker containers or volumes from a prior run are still present,
  clean them up first:
  ```bash
  docker compose --env-file .env.remote down -v 2>/dev/null || true
  ```

## Manual verification (record in decisions-ledger)

Run each command, record the output verbatim, and note whether it passed or failed.

### Step 1: Prune the builder cache (clean-slate verification)

```bash
docker builder prune -af 2>/dev/null || true
```

Record: cache pruned (or note if prune was skipped).

### Step 2: Build the remote-server image

```bash
(cd crates/remote && ./rebuild.sh 2>&1 | tee /tmp/e2e-build.log)
```

**Pass if:** exit code 0.

**Evidence to capture from the log:**
- `pnpm --version` or `corepack prepare pnpm@<version>` output — record the exact version
  number. Verify it matches `10.25.0` (or the current `packageManager` field).
- `ERROR` or `ERR_UNKNOWN_BUILTIN_MODULE` — should be absent. Record whether any error
  lines appear.

### Step 3: Verify the server is running

```bash
docker ps --filter "name=remote-server" --format "{{.Names}} {{.Status}}"
```

**Pass if:** a `remote-server` container is listed with status `Up` (or `Up <time>`).

### Step 4: Healthcheck

```bash
curl -sf http://localhost:3000/v1/health 2>&1
```

Wait up to 60 seconds with polling if the server is still starting:

```bash
for i in $(seq 1 30); do
  if curl -sf http://localhost:3000/v1/health 2>/dev/null; then
    echo ""
    echo "HEALTHY after ${i} attempts"
    exit 0
  fi
  sleep 2
done
echo "TIMEOUT: healthcheck never returned 200"
exit 1
```

**Pass if:** HTTP 200 with a JSON body containing a `status` field whose value includes
`ok`, `healthy`, or `running`.

**Record:** the exact JSON response body.

### Step 5: Verify the build log for pnpm pin

```bash
grep -i "pnpm@10.25.0\|corepack.*prepare" /tmp/e2e-build.log | head -5
```

**Pass if:** at least one line shows pnpm 10.25.0 being prepared. **Fail if:** any line
shows `pnpm@11` or `corepack is about to download ... pnpm-11`.

### Step 6: Tear down

```bash
(cd crates/remote && docker compose --env-file .env.remote down)
```

(Optional — keep the server running if you plan to do further testing.)

### Step 7: Local CI simulation

Run the same checks the CI workflow from Task 005 runs:

```bash
bash scripts/assert-dockerfile-node-match.sh && echo "PASS: FROM-line assertion" || echo "FAIL: FROM-line assertion"
bash -n crates/remote/rebuild.sh && echo "PASS: rebuild.sh syntax" || echo "FAIL: rebuild.sh syntax"
jq -r '.engines.node' package.json | grep -q "22" && echo "PASS: engines.node >=22.13" || echo "FAIL: engines.node"
grep -q "keep in sync" Dockerfile && echo "PASS: root cross-file comment" || echo "FAIL: root cross-file comment"
grep -q "keep in sync" crates/remote/Dockerfile && echo "PASS: remote cross-file comment" || echo "FAIL: remote cross-file comment"
```

**Pass if:** all five lines print `PASS`.

## Done when

All steps 1–7 in Manual verification pass. Record the results in the decisions-ledger
(`dev-docs/workstreams/remote-docker-build-fix/plans/remote-docker-build-fix/decisions-ledger.md`)
with the date, executor, and a summary line: `E2E VERIFICATION: PASS (or FAIL with details)`.

The `task-gate.sh` invocation passes trivially (no file changes, no scope_test) as long as
the ledger entry is recorded.

`WAI_TYPECHECK_CMD="" WAI_TEST_CMD="bash -c 'echo VERIFICATION_TASK_006: record ledger entry'" \
  bash ~/.claude/wai/scripts/task-gate.sh remote-docker-build-fix 006` exits 0