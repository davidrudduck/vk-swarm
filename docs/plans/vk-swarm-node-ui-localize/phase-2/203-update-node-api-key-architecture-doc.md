---
id: "203"
phase: 2
title: "Correct the node-api-keys architecture doc, which documents routes D3 removes"
status: ready
depends_on: ["202"]
parallel: false
conflicts_with: []
files:
  - docs/architecture/db/functions/postgresql-node-api-keys.mdx
irreversible: false
scope_test: "N/A"
allowed_change: edit
covers_criteria: [SC3]
---

## Failing test (write first)

N/A — documentation. Verified by the greps in Manual verification.

## Why this task exists

`docs/architecture/db/functions/postgresql-node-api-keys.mdx` cites the node's
`routes/nodes.rs` as the caller of four API-key endpoints. After task 101 those endpoints do not
exist on the node — the hive's `crates/remote/src/routes/nodes.rs` is the only caller. Leaving
the doc as-is ships a document that describes a 404 as if it were live, which is the exact class
of drift this workstream was opened to remove.

## Change

Four "Used By" citations point at the node's route module. Repoint each to the hive's.

- **File:** `docs/architecture/db/functions/postgresql-node-api-keys.mdx`

| Line | Before | After |
|---|---|---|
| 63 | `- ``routes/nodes.rs`` - POST /api/nodes/api-keys` | `- ``crates/remote/src/routes/nodes.rs`` - POST /v1/nodes/api-keys` |
| 108 | `- ``routes/nodes.rs`` - Key management` | `- ``crates/remote/src/routes/nodes.rs`` - Key management` |
| 141 | `- ``routes/nodes.rs`` - GET /api/nodes/api-keys` | `- ``crates/remote/src/routes/nodes.rs`` - GET /v1/nodes/api-keys` |
| 171 | `- ``routes/nodes.rs`` - DELETE /api/nodes/api-keys/:id` | `- ``crates/remote/src/routes/nodes.rs`` - DELETE /v1/nodes/api-keys/:id` |

If a citation for `POST /api/nodes/api-keys/:id/unblock` is also present, repoint it the same
way to `POST /v1/nodes/api-keys/:id/unblock`.

Then add this note under the document's first heading:

```markdown
> **Node API keys are managed on the hive only.** The node server exposes no
> `/api/nodes/api-keys*` routes — see
> [ADR-0013](../../../../dev-docs/adr/0013-restore-node-surface-hive-proxy-routes.md).
```

## Allowed moves

- Edit the cited lines and add the note, in that one `.mdx` file.

## STOP triggers

- If the line numbers above do not match the file's content (the doc changed since
  decomposition) — locate the citations by their text, and if the text does not match either,
  STOP and report.
- Do NOT edit any other doc under `docs/architecture/`.

## Manual verification (record in decisions-ledger)

```bash
grep -n '/api/nodes/api-keys' docs/architecture/db/functions/postgresql-node-api-keys.mdx
# Expected: NO output

grep -c 'crates/remote/src/routes/nodes.rs' docs/architecture/db/functions/postgresql-node-api-keys.mdx
# Expected: 4 (or 5 if the unblock citation was present)
```

## Done when

- No `/api/nodes/api-keys` reference survives in the file.
- Every "Used By" citation points at the hive's route module with a `/v1/` path.
- The ADR-0013 note is present.
