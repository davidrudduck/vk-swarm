---
doc_type: analysis
status: final
workstream: remote-docker-build-fix
---

# remote-docker-build-root-cause — Diagnosis of the fe-builder build failure

> **Analysis note, not a spec.** Sibling to
> [`2026-07-05-remote-docker-build-fix.md`](./2026-07-05-remote-docker-build-fix.md), which
> captures the fix intent. This doc records the root-cause investigation and is the durable
> home for the "why" — the spec links here so the diagnosis does not have to be restated in
> the spec body.

## Symptoms (what the user reported)

`./crates/remote/rebuild.sh` (or `docker compose --env-file .env.remote build remote-server`)
fails at the `fe-builder` stage with:

```text
! Corepack is about to download https://registry.npmjs.org/pnpm/-/pnpm-11.9.0.tgz
warn: This version of pnpm requires at least Node.js v22.13
warn: The current version of Node.js is v20.20.2
warn: Visit https://r.pnpm.io/comp to see the list of past pnpm versions with respective Node.js version support.
Error [ERR_UNKNOWN_BUILTIN_MODULE]: No such built-in module: node:sqlite
    at Module._load (node:internal/modules/cjs/loader:1031:13)
    at Module.require (node:internal/modules/cjs/loader:1289:19)
    at require (node:internal/modules/helpers:182:18)
    at ../store/index/lib/index.js (file:///root/.cache/node/corepack/v1/pnpm/11.9.0/dist/pnpm.mjs:54590:25)
    ...
failed to solve: process "/bin/sh -c pnpm install --filter ./remote-frontend --frozen-lockfile" did not complete successfully: exit code: 1
```

The build aborts. All downstream stages (`builder`, `runtime`) are marked `CANCELED`.

## Root cause (two independent bugs, stacked)

### Bug 1: `fe-builder` base image is `node:20-alpine`, missing `node:sqlite`

Evidence:

- `crates/remote/Dockerfile:5` — `FROM node:20-alpine AS fe-builder`.
- Repo-root `Dockerfile:2` — `FROM node:24-alpine AS builder`. The root Dockerfile is on 24.
- The two Dockerfiles silently drifted apart. There is no gate that flags the divergence.
- The `node:sqlite` builtin was added in Node 22.5+ (see Node release notes; pnpm 11.x
  documents this requirement in its own startup warning). Node 20 is end-of-life and has no
  `node:sqlite`.

This is the *direct* cause of the `ERR_UNKNOWN_BUILTIN_MODULE: node:sqlite` line in the
stack trace. pnpm 11.x hard-imports `node:sqlite` at module init.

### Bug 2: pnpm version is not pinned in the Dockerfile

Evidence:

- `package.json:53` — `"packageManager": "pnpm@10.25.0"`.
- `crates/remote/Dockerfile` (line 4 in the build trace) — `RUN corepack enable`. There is
  **no** `corepack prepare pnpm@10.25.0 --activate` or `npm install -g pnpm@10.25.0` step.
- The `packageManager` field is therefore *advisory*: `corepack enable` puts the shim on
  PATH and the shim fetches whatever version of pnpm is appropriate at first use, which on
  this Node 20 image is pnpm 11.9.0 (the version npm returns for `latest` on the date of
  the failure).
- The `engines.node = ">=18"` field in `package.json:50` is also too permissive — it does
  not exclude Node 20, and `engine-strict=true` in `.npmrc` only governs npm package engines,
  not the pnpm runtime itself.

This is *how* pnpm 11.9.0 ended up being run. Without this bug, pnpm 10.25.0 would have been
fetched and used, and the build would have proceeded (modulo any other issues).

## Why these two bugs compound

On a `node:24-alpine` base with a `corepack prepare` step:

- Bug 1 is fixed (Node 24 has `node:sqlite`).
- Bug 2 is fixed (`corepack prepare` enforces pnpm 10.25.0).
- pnpm 10.25.0 runs cleanly on Node 24, builds, produces a working image.

On a `node:20-alpine` base with `corepack prepare pnpm@10.25.0 --activate`:

- Bug 1 is *not* fixed, but pnpm 10.25.0 does not need `node:sqlite` (it was added in pnpm 11).
  So pnpm 10.x on Node 20 *would* work.
- This is a workable state, though not ideal — Node 20 is EOL.

On a `node:20-alpine` base *without* the `corepack prepare` step (current state):

- pnpm 11.9.0 is fetched.
- pnpm 11.9.0 needs `node:sqlite`.
- Node 20 lacks `node:sqlite`.
- Crash.

This is the state we are in. **Both bugs are required to produce the observed failure**;
fixing only one leaves a degraded but functional state. The spec fixes both.

## What is *not* the cause

- **`.env.remote.dev`** is application config (DB URL, OAuth, JWT, ports, RUST_LOG). It has
  zero Node/pnpm/corepack config and even if it did, `docker compose --env-file` only sets
  runtime env, not build env. The build never reads this file.
- **The Rust toolchain image** (`rust:1.89-slim-bookworm` in the `builder` stage) is
  unrelated. It was canceled because Docker aborted the build at the first failure, not
  because it had a problem.
- **The runtime base image** (`debian:bookworm-slim`) is unrelated. It is the third and
  final stage; it never started.
- **The `packageManager` field being missing or wrong** — it is present and correct
  (`pnpm@10.25.0`). The Dockerfile just doesn't honor it.
- **The `pnpm-lock.yaml`** — there is no lockfile mismatch; the failure is upstream of
  resolution.

## Why this slipped through

- `engines.node = ">=18"` was permissive enough to accept Node 20 silently.
- There is no path-triggered CI job that builds the remote image; the root Dockerfile is
  built (because the node image is part of the local-build path) but `crates/remote` is not.
- The `fe-builder` stage of `crates/remote/Dockerfile` was last exercised by a subagent that
  did not run the actual `docker buildx build` end-to-end against a clean cache; it reviewed
  the file statically and chose `node:20-alpine` (presumably because pnpm 10.x supported it
  at the time). The drift to the root Dockerfile's `node:24-alpine` was not caught because
  no comparison was made.

## Evidence index

| Claim | File:line |
| --- | --- |
| `fe-builder` base is `node:20-alpine` | `crates/remote/Dockerfile:5` |
| Root Dockerfile base is `node:24-alpine` | `Dockerfile:2` |
| `packageManager` pin is `pnpm@10.25.0` | `package.json:53` |
| `engines.node` is `>=18` | `package.json:50` |
| `engine-strict=true` | `.npmrc:1` |
| All packages are `private: true` (engines is internal) | `package.json:5`, `frontend/package.json:4`, `remote-frontend/package.json:4` |
| Rebuild script is the source of truth for the build flow | `crates/remote/rebuild.sh:1-30` |
| Compose file consumes `.env.remote` at runtime, not build | `crates/remote/docker-compose.yml` (file exists; consumed by `rebuild.sh` via `--env-file`) |
