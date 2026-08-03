# Task 501 — LIVE "after" evidence (feature branch deployed)

Captured **2026-08-03** against the user's real node deployment, read-only GETs only.
Pairs with [`501-live-before-evidence.md`](501-live-before-evidence.md), same host, same six paths.

## Target under test — build identity verified BEFORE trusting any probe

```text
$ curl -s http://10.69.96.233:9001/api/health
{"status":"ok","version":"0.0.125","git_commit":"374598a7","git_branch":"feat/vk-swarm-node-ui-localize","build_timestamp":"2026-08-03T20:22:33Z","database_ready":true}
```

```text
$ git rev-parse --short=8 HEAD
374598a7
$ git branch --contains 374598a7
* feat/vk-swarm-node-ui-localize
```

The deployed commit is **byte-identical to the branch HEAD under review**, not a near build.
The before-capture ran against `main`/`feff74be`; this is the same host on this branch.

## The six paths, after

```text
/api/nodes                                                 -> 400 text/plain; charset=utf-8
/api/swarm/projects                                        -> 400 text/plain; charset=utf-8
/api/swarm/labels                                          -> 400 text/plain; charset=utf-8
/api/swarm/templates                                       -> 400 text/plain; charset=utf-8
/api/merged-projects                                       -> 200 text/html
/api/projects/with-stats                                   -> 200 application/json
```

### Bodies — the decisive part

```text
$ curl -s http://10.69.96.233:9001/api/nodes
Failed to deserialize query string: missing field `organization_id`
$ curl -s http://10.69.96.233:9001/api/swarm/projects
Failed to deserialize query string: missing field `organization_id`
$ curl -s http://10.69.96.233:9001/api/swarm/labels
Failed to deserialize query string: missing field `organization_id`
$ curl -s http://10.69.96.233:9001/api/swarm/templates
Failed to deserialize query string: missing field `organization_id`
```

This is **stronger evidence than the content-type flip the before-doc predicted.** That message is
emitted by Axum's `Query<ListNodesQuery>` extractor rejecting a request that had already been
**routed to the restored handler** (`crates/server/src/routes/nodes.rs:22-28`). The SPA catch-all
(`routes/mod.rs:76` → `frontend.rs:40-43`) can only ever return `200 text/html`; it has no
extractor and cannot name `organization_id`. A route that does not exist cannot reject a query
string. Registration is therefore proven by the response BODY, not merely inferred from a header.

### With a well-formed `organization_id` — the proxy actually traverses to the hive

```text
$ O=00000000-0000-0000-0000-000000000000
$ curl -s "http://10.69.96.233:9001/api/nodes?organization_id=$O"
{"success":false,"data":null,"error_data":null,"message":"Unauthorized. Please sign in again."}
$ curl -s "http://10.69.96.233:9001/api/swarm/projects?organization_id=$O"
{"success":false,"data":null,"error_data":null,"message":"Unauthorized. Please sign in again."}
$ curl -s "http://10.69.96.233:9001/api/swarm/labels?organization_id=$O"
{"success":false,"data":null,"error_data":null,"message":"Unauthorized. Please sign in again."}
$ curl -s "http://10.69.96.233:9001/api/swarm/templates?organization_id=$O"
{"success":false,"data":null,"error_data":null,"message":"Unauthorized. Please sign in again."}
```

All four `401 application/json`, in the `ApiResponse` envelope.

**Disambiguating the 401 (two call sites share this string).** `crates/server/src/error.rs:261`
(`ApiError::RemoteClient(RemoteClientError::Auth)`) and `error.rs:309` (`ApiError::Unauthorized`)
emit identical text. The router settles it: `nodes::router()`
(`crates/server/src/routes/nodes.rs:61-65`) declares only `.route(...)` calls — **no `.layer()`, no
auth middleware**. `list_nodes` takes only `State` and `Query`. So no local guard can produce a 401
before the handler body runs; the only reachable source is `client.list_nodes(...)` returning
`RemoteClientError::Auth` — i.e. **the node built a remote client, called the hive, and propagated
the hive's rejection of an unauthenticated curl.** That is a full end-to-end proxy traversal, live.

**A second fact falls out of the same observation.** `deployment.remote_client()?`
(`routes/nodes.rs:26`) returned `Ok` — otherwise task 401's `From<RemoteClientNotConfigured>`
mapping would have produced `503 HiveNotConfigured` instead. So this node **is** hive-configured,
and the SC4 branch correctly did **not** fire. SC4's 503 path is therefore not live-observable on
this host by construction; it remains covered by the in-process registration tests. Stated plainly
rather than implied.

### `/api/merged-projects` — deleted, now falls through to the SPA

```text
$ curl -s http://10.69.96.233:9001/api/merged-projects | head -c 120
<!DOCTYPE html>
<html><head><title>Build frontend first</title></head>
<body><h1>Please build the frontend</h1></body></
```

Exactly the inverse of the before-state (`200 application/json`): the route is gone, so the
catch-all serves the SPA placeholder. ADR-0014 satisfied.

### `/api/projects/with-stats` — live, with real data

```text
$ curl -s http://10.69.96.233:9001/api/projects/with-stats | head -c 400
{"success":true,"data":{"projects":[{"id":"c8809147-3066-439e-9f2b-9477cb3e8bec","name":"vibe-kanban","git_repo_path":"/home/david/Code/vibe-kanban","created_at":"2025-11-28T03:41:40.239Z","remote_project_id":"e9debe6a-f267-4243-8e46-e2fabdbe66c8","last_attempt_at":"2026-06-25T01:24:50Z","github_enabled":false,"github_owner":null,"github_repo":null,"github_open_issues":0,"github_open_prs":0,"githu
```

Was `400 text/plain` on `main` (the `.nest("/{id}")` `Path<Uuid>` shadowing recorded during the
run). Now `200 application/json` serving the node's real project rows.

## Before → after, side by side

| Path | before (`main`/`feff74be`) | after (`374598a7`) | verdict |
|---|---|---|---|
| `/api/nodes` | `200 text/html` (SPA) | `400` extractor msg → `401` hive auth | registered + proxying |
| `/api/swarm/projects` | `200 text/html` (SPA) | `400` extractor msg → `401` hive auth | registered + proxying |
| `/api/swarm/labels` | `200 text/html` (SPA) | `400` extractor msg → `401` hive auth | registered + proxying |
| `/api/swarm/templates` | `200 text/html` (SPA) | `400` extractor msg → `401` hive auth | registered + proxying |
| `/api/merged-projects` | `200 application/json` | `200 text/html` (SPA) | deleted as designed |
| `/api/projects/with-stats` | `400 text/plain` | `200 application/json` + real rows | added as designed |

Every row inverted in the predicted direction. The incident symptom recorded live in the
before-capture — four node-surface routes serving HTML to screens expecting JSON — is gone.
