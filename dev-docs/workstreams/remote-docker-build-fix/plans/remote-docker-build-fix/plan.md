---
topic: remote-docker-build-fix
spec: docs/superpowers/specs/2026-07-05-remote-docker-build-fix.md
status: ready
---

# Plan — remote-docker-build-fix (unblock remote/hive Docker build)

## Approach

Three phases, each one commit unit. **Phase 1** applies the actual fix to the Dockerfiles,
docker-compose.yml, rebuild.sh, and package.json — the changes that make the build pass. **Phase
2** adds the enforcement gates (FROM-line assertion script, CI workflow) so future drift is caught
at PR time rather than on a production deploy. **Phase 3** is a manual end-to-end verification that
runs `rebuild.sh`, composes up, and hits `/v1/health`.

Tasks within a phase that touch the **same file** are chained via `depends_on`. Independent-file
tasks are parallel-authorable but run sequentially in the executor.

This is a **build-system-only** workstream: no Rust, no frontend behaviour, no API contract changes.
The fix is a Dockerfile base bump + a pnpm pin + a `package.json` engines bump + a CI job + a
cross-file assertion. The `docker compose --env-file .env.remote` flow is the source of truth.

## Phases

1. **phase-1-build-fix** — Dockerfile fixes + package.json engines bump
2. **phase-2-enforcement** — FROM-line assertion script + CI workflow
3. **phase-3-verification** — manual E2E verification

## Tasks

| id | phase | title | dep: | conflicts: | covers |
|---|---|---|---|---|---|
| 001 | 1 | crates/remote/Dockerfile fe-builder: node:24-alpine + ARG PNPM_VERSION + corepack prepare; docker-compose.yml + rebuild.sh: pipe PNPM_VERSION | dep: - | conflicts: 002,004 | SC3,SC4 |
| 002 | 1 | Root Dockerfile: pin pnpm@10.25.0 + cross-file coupling comment | dep: - | conflicts: 001,004 | SC4 |
| 003 | 1 | package.json: tighten engines.node >=22.13 | dep: - | conflicts: none | SC5 |
| 004 | 2 | Create FROM-line assertion script (grep-based node version match) | dep: - | conflicts: 001,002,005 | SC3 |
| 005 | 2 | Create .github/workflows/remote-hive-build.yml CI job | dep: 004 | conflicts: 004,006 | SC6 |
| 006 | 3 | Manual E2E verification: rebuild.sh + compose up + healthcheck | dep: 001,002,003,005 | conflicts: 005 | SC1,SC2 |