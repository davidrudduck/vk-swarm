---
id: "503"
phase: 5
title: Add npm runtime-vuln CI gate (check-npm-runtime-vulns + overrides)
status: passed
depends_on: []
parallel: false
conflicts_with: []
files:
  - scripts/check-npm-runtime-vulns.mjs
  - package.json
  - pnpm-lock.yaml
irreversible: false
scope_test: "N/A"
allowed_change: mixed
covers_criteria: [SC7]
---
## Failing test (write first)

N/A — this is a CI-gate script, not unit-testable hermetically (it shells out to `pnpm audit`, which
hits the live npm advisory registry). Verification is by the `## Manual verification` section below
(seeded-advisory check) plus the automated clean-pass in `## Done when` (`WAI_TEST_CMD` runs the gate
and asserts exit 0 on the current tree).

## Change

For each file in `files:`:

### File 1 — `scripts/check-npm-runtime-vulns.mjs` (CREATE)

Create this file verbatim from upstream `vibe-kanban/scripts/check-npm-runtime-vulns.mjs` (cited
source). Full content:

```javascript
#!/usr/bin/env node
// Fails CI if any high/critical advisory references one of the runtime-reachable
// modules listed in BLOCKED. Build-chain-only advisories (sucrase, sentry plugins,
// etc.) are not blocked here — they're tracked by `pnpm audit` itself.

import { execSync } from 'node:child_process';

// Runtime-reachable modules that ship in the production bundle. Build-only
// modules (sucrase, sentry-plugin, etc.) are intentionally excluded.
//
// `lodash` / `lodash-es` are NOT in this list: the open advisories patch at
// "^4.18.0" but no such release exists yet upstream. The vulnerable
// `_.template` path is not invoked by anything in this repo. Revisit when
// upstream ships a patched release.
const BLOCKED = new Set(['preact', 'fast-uri', 'devalue']);

const BLOCKED_SEVERITY = new Set(['high', 'critical']);

let raw;
try {
  raw = execSync('pnpm audit --prod --json', { encoding: 'utf8' });
} catch (err) {
  // pnpm audit exits non-zero when vulnerabilities are found, but still prints
  // JSON to stdout. Use that.
  raw = err.stdout?.toString() ?? '';
  if (!raw.trim()) {
    console.error('pnpm audit produced no output:', err.message);
    process.exit(2);
  }
}

let report;
try {
  report = JSON.parse(raw);
} catch (err) {
  console.error('Failed to parse pnpm audit JSON:', err.message);
  process.exit(2);
}

// Security gate: fail closed if the audit JSON does not have the expected
// `advisories` field. Otherwise a future schema change would let this gate
// pass with zero offenders simply because the parser couldn't find them.
if (
  !('advisories' in report) ||
  typeof report.advisories !== 'object' ||
  report.advisories === null
) {
  console.error(
    'Unsupported pnpm audit JSON schema: missing or non-object `advisories`.',
  );
  process.exit(2);
}

const advisories = report.advisories;
const offenders = [];

for (const adv of Object.values(advisories)) {
  if (!BLOCKED_SEVERITY.has(adv.severity)) continue;
  if (!BLOCKED.has(adv.module_name)) continue;
  const path =
    adv.findings?.[0]?.paths?.[0] ?? '(unknown path)';
  offenders.push({
    module: adv.module_name,
    severity: adv.severity,
    title: adv.title,
    vulnerable: adv.vulnerable_versions,
    patched: adv.patched_versions,
    path,
  });
}

if (offenders.length === 0) {
  console.log(
    '✅ No high/critical advisories on runtime-reachable modules ' +
      `(${[...BLOCKED].join(', ')}).`,
  );
  process.exit(0);
}

console.error(
  `❌ ${offenders.length} blocked advisory${offenders.length === 1 ? '' : 'ies'} found:\n`,
);
for (const o of offenders) {
  console.error(`  - ${o.module} ${o.vulnerable} [${o.severity}]`);
  console.error(`      ${o.title}`);
  console.error(`      patched: ${o.patched}`);
  console.error(`      via: ${o.path}\n`);
}
console.error(
  'Add a pnpm.overrides entry in root package.json to force a patched version,\n' +
    'then re-run `pnpm install`.',
);
process.exit(1);
```

### File 2 — `package.json` (EDIT) — two surgical changes

**(a) Wire the gate into the `lint` script** (upstream runs it as the last clause of `lint`;
`vibe-kanban/package.json:13`). The fork's `lint` differs, so append the same clause to it.

- **Anchor:** the `"lint"` script line (package.json:17).
- **Before:**
```json
    "lint": "pnpm run frontend:lint && pnpm run backend:lint",
```
- **After:**
```json
    "lint": "pnpm run frontend:lint && pnpm run backend:lint && node scripts/check-npm-runtime-vulns.mjs",
```

**(b) Merge upstream's runtime overrides into the existing `pnpm.overrides` block.** The fork ALREADY
has an `overrides` block (`@tanstack/db`, `@codemirror/state`) — this is a **merge, not a replace**.
Add upstream's three runtime pins (`vibe-kanban/package.json:73-79`) alongside the existing two.

