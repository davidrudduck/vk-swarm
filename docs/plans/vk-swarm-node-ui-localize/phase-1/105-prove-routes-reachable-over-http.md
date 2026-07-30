---
id: "105"
phase: 1
title: "Prove every restored route is reachable over HTTP against a running server"
status: ready
depends_on: ["104"]
parallel: false
conflicts_with: []
files:
  - docs/plans/vk-swarm-node-ui-localize/reviews/105-reachability-evidence.md
irreversible: false
scope_test: "N/A"
allowed_change: create
covers_criteria: [SC1, SC2]
---

## Failing test (write first)

N/A by design — and the reason matters, so read it before substituting a unit test.

The bug this workstream fixes is that the routes are **not registered**. A test that calls
`list_nodes()` (or any restored handler) directly would pass on `main` today, before any of
tasks 101–104 exist, because the handler function was never the broken part. Such a test proves
the proxy works and proves **nothing** about reachability — it is hollow, and this plan refuses
it (see plan.md, "Known limitation").

Tasks 101-104 now carry in-process router tests via task 100's harness, which cover registration
and the configured/absent-hive contract. This task is **not** redundant with them: it is the
end-to-end check against a really-running server — the same evidence the run-close reachability
gate and `wai-evidence.sh` require — and it is what proves the binary a user actually starts
serves these paths.

## Change

Start the dev server and record the observed HTTP status of every restored path.

- **File:** `docs/plans/vk-swarm-node-ui-localize/reviews/105-reachability-evidence.md`
- **Before:** (does not exist)
- **After:** a file containing the verbatim command output from the Manual verification block
  below — no paraphrase, no summary. Fenced code blocks only.

## Allowed moves

- Create the evidence file. This task changes NO source code.

## STOP triggers

- If any path returns `text/html`, the registration in tasks 101–104 is incomplete. STOP and report
  which path — do not "fix" it from this task; the fix belongs in the owning task's file.
- If the server will not start, STOP and report the startup error. Do not record partial
  evidence as a pass.
- If you are tempted to add a Rust unit test calling a handler function directly to "cover" this
  task — STOP and re-read the Failing test section.

## Manual verification (emit verbatim; the ORCHESTRATOR records it)

```bash
# 1. Start the node server (a hive need NOT be configured for this check)
pnpm run dev    # note the BACKEND_PORT it reports; export it as PORT below

# 2. Every restored path must be REGISTERED. Do NOT use the status code for this:
#    an UNREGISTERED /api path returns 200 + text/html, because the outer router's
#    catch-all `.route("/{*path}", ...)` (crates/server/src/routes/mod.rs:76) serves
#    index.html with StatusCode::OK (crates/server/src/routes/frontend.rs:40-43).
#    The discriminator is CONTENT-TYPE:
#      application/json -> registered (the not-configured error, 400 today / 503 after
#                          task 401, is a PASS for this task)
#      text/html        -> NOT registered; the request fell through to the SPA.
for p in \
  "/api/nodes?organization_id=00000000-0000-0000-0000-000000000000" \
  "/api/nodes/00000000-0000-0000-0000-000000000000" \
  "/api/nodes/00000000-0000-0000-0000-000000000000/projects" \
  "/api/swarm/projects?organization_id=00000000-0000-0000-0000-000000000000" \
  "/api/swarm/labels?organization_id=00000000-0000-0000-0000-000000000000" \
  "/api/swarm/templates?organization_id=00000000-0000-0000-0000-000000000000" ; do
  printf '%s -> ' "$p"
  curl -s -o /dev/null -w '%{http_code} %{content_type}\n' "http://127.0.0.1:${PORT}${p}"
done
# Expected: six lines, EVERY one application/json. Any text/html means that route is
#           still unregistered, whatever its status code says.

# 3. D3 assertion — the API-key surface must NOT be reachable (SC3)
curl -s -o /dev/null -w '%{http_code} %{content_type}\n' \
  "http://127.0.0.1:${PORT}/api/nodes/api-keys?organization_id=00000000-0000-0000-0000-000000000000"
# Expected: text/html — the SPA fallback, proving the route is NOT registered.
# NOTE: this is NOT a 404. A deleted/absent route falls through to the catch-all,
# which answers 200 + index.html. Asserting 404 here would FAIL.
```

Paste all three blocks' real output into the evidence file and into the decisions-ledger.

## Done when

- `reviews/105-reachability-evidence.md` exists and contains verbatim output showing six
  responses whose content-type is `application/json`.
- The API-key path returns `text/html` (the SPA fallback), confirming D3 held.
- No source file was modified by this task.
