---
id: "203"
phase: 2
title: "Correct the node-api-keys architecture doc, which documents routes D3 removes"
status: passed
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

## Amendments (ORCHESTRATOR, pre-dispatch)

**C1 — anchors re-verified against the live file; the task text is ACCURATE, implement as written.**
`grep -n 'routes/nodes.rs' docs/architecture/db/functions/postgresql-node-api-keys.mdx` returns
exactly six hits, at exactly the line numbers tabulated below (63, 108, 141, 171, 189, 329) with
exactly the quoted text. There is no seventh. The STOP trigger about drifted line numbers should
therefore NOT fire.

**C2 — the ADR relative link resolves; do not "correct" it.** From
`docs/architecture/db/functions/`, the path `../../../../dev-docs/adr/0013-restore-node-surface-hive-proxy-routes.md`
normalises to `dev-docs/adr/0013-restore-node-surface-hive-proxy-routes.md`, which EXISTS. Use the
four `../` exactly as written.

**C4 — the task text contradicted ITSELF; this amendment resolves it (found post-implementation).**
The original `## Manual verification` asserted `grep -n '/api/nodes/api-keys' <file>` returns NO
output, and `## Done when` said "No `/api/nodes/api-keys` reference survives in the file". But the
ADR-0013 note that this SAME task dictates adding reads:

> The node server exposes no `/api/nodes/api-keys*` routes

— i.e. the note necessarily contains the forbidden string, in order to say the routes are gone. The
two instructions could not both be satisfied.

The implementer did the RIGHT thing and reported the hit verbatim rather than silently "fixing" it
by deleting the note or weakening the wording. The defect is mine, in the task text. Resolution: the
grep now excludes the disclaimer blockquote. The real invariant is "no CITATION presents
`/api/nodes/api-keys` as a live node endpoint", which holds — the only remaining occurrence is the
note asserting the opposite. Verified: `grep -n '/api/nodes/api-keys' <file>` returns exactly one
line, `9:> \`/api/nodes/api-keys*\` routes — see`, which is the note.

**C3 — prose heading, not frontmatter.** The file opens with YAML frontmatter (`---` … `---`)
followed by `# Node API Key Functions` at line 6. "Under the document's first heading" means after
that `#` line — NOT inside the frontmatter block. Placing it in the frontmatter would corrupt the
document.

**C5 — line 189's replacement text was ITSELF a false citation; corrected post-panel.** The 203
panel's angle-7 check ("are the repointed claims TRUE?") caught that my dictated replacement for the
"Hard delete option" row was wrong. Verified chain:

- `NodeApiKeyRepository::delete` (`crates/remote/src/db/node_api_keys.rs:178`) has exactly one
  caller: `NodeServiceImpl::delete_api_key` (`crates/remote/src/nodes/service.rs:263-266`).
- `delete_api_key` has NO caller anywhere in `crates/` (`grep -rn 'delete_api_key' crates/ | grep -v
  'fn delete_api_key'` returns nothing).
- The hive's `DELETE /v1/nodes/api-keys/{key_id}` (`crates/remote/src/routes/nodes.rs:57`) is bound
  to `revoke_api_key`, a SOFT revoke — it never reaches the hard delete.

So `routes/nodes.rs` was never the "Used By" for this function, on the node OR the hive. Repointing
a false citation to a differently-false citation would have shipped exactly the drift this
workstream exists to remove. The row now cites the real caller and states plainly that no route
reaches it. **Fixed in-session** (CLAUDE.md "No Deferred Remediation") — this is a correction to the
same file task 203 owns, not a scope stretch.

**C6 — "the five that name a URL" in `## Done when` was an arithmetic slip.** The table has FOUR
URL-bearing rows (63, 141, 171, 329); rows 108 and 189 name no URL. All URL-bearing citations were
converted to `/v1/`. Corrected below.

## Change

Four "Used By" citations point at the node's route module. Repoint each to the hive's.

- **File:** `docs/architecture/db/functions/postgresql-node-api-keys.mdx`

| Line | Before | After |
|---|---|---|
| 63 | `- ``routes/nodes.rs`` - POST /api/nodes/api-keys` | `- ``crates/remote/src/routes/nodes.rs`` - POST /v1/nodes/api-keys` |
| 108 | `- ``routes/nodes.rs`` - Key management` | `- ``crates/remote/src/routes/nodes.rs`` - Key management` |
| 141 | `- ``routes/nodes.rs`` - GET /api/nodes/api-keys` | `- ``crates/remote/src/routes/nodes.rs`` - GET /v1/nodes/api-keys` |
| 171 | `- ``routes/nodes.rs`` - DELETE /api/nodes/api-keys/:id` | `- ``crates/remote/src/routes/nodes.rs`` - DELETE /v1/nodes/api-keys/:id` |
| 189 | `- ``routes/nodes.rs`` - Hard delete option` | `- ``crates/remote/src/routes/nodes.rs`` - Hard delete option` |
| 329 | `- ``routes/nodes.rs`` - POST /api/nodes/api-keys/:id/unblock` | `- ``crates/remote/src/routes/nodes.rs`` - POST /v1/nodes/api-keys/:id/unblock` |

All six were enumerated at decomposition with
`grep -n 'routes/nodes.rs' docs/architecture/db/functions/postgresql-node-api-keys.mdx`; there is
no seventh. Line 189's "Hard delete option" is easy to miss because it names no URL — it still
cites the node module that no longer has these routes.

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

## Manual verification (emit verbatim; the ORCHESTRATOR records it)

```bash
# C4 (see Amendments): the ADR-0013 note deliberately CONTAINS this string in order to state
# that the routes do not exist. Exclude the disclaimer blockquote; what must not survive is a
# CITATION presenting /api/nodes/api-keys as a live node endpoint.
grep -n '/api/nodes/api-keys' docs/architecture/db/functions/postgresql-node-api-keys.mdx | grep -v '^9:>'
# Expected: NO output

grep -c 'crates/remote/src/routes/nodes.rs' docs/architecture/db/functions/postgresql-node-api-keys.mdx
# Expected: exactly 6

grep -n 'routes/nodes.rs' docs/architecture/db/functions/postgresql-node-api-keys.mdx | grep -v 'crates/remote'
# Expected: NO output (no bare `routes/nodes.rs` citation survives)
```

## Done when

- No `/api/nodes/api-keys` CITATION survives (the ADR-0013 note's mention, which states the routes
  do NOT exist, is expected and required — see amendment C4).
- The four URL-bearing "Used By" citations point at `crates/remote/src/routes/nodes.rs` with `/v1/`
  paths; the "Key management" row cites the same module; and the "Hard delete option" row cites the
  real caller `crates/remote/src/nodes/service.rs` with a note that no route reaches it (see C5).
- The ADR-0013 note is present.
