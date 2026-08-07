# F-2026-08-06-01 — `e2e-test.sh` is a live-data hazard on hosts running the production hive

- **Finding id**: F-2026-08-06-01
- **Date**: 2026-08-06
- **Severity**: high (potential destruction of production data)
- **Component**: `remote-frontend/scripts/e2e-test.sh`, `crates/remote/docker-compose.dev.yml`
- **Status**: open (documented only — fixing the script is out of scope for `feat/hive-oauth-sw-bypass`)

## Problem

Running `./scripts/e2e-test.sh` as documented is unsafe on any host where the
production hive runs as the docker compose project named `remote` (this host:
ports 9000/9005/5434, volume `remote-dev-db-data` containing real data).

Three compounding defects:

1. **Compose project name collides with prod.** The script invokes
   `docker compose -f docker-compose.dev.yml ... up/down` from `crates/remote/`
   without `-p`/`COMPOSE_PROJECT_NAME`, so the project name defaults to the
   directory name — `remote` — identical to the live production project. Its
   cleanup trap runs `docker compose ... down -v`, which can stop production
   containers and **delete production volumes**.
2. **Health check hardcodes the prod port.** `wait_for_health
   "http://localhost:9000/v1/health"` succeeds against the LIVE hive even when
   the dev stack never started, masking startup failures and green-lighting the
   next steps against production.
3. **Playwright base URL hardcodes the prod port.** The script `export`s
   `PLAYWRIGHT_BASE_URL="http://localhost:9000"` unconditionally (clobbering
   any caller-supplied value), so the E2E suite drives the live hive.

The compose file itself is parameterizable (`SERVER_PORT`, `POSTGRES_PORT`,
`SERVER_PUBLIC_BASE_URL`, `VITE_*`); the script is what pins everything to the
prod endpoints.

## Recommendation

- Parameterize the script: accept `COMPOSE_PROJECT_NAME` (default to a
  dedicated non-`remote` name, e.g. `vk-e2e`), `SERVER_PORT`, and
  `POSTGRES_PORT`; derive the health-check URL and `PLAYWRIGHT_BASE_URL` from
  those instead of hardcoding `localhost:9000` (and respect a caller-supplied
  `PLAYWRIGHT_BASE_URL`).
- Refuse to run when the target compose project already has running containers
  the script did not create (guard before `up` AND before the `down -v`
  cleanup trap), so a name collision fails loudly instead of destroying state.

## Verified safe procedure used on 2026-08-06 (branch validation)

The script's steps were run manually with full isolation; 21/21 Playwright
tests passed and prod was untouched:

```bash
cd crates/remote
COMPOSE_PROJECT_NAME=wsae2e SERVER_PORT=9110 POSTGRES_PORT=5436 \
  SERVER_PUBLIC_BASE_URL=http://localhost:9110 \
  VITE_APP_BASE_URL=http://localhost:9110 VITE_API_BASE_URL=http://localhost:9110 \
  docker compose -f docker-compose.dev.yml [-f <extra_hosts override>] up -d --build
COMPOSE_PROJECT_NAME=wsae2e docker compose -f docker-compose.dev.yml \
  exec -T remote-db psql -U postgres -d vibe_remote -f /dev/stdin < scripts/seed-e2e-db.sql
cd ../../remote-frontend
PLAYWRIGHT_BASE_URL=http://localhost:9110 npx playwright test \
  --config=playwright.docker.config.ts --reporter=list
docker compose -p wsae2e -f ../crates/remote/docker-compose.dev.yml down -v
```

Port note: 9100 was already bound on this host; 9110/5436 were free
(5435 was held by another validation stack).

## Related host issue: Docker embedded-DNS NXDOMAIN on fresh networks

On this host, containers on a freshly created compose network could not
resolve sibling service names (`remote-db` → NXDOMAIN via the `raverx.net`
search domain), so `remote-server` and `seed-db` crash-looped with
"failed to lookup address information". Workaround for future runs: pin the
name with an `extra_hosts` compose override after the db container is up:

```yaml
services:
  remote-server:
    extra_hosts:
      - "remote-db:<db container IP>"
  seed-db:
    extra_hosts:
      - "remote-db:<db container IP>"
```

This is a daemon/host-level issue, not a defect in the compose file or the
branch under test.
