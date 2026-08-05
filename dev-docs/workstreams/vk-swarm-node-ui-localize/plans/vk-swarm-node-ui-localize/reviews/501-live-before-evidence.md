# Task 501 — LIVE "before" evidence (incident symptom observed in production)

Captured against the user's real deployment on **2026-08-03**, read-only GETs only.
This is the reachability gate's part (c): the incident symptom **observed live**, not inferred.

> **Host addresses redacted.** `NODE_HOST` and `HIVE_HOST` stand for the real node and hive
> addresses this evidence was captured against. This repository is public, so the internal
> addresses were replaced before publication. Every status code, content-type, response body,
> command, and commit hash below is **unmodified** — only the host portion is opaque. The capture
> is reproducible by substituting the real hosts.

## Target under test

```text
$ curl -s http://NODE_HOST/api/health
{"status":"ok","version":"0.0.125","git_commit":"feff74be","git_branch":"main","build_timestamp":"2026-08-01T09:34:53Z","database_ready":true}
```

`git_branch: main`, `git_commit: feff74be` — i.e. the pre-workstream baseline, NOT this feature
branch. That is what makes this a valid *before* observation.

The hive is up and healthy, so this is not a hive outage:

```text
$ curl -s -o /dev/null -w 'hive health -> %{http_code} %{content_type}\n' https://HIVE_HOST/v1/health
hive health -> 200 application/json
```

## The incident symptom, live

```text
/api/nodes                                                 -> 200 text/html
/api/swarm/projects                                        -> 200 text/html
/api/swarm/labels                                          -> 200 text/html
/api/swarm/templates                                       -> 200 text/html
/api/merged-projects                                       -> 200 application/json
/api/projects/with-stats                                   -> 400 text/plain; charset=utf-8
```

### What this proves

1. **The four node-surface routes are unregistered in production right now.** They return
   `200 text/html` — the SPA's `index.html`, served by the outer catch-all
   `.route("/{*path}", get(frontend::serve_frontend))` (`crates/server/src/routes/mod.rs:76`) with
   `StatusCode::OK` (`frontend.rs:40-43`). The node's Nodes and swarm screens call these paths and
   receive HTML where they expect JSON.

2. **The spec's "404" wording was factually wrong, and this is the live proof.** Nothing 404s. A
   gate asserting `assert_ne!(status, 404)` would have PASSED against this broken production
   server. That vacuity affected seven task files at decompose time and was corrected during the
   run (see the ledger's task 100/105 entries); this capture is the production-side confirmation.

3. **`/api/merged-projects` is still live on main** (`200 application/json`) — the endpoint task 303
   removes, replaced by `/api/projects/with-stats`.

4. **`/api/projects/with-stats` does not exist on main.** It returns `400 text/plain` rather than
   the SPA fallback because the literal `with-stats` is captured by `.nest("/{id}", ...)` and fails
   `Path<Uuid>` parsing — the same static-vs-dynamic shadowing behaviour recorded for
   `/api/nodes/api-keys` during the run.

## Expected AFTER state (feature branch deployed)

| Path | before (main) | after (this branch) |
|---|---|---|
| `/api/nodes` | `200 text/html` | `200 application/json` (hive configured) or `503 application/json` (no hive) |
| `/api/swarm/projects` | `200 text/html` | `200`/`503 application/json` |
| `/api/swarm/labels` | `200 text/html` | `200`/`503 application/json` |
| `/api/swarm/templates` | `200 text/html` | `200`/`503 application/json` |
| `/api/merged-projects` | `200 application/json` | `200 text/html` (route deleted → SPA fallback) |
| `/api/projects/with-stats` | `400 text/plain` | `200 application/json` |

The decisive assertion is the CONTENT-TYPE flip on the first four rows: `text/html` → `application/json`
means the routes are registered. A status code alone cannot show this on a server with a SPA catch-all.
