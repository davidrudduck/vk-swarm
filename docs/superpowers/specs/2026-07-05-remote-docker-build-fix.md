---
doc_type: spec
status: active
workstream: remote-docker-build-fix
change_kind: bugfix
---

# remote-docker-build-fix — Stand up the remote/hive image end-to-end

> **Bugfix scope.** This doc captures the *intent* and non-negotiables for unblocking the
> `crates/remote` Docker build. The root cause analysis is in
> [`docs/superpowers/specs/2026-07-05-remote-docker-build-root-cause.md`](./2026-07-05-remote-docker-build-root-cause.md)
> (sibling analysis note — to be created at `/wai:spec` time). This spec does not restate the
> diagnosis; it fixes the outcome.

## Intent (what / why)

When a user runs `./crates/remote/rebuild.sh` (or `docker compose --env-file .env.remote build
remote-server`) against a clean builder cache, the `fe-builder` stage fails with
`ERR_UNKNOWN_BUILTIN_MODULE: No such built-in module: node:sqlite` because:

1. `crates/remote/Dockerfile:5` pins `FROM node:20-alpine`, which lacks the `node:sqlite` builtin
   that current pnpm versions hard-require.
2. The Dockerfile `corepack enable`s but never `corepack prepare`s the pnpm version pinned in
   `package.json` (`pnpm@10.25.0`), so corepack is free to fetch pnpm 11.x from npm — a version
   that is itself incompatible with the available Node runtime.

The hive/remote image has never actually been stood up end-to-end; this is the first real test
and it fails. The node image (`Dockerfile` at repo root) was migrated to `node:24-alpine` and
works; the remote one was missed in that drift. We want a clean reproducible build of
`crates/remote/Dockerfile` and a green healthcheck on `docker compose up` with the
`.env.remote.dev` profile — so the hive can be deployed in production for real.

## Users / who is affected

- **Primary:** the operator trying to stand up the remote/hive server in production. Today they
  get a build failure with no obvious cause, and the documented `.env.remote.dev` profile does
  not produce a working container.
- **Secondary:** any future maintainer who runs the same `./rebuild.sh` flow and hits the same
  failure, because the two Dockerfiles (root + `crates/remote`) silently drift in node-base
  version with nothing catching the divergence.
- **CI:** the missing smoke test means a future drift to either Dockerfile goes unnoticed in PR
  review. The "tested it once, works" assumption has not been re-verified.

## Success criteria

- SC1: `docker compose --env-file .env.remote build remote-server` exits 0 from a clean builder
  cache on a Linux x86_64 host. → US1