- **Anchor:** the `"pnpm"` block (package.json:54-59).
- **Before:**
```json
  "pnpm": {
    "overrides": {
      "@tanstack/db": "^0.6.4",
      "@codemirror/state": "^6.6.0"
    }
  }
```
- **After:**
```json
  "pnpm": {
    "overrides": {
      "@tanstack/db": "^0.6.4",
      "@codemirror/state": "^6.6.0",
      "preact@<10.27.3": "^10.27.3",
      "devalue@<5.6.4": "^5.6.4",
      "fast-uri@<3.1.2": "^3.1.2"
    }
  }
```

After editing, run `pnpm install` so the lockfile reflects the new overrides (otherwise CI/install
will flag a drift, and the overrides protect nothing until the lockfile pins the patched versions).
`pnpm-lock.yaml` is in `files:` precisely so this regeneration is allowed by the gate's "no file
outside `files:`" invariant: if the three pinned modules (preact/devalue/fast-uri) are present in the
fork's tree the lockfile diff is expected; if they are inert the lockfile is unchanged (a listed-but-
unmodified file is harmless). Either way, commit the resulting `pnpm-lock.yaml`.

## Allowed moves

- Create `scripts/check-npm-runtime-vulns.mjs` verbatim from upstream.
- Append ` && node scripts/check-npm-runtime-vulns.mjs` to the `lint` script — ONLY that one line.
- Add ONLY the three runtime override entries to the existing `pnpm.overrides` block; do NOT remove or
  reorder the existing `@tanstack/db` / `@codemirror/state` entries.
- Do NOT add a new GitHub Actions workflow. The fork has no `test.yml`; upstream wires the gate into
  the `lint` npm script, and so do we (mirror upstream's wiring location).
- Do NOT touch any other `package.json` field, `frontend/package.json`, or any other script.

## STOP triggers

- The `"lint"` line is not exactly `"pnpm run frontend:lint && pnpm run backend:lint",`.
- The `pnpm.overrides` block does not contain `@tanstack/db` / `@codemirror/state` (means the fork
  drifted — STOP and re-anchor).
- `node scripts/check-npm-runtime-vulns.mjs` exits non-zero on the **current** tree (means a real
  advisory hit, OR the `BLOCKED` set matches a present vulnerable module — investigate before claiming
  done; see the BLOCKED-set caveat below).
- The change would require editing any file not in `files:` (the three listed files —
  `scripts/check-npm-runtime-vulns.mjs`, `package.json`, `pnpm-lock.yaml` — are the only ones the
  `pnpm install` regeneration may touch; anything else means STOP).

## Sibling alignment

No sibling **security/advisory** gate exists in the fork's `scripts/` (`ls scripts/ | grep -i vuln` →
absent). The nearest sibling by location is `scripts/check-i18n.sh` (read it) — but that is a **bash**
i18n-key-regression checker (literal-string counts, locale key consistency), a wholly different
concern, not wired into the fork's npm `lint`/`check` scripts, and sharing no logic with an
advisory gate. Justified divergence: the new file is a verbatim forward-port of upstream's Node ESM
script, not an adaptation of the bash sibling.

This is a faithful verbatim forward-port of upstream `vibe-kanban/scripts/check-npm-runtime-vulns.mjs`
and the `pnpm.overrides` pins at `vibe-kanban/package.json:73-79`. The only divergence from upstream is
the **wiring location**: upstream appends the gate to a `lint` script that also runs
`check-unused-i18n-keys.mjs` and other checks; the fork's `lint` is `frontend:lint && backend:lint`, so
we append to that instead.

## BLOCKED-set caveat (record in decisions-ledger; flag to implementer)

Upstream's `BLOCKED = {preact, fast-uri, devalue}` tracks **upstream's** dependency tree. Those three
modules may be **inert in the fork's tree** (not present, or not vulnerable), in which case the gate
always passes and provides no protection until the fork actually depends on a vulnerable version.
**Port the script verbatim as instructed, but the implementer MUST validate `BLOCKED` against the
fork's actual `pnpm audit --prod --json` output** and, if the fork's runtime-reachable surface differs,
adjust `BLOCKED` to the fork's real high/critical runtime advisories (recording the change in the
decisions-ledger). The three override pins are harmless if the modules are absent (overrides only bind
when the package resolves).

## Manual verification (record in decisions-ledger)

1. **Clean tree passes:** `node scripts/check-npm-runtime-vulns.mjs` → exits `0` and prints
   `✅ No high/critical advisories on runtime-reachable modules (preact, fast-uri, devalue).`
   (Requires `pnpm install` to have run first so `pnpm audit` resolves the workspace.)
2. **Gate fires on a seeded advisory:** temporarily edit the `BLOCKED` set in the script to include a
   module that the fork's `pnpm audit --prod --json` currently reports as `high`/`critical` (run
   `pnpm audit --prod --json | node -e "…"` to find one, or inject a known-vulnerable dep into a
   throwaway branch). Re-run `node scripts/check-npm-runtime-vulns.mjs` → exits `1` and prints
   `❌ N blocked advisory…` with the offending module. **Revert the seed afterward.**
3. **`lint` invokes the gate:** `pnpm run lint` runs the gate as its final clause (observe the
   `✅`/`❌` line in output).

## Done when

`WAI_TYPECHECK_CMD="true" WAI_TEST_CMD="node scripts/check-npm-runtime-vulns.mjs" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-node-foundations 503` exits 0
