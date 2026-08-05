# Task 105 — reachability evidence (end-to-end, real running server)

All output below is verbatim. No paraphrase, no summary.

## Provenance

- Commit under test: `690ffab089c2929c4f31b112899407f8e5ab8919` (working tree clean, 0 modified files)
- Binary: `./target/debug/vks-node-server`, built from that commit with
  `cargo build -p server --bin vks-node-server`
- Launched with an ISOLATED asset dir so the probe could not touch the developer's `dev_assets/`
  — this is task 099's `VK_ASSET_DIR` override doing exactly the job it was added for:

```bash
VK_ASSET_DIR="$SCRATCH/e2e-assets" HOST=127.0.0.1 BACKEND_PORT=9412 RUST_LOG=warn \
  ./target/debug/vks-node-server
```

- No hive is configured, which is the intended condition for this check (see block 2's note).
- A pre-existing vibe-kanban instance belonging to a DIFFERENT checkout
  (`/home/david/Tools/vk-swarm`, backend port 9002) was running throughout and was deliberately
  left untouched; this probe used port 9412 and was stopped by exact PID.

## Block 1 — server started

```text
$ curl -s -o /dev/null -w '%{http_code} %{content_type}\n' http://127.0.0.1:9412/api/health
200 application/json
```

## Block 2 — every restored path is REGISTERED

The discriminator is CONTENT-TYPE, not status code: an unregistered `/api` path returns
`200 + text/html` because the outer router's catch-all `.route("/{*path}", ...)`
(`crates/server/src/routes/mod.rs:76`) serves `index.html` with `StatusCode::OK`
(`crates/server/src/routes/frontend.rs:40-43`).

`application/json` → registered. The `400 "Remote client not configured"` body is the
not-configured error and is a PASS for this task (task 401 later changes it to 503).

```text
=== BLOCK 2: restored route reachability ===
/api/nodes?organization_id=00000000-0000-0000-0000-000000000000 -> 400 application/json
/api/nodes/00000000-0000-0000-0000-000000000000 -> 400 application/json
/api/nodes/00000000-0000-0000-0000-000000000000/projects -> 400 application/json
/api/swarm/projects?organization_id=00000000-0000-0000-0000-000000000000 -> 400 application/json
/api/swarm/labels?organization_id=00000000-0000-0000-0000-000000000000 -> 400 application/json
/api/swarm/templates?organization_id=00000000-0000-0000-0000-000000000000 -> 400 application/json
```

Six paths, six `application/json`. Zero `text/html`.

### Negative control (added by the ORCHESTRATOR — not in the original task text)

Six JSON responses only prove registration if a genuinely unregistered path behaves differently
on THIS binary. Without this control the block above would be consistent with "everything returns
JSON". It does not:

```text
=== CONTROL: a deliberately unregistered path (must be text/html) ===
/api/definitely-not-a-route -> 200 text/html
```

The SPA fallback is live on this binary and returns `200 text/html`, so the `application/json`
results in block 2 are a real discrimination, not an artefact.

### Sample body

```text
$ curl -s "http://127.0.0.1:9412/api/swarm/templates?organization_id=00000000-0000-0000-0000-000000000000"
{"success":false,"data":null,"error_data":null,"message":"Remote client not configured"}
```

## Block 3a — D3 / SC3 source-level proof (PRIMARY evidence)

```bash
$ git grep -n 'api-keys\|api_key' -- crates/server/src/routes/ || echo "NO api-key surface in routes"
NO api-key surface in routes
```

No API-key surface exists anywhere in the node's route layer.

## Block 3b — D3 HTTP behaviour (recorded for completeness)

Since task 101 restored `/nodes/{node_id}` as `Path<Uuid>`, the literal path `/api/nodes/api-keys`
now MATCHES that route and fails UUID parsing. It therefore returns `400 text/plain`, NOT the SPA
fallback. What matters is that no JSON key listing is returned:

```text
$ curl -s -w '\n-> %{http_code} %{content_type}\n' \
    "http://127.0.0.1:9412/api/nodes/api-keys?organization_id=00000000-0000-0000-0000-000000000000"
Invalid URL: Cannot parse `node_id` with value `api-keys`: UUID parsing failed: invalid character: expected an optional prefix of `urn:uuid:` followed by [0-9a-fA-F-], found `p` at 2
-> 400 text/plain; charset=utf-8
```

A `node_id` parse error, not a key list. D3 holds.

## Result

- Six restored paths: all `application/json` → all registered.
- Negative control: `200 text/html` → the discriminator is real on this binary.
- D3/SC3: no api-key surface in source; HTTP probe returns a parse error, never a key listing.
- No source file was modified by this task.
