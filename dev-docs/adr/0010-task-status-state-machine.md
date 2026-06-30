# ADR-0010 — Explicit task.status state machine with single-author transitions

- **Status:** accepted
- **Date:** 2026-06-30
- **Workstream:** vk-swarm-hive-redesign
- **Supersedes behaviour of:** field-level last-write-wins on `task.status`
  (`upsert_from_node`/`upsert_remote_task`)

## Context

Today `task.status` is reconciled field-by-field with no explicit transition model: the hive's
`upsert_from_node` stamps a monotonic delivery counter and last-write-wins (analysis §2.4), and the node
accepts hive status via the `remote_version`-only gate that silently clobbers concurrent local edits
(`task/sync.rs:300`, `task/queries.rs:305-307`). There is **no agreement on who may author which
transition**.

Two concrete hazards confirmed in code:

- **Status enum value drift** — SQLite stores `inprogress`/`inreview`; Postgres stores the hyphenated
  enum `in-progress`/`in-review` (agent-C schema diff). Any path that doesn't map is a silent-corruption
  risk.
- Paperclip demonstrates the pattern worth copying is its **plugin-lifecycle `VALID_TRANSITIONS` map**
  (`paperclip/plugin-lifecycle.ts:79-93`), **not** its ad-hoc issue path whose `assertTransition` only
  rejects *unknown* statuses (`issues.ts:109`).

## Decision

Define `task.status` as an **explicit guarded transition matrix** where **every transition has exactly
one authoritative author** (SC4).

**Reconciliation with the real enum (ratified 2026-06-30).** `task.status` is exactly
`{Todo, InProgress, InReview, Done, Cancelled}` in both crates (`crates/db/src/models/task/mod.rs:27`,
`crates/remote/src/db/tasks.rs:25`) — there is **no `Assigned` and no `Failed` variant**. SC4's
parenthetical `assigned`/`failed` are **authority labels for lifecycle concepts, not `task.status`
values**, and map to other columns:
- **`assigned`** = the existence of an **active `node_task_assignments` row** (hive-authored, the
  assignment layer — ADR-0009), NOT a `task.status` value.
- **`failed`** = an **`execution_status`** outcome on the assignment / execution process (node-reported,
  the execution layer), NOT a `task.status` value.

The `task.status` matrix over the real enum:

| Transition | Author |
|---|---|
| `todo → in-progress` | **hive** (on assignment + node start) |
| `in-progress → in-review` | **node** (run completed, awaiting review) |
| `in-progress → done` | **node** (terminal) |
| `in-review → done` / `in-review → in-progress` (reopen) | **hive** (operator review) |
| `* → cancelled` | **hive** (operator action) |

(The "assigned" milestone is the `node_task_assignments` row created by the hive's atomic checkout,
ADR-0009; the "failed" outcome is the assignment/execution `execution_status`. Both are single-author
in their own layer, so the no-field-level-conflict guarantee holds across all three surfaces.)

- A **node-reported** transition is accepted by the hive only when carried by a valid lease + current
  fencing token ([ADR-0009](./0009-lease-checkout-fencing.md)); illegal transitions are rejected, not
  merged. Because each transition has a single author, there is **no field-level status conflict**
  (SC4).
- **One canonical wire representation** for the status value resolves the `inprogress`/`in-progress`
  drift; the node and hive serialize identically (the mapping is applied once, at the boundary).

## Consequences

- Removes the last-write-wins status race and the silent-clobber window for status specifically;
  combined with [ADR-0007](./0007-single-inbound-channel-one-delete-one-conflict.md)'s dirty-guard,
  status is never lost.
- Canonicalizing the enum value is a wire-format/representation change touching both schemas
  (irreversible; covered in the [ADR-0011](./0011-hive-only-state-cutover.md) cutover mapping).
- The matrix is the contract P5 (dependency/priority) and P6 (management agent) build on.

## Alternatives considered

- **Keep last-write-wins, add a status-only conflict resolver** — rejected: still a merge of a field
  two parties write; the state machine removes the conflict at the source.
- **Single global status enum shared verbatim** — desirable but out of scope; this ADR fixes the
  *value representation* mismatch without consolidating the two schema definitions (hygiene, deferred).
