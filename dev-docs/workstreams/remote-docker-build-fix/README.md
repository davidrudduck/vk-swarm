---
workstream: remote-docker-build-fix
doc_type: readme
status: shipped
title: "Fix crates/remote Docker build — unblock remote/hive standup"
staging_pointers:
  - dev-docs/workstreams/remote-docker-build-fix/plans/remote-docker-build-fix
  - dev-docs/workstreams/remote-docker-build-fix/spec/2026-07-05-remote-docker-build-fix.md
---

# remote-docker-build-fix

Unblock the `crates/remote` Docker build. `fe-builder` stage fails with
`ERR_UNKNOWN_BUILTIN_MODULE: node:sqlite` because it pins `node:20-alpine` (missing the
builtin) while the repo-root `Dockerfile` is on `node:24-alpine`. The Dockerfile also doesn't
`corepack prepare` the pinned pnpm version, so corepack fetches pnpm 11.9.0 which is itself
incompatible with the available Node runtime. Fix: bump base image, pin pnpm via corepack,
tighten `engines.node`, add a CI job, document the cross-file coupling.