- SC2: `docker compose --env-file .env.remote up -d remote-server` brings the container up and
  `curl -s http://localhost:3000/v1/health` returns HTTP 200 with a JSON body whose
  `status` field is `ok` (or the documented equivalent for this app's healthcheck contract). → US1
- SC3: The `fe-builder` stage base image in `crates/remote/Dockerfile` matches the node base image
  used by the repo-root `Dockerfile` (both `node:24-alpine` or both `node:<same>-alpine`). The
  matching is encoded by a comment cross-referencing the other file; the actual equality is
  enforced by a small grep-based assertion in CI. → US2
- SC4: The pnpm version actually executed by the `fe-builder` stage equals the
  `packageManager` value in `package.json` (currently `pnpm@10.25.0`). This is enforced by a
  `corepack prepare pnpm@<pinned> --activate` step in the Dockerfile (or equivalent explicit
  `npm install -g pnpm@<pinned>`) and verified by `pnpm --version` being captured in the build
  log. → US2
- SC5: `engines.node` in `package.json` is tightened from `">=18"` to `">=22.13"` so that any
  future drift to a too-old base image fails fast at `pnpm install` with a clear engine error,
  not deep inside pnpm with `ERR_UNKNOWN_BUILTIN_MODULE`. → US3
- SC6: A CI job (`.github/workflows/remote-hive-build.yml` or equivalent) runs
  `./crates/remote/rebuild.sh` end-to-end on every PR touching `crates/remote/**`,
  `Dockerfile`, `package.json`, `pnpm-lock.yaml`, or the root `pnpm-workspace.yaml`, and fails
  the PR on non-zero exit. → US4

## Test strategy

- TS1: From a clean builder cache, run `docker compose --env-file .env.remote build
  --no-cache remote-server` and assert exit code 0. Capture the build log; grep it for
  `pnpm --version` output and assert it matches the `package.json` `packageManager` field.
- TS2: After a successful build, run `docker compose --env-file .env.remote up -d
  remote-server` and poll `http://localhost:3000/v1/health` for up to 60 seconds; assert HTTP
  200 and a JSON body whose `status` is healthy.
- TS3: Grep the build context for the `fe-builder` `FROM` line and assert it equals the
  `node:` major version of the repo-root `Dockerfile` `builder` stage. Encode as a shell
  assertion script runnable in CI and locally.
- TS4: Run `pnpm --version` inside a one-shot `docker run --rm node:24-alpine corepack enable
  && corepack prepare pnpm@10.25.0 --activate && pnpm --version` and assert the output is
  `10.25.0` (or the current `packageManager` value). This proves the pin is honored
  independent of any Dockerfile.
- TS5: Run `pnpm install --frozen-lockfile` locally (or in a throwaway container) with the
  pinned Node 24 base and assert no `ERR_UNKNOWN_BUILTIN_MODULE` or "requires at least Node.js
  v22.13" warnings appear.
- TS6: CI workflow file lints and is path-triggered on the documented file globs. The job
  exercises `rebuild.sh` end-to-end; it fails the PR on any non-zero exit.

## User stories

- **US1:** As the operator standing up the hive in production, when I run
  `./crates/remote/rebuild.sh` from a clean state, I get a working container with a green
  healthcheck on port 3000 — no `ERR_UNKNOWN_BUILTIN_MODULE`, no manual Dockerfile edits.
- **US2:** As a future maintainer changing either Dockerfile, when I bump the node base image,
  the matching constraint makes it impossible to ship the two files out of sync silently.
- **US3:** As a maintainer, when I introduce a `package.json` engine incompatibility, the
  install step fails with a clear engine error, not an opaque crash inside pnpm internals.
- **US4:** As a reviewer, when I open a PR that drifts the remote build, CI surfaces the
  failure on the PR — I don't have to discover it on a production deploy.

## Constraints

- **No behaviour change beyond the build environment.** This is a build-system fix. The
  runtime behaviour of the remote/hive server is unchanged: same image content, same ports, same
  env vars, same healthcheck contract. The user-facing API is bit-for-bit identical.
- **`engine-strict=true` in `.npmrc` stays on.** Tightening `engines.node` is *compatible* with
  `engine-strict=true`; the two are independent levers (the former constrains npm, the latter
  constrains pnpm via the pinned base).
- **No new dependencies, no new tools.** The fix is a Dockerfile base bump + a `corepack
  prepare` line + a `package.json` engines bump + a CI workflow. No new packages are added to
  `pnpm-lock.yaml` (other than what corepack already fetches as `pnpm@10.25.0`).
- **`docker compose --env-file .env.remote` flow is the source of truth.** The fix must work
  with the existing `rebuild.sh` script, the existing `docker-compose.yml`, and the existing
  `.env.remote.dev` profile. No new env-var conventions are introduced.
- **Both Dockerfiles stay consistent.** The node base image in `crates/remote/Dockerfile:5` and
  the node base image in the repo-root `Dockerfile:2` are kept on the same major version. The
  version is documented in a comment in each file cross-referencing the other.
- **`packageManager` pin is honored, not bypassed.** The fix uses `corepack prepare pnpm@<pinned>
  --activate` (or `npm install -g pnpm@<pinned>`) so the pin in `package.json` is enforced, not
  advisory. Corepack is not told to fall back to its own default under any circumstance.

## Out of scope

- Changing the runtime base image (`debian:bookworm-slim`) — the build failure is in
  `fe-builder`, not `runtime`.
- Changing the Rust toolchain image (`rust:1.89-slim-bookworm`) — also unaffected.
- Adding a hot-reload dev container for the `remote-frontend` outside Docker. The dev-loop
  question for the remote-frontend is tracked in a separate workstream; this fix only restores
  the *build* of the existing Dockerfile.
- Investigating why the `crates/remote` frontend subagent previously chose `node:20-alpine` over
  `node:24-alpine` in the first place. The fix makes the two Dockerfiles match; the *why* of the
  drift is for a separate retrospective.
- Optimizing build time or image size. The `pnpm` cache mount is preserved as-is.
- Changing the `pnpm` major version. We pin to the version already in `package.json`
  (`pnpm@10.25.0`); bumping to pnpm 11.x is a separate decision and is blocked by Node 22.13+,
  which Node 24 satisfies.

## Approach

The fix is the minimal, layered set of edits that makes the remote build identical in environment
to the root build, and then locks that environment in:

1. **Bump the `fe-builder` base image** in `crates/remote/Dockerfile:5` from `node:20-alpine` to
   `node:24-alpine`. This is the direct cause of the `ERR_UNKNOWN_BUILTIN_MODULE: node:sqlite`
   crash — Node 20 lacks that builtin, Node 24 has it. Bumping to 24 (not 22) keeps the two
   Dockerfiles on the same major version, matching the repo-root `Dockerfile`.
2. **Pin pnpm in the Dockerfile** by adding `corepack prepare pnpm@10.25.0 --activate`
   immediately after `corepack enable`. This makes the `packageManager` field in
   `package.json:53` *enforced* rather than advisory. The version string is sourced from
   `package.json` (or, more precisely, from the value at PR-time; the script that generates
   `rebuild.sh` will read it via `jq -r .packageManager` and substitute it into the Dockerfile
   via `ARG` or a build secret — see Design §"Pinning discipline" below).
3. **Tighten `engines.node`** in `package.json:50` from `">=18"` to `">=22.13"`. This is the
   belt to the Dockerfile's suspenders: even if a future maintainer hand-edits the Dockerfile
   to a too-old base, `pnpm install` will fail fast with `EBADENGINE` instead of crashing
   deep inside pnpm. `22.13` matches the minimum pnpm 11.x requires, so this is the
   forward-compatible floor for pnpm 10.x as well.
4. **Add a CI job** at `.github/workflows/remote-hive-build.yml` that runs
   `./crates/remote/rebuild.sh` end-to-end on every PR touching the relevant file globs. The
   job is path-triggered and runs the actual build, not a smoke test of a mocked image.
5. **Document the cross-file coupling** with a one-line comment in each Dockerfile pointing at
   the other, so the next maintainer sees the dependency. This is documentation, not
   enforcement — the CI job in step 4 is the enforcement.

The order of operations matters: do (1) and (2) together in one commit so the build can
succeed. (3) is a separate commit because it changes a different file's contract. (4) and (5)
are independent and can be in their own commit.

## Design / architecture

### Pinning discipline

The `packageManager` field in `package.json` is currently advisory because `corepack enable`
does not auto-prepare the pinned version — it only puts the `corepack` shim on `PATH` and
configures the shim to fetch on first use, and only if the user has not set
`COREPACK_ENABLE_STRICT=0` or `COREPACK_DEFAULT_TO_LATEST=0`. On a fresh image, the shim's
default behaviour is to fetch whatever `pnpm` major version npm has flagged as latest, which on
the date of the failure is pnpm 11.9.0.

The Dockerfile will set the pin explicitly so the value is read from a single source of truth
(`package.json`) at build time, not duplicated:

```dockerfile
# In crates/remote/Dockerfile
ARG PNPM_VERSION
RUN corepack enable \
 && corepack prepare "pnpm@${PNPM_VERSION}" --activate
```

The Compose file or `rebuild.sh` passes the version in as a build arg, sourced from
`package.json` via `jq -r .packageManager | sed 's/pnpm@//'`. This way, changing the pin in
`package.json` automatically propagates to the Dockerfile build with no manual edit.

The repo-root `Dockerfile` will get the same treatment (separate commit, same pattern) so both
images stay in lockstep.

### Cross-file coupling

Two Dockerfiles (`Dockerfile` at root and `crates/remote/Dockerfile`) both build `fe-builder`
stages and must agree on:

- The node base image major version (currently `24-alpine`).
- The pnpm version (sourced from `package.json`, both files read it the same way).

The coupling is enforced by:

- A grep-based CI assertion that the `FROM node:` line in both files has the same major
  version. Lives in the CI workflow added in step (4). Fails PR if they drift.
- A one-line comment in each Dockerfile cross-referencing the other:
  `# keep in sync with <other path>'s fe-builder stage`.

### CI workflow shape

The workflow runs the *real* build, not a mocked or partial one. Reasoning: a mocked smoke
test would have re-introduced this exact bug, because the original failure only shows up
when pnpm 11.x is fetched and Node 20 is the base. The CI must run the real `rebuild.sh`
end-to-end on a Linux runner with Docker available.

```yaml
name: remote-hive-build
on:
  pull_request:
    paths:
      - 'crates/remote/**'
      - 'Dockerfile'
      - 'package.json'
      - 'pnpm-lock.yaml'
      - 'pnpm-workspace.yaml'
      - '.github/workflows/remote-hive-build.yml'
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: ./crates/remote/rebuild.sh
```

The job deliberately uses `rebuild.sh` (no `--full` flag), so it only rebuilds the
`remote-server` container — fast enough for PR feedback, matches the local dev loop. The
`--full` rebuild is exercised manually or on a nightly schedule.

### Failure modes covered

- **Drift to too-old Node base in `crates/remote/Dockerfile`** — caught by the `engines.node`
  bump + the cross-file CI assertion. Both fail fast with clear messages.
- **Drift in `packageManager` value vs. what's actually run** — caught by the
  `corepack prepare` step in the Dockerfile (build fails if the pin is wrong) and by the
  `pnpm --version` assertion in the build log.
- **New dep added that needs a newer Node than what's in `engines.node`** — caught by
  `engine-strict=true` (already on) failing `pnpm install` with `EBADENGINE`.
- **`rebuild.sh` not exercising the new flow** — caught by the CI workflow running the actual
  script, not a re-implementation of it.

## Decisions

- **D1: Bump `crates/remote/Dockerfile:5` from `node:20-alpine` to `node:24-alpine`.** Reversible
  (downstream can pick a different major later). No ADR. The choice of 24 (not 22) is to match
  the repo-root `Dockerfile` exactly; 22 would also fix the immediate crash but would
  re-introduce drift.
- **D2: Pin pnpm via `corepack prepare` in the Dockerfile, sourced from `package.json` at build
  time via `ARG PNPM_VERSION`.** Reversible. No ADR. The alternative — hard-coding
  `pnpm@10.25.0` in the Dockerfile — was rejected because it would create a second source of
  truth that drifts from `package.json`.
- **D3: Tighten `engines.node` from `">=18"` to `">=22.13"`.** Reversible (we can relax it
  again if needed). No ADR. The package is `private: true` (verified at `package.json:5`) so
  this is an internal contract change, not a published one. The exact value `22.13` is the
  minimum that pnpm 11.x requires; pnpm 10.x (our current pin) is happy on 22.13+, so this is
  a forward-compatible floor.
- **D4: Add a CI workflow that runs `rebuild.sh` on every PR touching the relevant files.**
  Reversible. No ADR. The alternative — running a mocked smoke test — was rejected because the
  original bug would not have been caught by a mock.
- **D5: Document the cross-file coupling with comments in each Dockerfile, enforced by a grep
  assertion in CI.** Reversible. No ADR. The alternative — a generated Dockerfile from a
  template — was rejected as overkill for a 2-file coupling.
- **D6: Apply the same `corepack prepare` + ARG pattern to the repo-root `Dockerfile` in a
  separate commit.** Reversible. No ADR. The reason for a separate commit is to keep the
  remote build fix atomic and reviewable; bundling the root-Dockerfile change would conflate
  two unrelated changes and bloat the diff.

No irreversible decisions taken in this spec. No ADRs required.

