---
id: "002"
phase: 1
title: Root Dockerfile: pin pnpm@10.25.0 + cross-file coupling comment
status: passed
depends_on: []
parallel: false
conflicts_with: ["001", "004"]
files:
  - Dockerfile
irreversible: false
scope_test: "N/A"
allowed_change: edit
covers_criteria: [SC4]
covers_tests: []
---
## Failing test (write first)

The following shell snippet FAILS on the current codebase (exits non-zero) and PASSES
after this task is applied.

```bash
# FAILS NOW: root Dockerfile installs unpinned pnpm
grep -q 'npm install -g pnpm@' Dockerfile || { echo "FAIL: root Dockerfile does not pin pnpm version"; exit 1; }
# FAILS NOW: root Dockerfile missing cross-file coupling comment
grep -q 'crates/remote/Dockerfile' Dockerfile || { echo "FAIL: root Dockerfile missing cross-file coupling comment"; exit 1; }
echo "PASS: root Dockerfile pin + comment applied"
```

## Change

### File: `Dockerfile` (repo root)

**Anchor — the `RUN npm install -g pnpm` line at line 28.** Two changes: (a) add a
version pin so the installed pnpm version matches `package.json`'s `packageManager`
field, and (b) add a cross-file coupling comment pointing at the remote Dockerfile.

Before:
```dockerfile
# Install pnpm and dependencies
RUN npm install -g pnpm && pnpm install
```

After:
```dockerfile
# Install pnpm and dependencies
# keep in sync with crates/remote/Dockerfile's fe-builder pnpm version
RUN npm install -g pnpm@10.25.0 && pnpm install
```

**Notes:**
- `pnpm@10.25.0` is the version pinned in `package.json:53`. If the `packageManager` field
  changes, this line must be updated too. The cross-file coupling comment flags the dependency.
- The root Dockerfile uses `npm install -g pnpm` (not corepack), so we pin via the npm registry
  tag directly. No ARG is needed here because the root Dockerfile is not built via
  docker-compose (it has no compose file that could pass build args).
- Do NOT change any other line in this file. The `FROM node:24-alpine` line stays as-is
  (it is already correct).
- Do NOT add corepack to this Dockerfile — it is a different build strategy and switching
  package managers is out of scope.

## Allowed moves
- Edit only lines 28 and the comment above it (lines ~27–28).
- Do NOT change the `FROM`, `COPY`, `RUN cargo build`, `RUN npm run`, `HEALTHCHECK`,
  `ENTRYPOINT`, or `CMD` lines.
- Do NOT add or remove any `ARG` or `ENV` lines.

## STOP triggers
- If line 28 does NOT contain `RUN npm install -g pnpm && pnpm install`, halt and report
  the actual line content.
- If `"packageManager": "pnpm@10.25.0"` is NOT in `package.json`, halt — the version
  to pin has changed. Update this task's Before/After to match the actual value.
- If any file outside `files:` changes, halt.

## Manual verification (record in decisions-ledger)

1. Verify the edit is correct:
   ```bash
   grep "npm install -g pnpm@10.25.0" Dockerfile
   ```
   Expected: exact match on the After line.

2. Verify the pin matches package.json:
   ```bash
   ACTUAL=$(grep pnpm@ Dockerfile | grep -oP 'pnpm@[\d.]+' | head -1)
   EXPECTED=$(jq -r .packageManager package.json)
   [ "$ACTUAL" = "$EXPECTED" ] && echo "MATCH" || echo "MISMATCH: Dockerfile=$ACTUAL package.json=$EXPECTED"
   ```
   Expected: MATCH.

3. Verify the cross-file comment references the remote Dockerfile:
   ```bash
   grep -q "crates/remote/Dockerfile" Dockerfile
   ```
   Expected: exit 0.

## Done when
`WAI_TYPECHECK_CMD="" WAI_TEST_CMD="" bash ~/.claude/wai/scripts/task-gate.sh remote-docker-build-fix 002` exits 0