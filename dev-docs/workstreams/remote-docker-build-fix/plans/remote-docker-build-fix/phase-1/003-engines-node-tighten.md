---
id: "003"
phase: 1
title: "package.json: tighten engines.node from >=18 to >=22.13"
status: passed
depends_on: []
parallel: false
conflicts_with: []
files:
  - package.json
irreversible: false
scope_test: "N/A"
allowed_change: edit
covers_criteria: [SC5]
covers_tests: [TS5]
---
## Failing test (write first)
N/A — covered by existing tests: no unit test applies to `engines` field semantics. The
verification is a throwaway container test (see Manual verification below). Before this
task, `engines.node = ">=18"` is too permissive — it accepts Node 20, which lacks
`node:sqlite` and would crash pnpm 11.x. After, any attempt to install on Node <22.13
fails with `EBADENGINE`.

## Change

### File: `package.json` (repo root)

**Anchor — the `engines` block at lines 49–52 (under the `"resolutions"` closing brace).**

Before:
```json
  "engines": {
    "node": ">=18",
    "pnpm": ">=8"
  },
```

After:
```json
  "engines": {
    "node": ">=22.13",
    "pnpm": ">=8"
  },
```

**Notes:**
- `22.13` is the minimum version pnpm 11.x requires (`node:sqlite` builtin added in Node 22.5+;
  pnpm 11.x explicitly warns "requires at least Node.js v22.13"). pnpm 10.x (our current pin)
  is compatible with 22.13+, so this is a forward-compatible floor.
- `engine-strict=true` in `.npmrc` is already set. This means `pnpm install` will fail with
  `EBADENGINE` if the Node runtime is <22.13. The field is currently advisory without
  `engine-strict`, but with `.npmrc:1` it is enforced.
- The package is `private: true` (line 5), so tightening engines does not affect any
  downstream npm consumers — it is an internal contract only.
- Do NOT change `"pnpm": ">=8"` — pnpm's own version constraint stays as-is.
- Do NOT change the `packageManager` field or any other field.

## Allowed moves
- Edit only the `"node"` value on line 50 from `">=18"` to `">=22.13"`.
- Do NOT change any other key or value in `package.json`.
- Do NOT add, remove, or reorder any field.

## STOP triggers
- If line 50 does NOT read `"node": ">=18",`, halt and report the actual line content.
- If `"private": true` is NOT on line 5, halt. This is unexpected for a private package.
- If any file outside `files:` changes, halt.

## Manual verification (record in decisions-ledger)

1. Verify the edit is correct:
   ```bash
   jq -r '.engines.node' package.json
   ```
   Expected: `>=22.13`

2. Verify `engine-strict` is still on:
   ```bash
   head -1 .npmrc
   ```
   Expected: `engine-strict=true`

3. Throwaway container test — prove the engine constraint rejects a too-old Node:
   ```bash
   docker run --rm -v "$PWD":/repo -w /repo node:20-alpine \
     sh -c 'corepack enable && corepack prepare pnpm@10.25.0 --activate && pnpm --version'
   ```
   Expected: `pnpm --version` outputs `10.25.0` (pnpm install itself succeeds because
   `engines` governs dependent packages, not the package manager runtime).
   ```bash
   docker run --rm -v "$PWD":/repo -w /repo node:20-alpine \
     sh -c 'corepack enable && corepack prepare pnpm@10.25.0 --activate && pnpm install --frozen-lockfile 2>&1'
   ```
   Expected: exits non-zero with `EBADENGINE: Unsupported engine` — proves that the
   engine strict setting rejects a too-old Node at the dependency install step.

4. Verify the package.json parses cleanly:
   ```bash
   jq . package.json > /dev/null && echo "VALID"
   ```
   Expected: VALID.

## Done when
`WAI_TYPECHECK_CMD="" WAI_TEST_CMD="" bash ~/.claude/wai/scripts/task-gate.sh remote-docker-build-fix 003` exits 0