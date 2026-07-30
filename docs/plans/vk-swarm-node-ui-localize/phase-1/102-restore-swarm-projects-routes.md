---
id: "102"
phase: 1
title: "Restore crates/server/src/routes/swarm_projects.rs verbatim and register it"
status: ready
depends_on: ["101"]
parallel: false
conflicts_with: ["101","103","104"]
files:
  - crates/server/src/routes/swarm_projects.rs
  - crates/server/src/routes/mod.rs
siblings:
  - crates/server/src/routes/organizations.rs
irreversible: false
scope_test: "N/A"
allowed_change: mixed
covers_criteria: [SC1]
---

## Failing test (write first)

N/A — Rust route module with no unit-test seam; reachability is proven over HTTP in task 105.
See `## Manual verification` below.

## Sibling alignment (required reading before you write)

Read `crates/server/src/routes/organizations.rs` — the live example of this pattern in this
crate. The restored module must obtain its client via `deployment.remote_client()?` and wrap
responses in `ResponseJson(ApiResponse::success(..))` exactly as it does. Record any divergence
in the decisions-ledger.

## Change

### 1. Create `crates/server/src/routes/swarm_projects.rs` — VERBATIM, no edits

```bash
git show 35b378a5^:crates/server/src/routes/swarm_projects.rs > crates/server/src/routes/swarm_projects.rs
```

Do **not** reformat, rename, or "improve" anything in the recovered file. Every type and
`RemoteClient` method it imports was verified present on `main` during decomposition. Its
`router()` registers:

- `/swarm/projects` (list, create)
- `/swarm/projects/{project_id}` (get, update, delete)
- `/swarm/projects/{project_id}/merge`
- `/swarm/projects/{project_id}/nodes` (list, link)
- `/swarm/projects/{project_id}/nodes/{node_id}` (unlink)

### 2. Register it in `crates/server/src/routes/mod.rs`

- **Anchor:** the `pub mod` declaration block (alphabetical)
- **Change:** add `pub mod swarm_projects;` in alphabetical position.

- **Anchor:** the `base_routes` builder — the `.merge(...)` chain
- **Change:** append one line `.merge(swarm_projects::router())` immediately after the previously added
  `.merge(` line from task 101 (so the four restored modules sit together, in task order).

## Allowed moves

- Create `crates/server/src/routes/swarm_projects.rs` from the git recovery command above.
- Add exactly one `pub mod` line and exactly one `.merge(...)` line to `routes/mod.rs`.

## STOP triggers

- If `crates/server/src/routes/swarm_projects.rs` already exists on disk.
- If the `git show` command fails or produces an empty file.
- If any import in the recovered file fails to resolve — do NOT substitute a different type; STOP.
- If making it compile would require editing a file not listed in `files:`.

## Manual verification (record in decisions-ledger)

```bash
cargo check -p server
# Expected: no errors, no unused-import warnings for the new module

cargo clippy -p server --all-targets --all-features -- -D warnings
# Expected: clean

git diff --stat crates/server/src/routes/swarm_projects.rs
# Expected: the file is byte-identical to 35b378a5^ (no local edits)
```

## Done when

- `crates/server/src/routes/swarm_projects.rs` exists, byte-identical to `35b378a5^`.
- `routes/mod.rs` declares and merges it.
- `cargo check` and `cargo clippy -D warnings` are clean.
