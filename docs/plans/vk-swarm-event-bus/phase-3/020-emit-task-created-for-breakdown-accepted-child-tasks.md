---
id: "020"
phase: 3
title: "Emit TaskCreated for the child tasks a breakdown acceptance creates"
status: ready
depends_on: ["004","006"]
parallel: false
conflicts_with: []
files:
  - "crates/db/src/models/task_breakdown/queries.rs"
  - "crates/db/src/models/task_breakdown/mod.rs"
  - "crates/services/tests/event_bus_end_to_end.rs"
siblings:
  - "crates/db/src/models/task/queries.rs"
irreversible: false
scope_test: "crates/db"
allowed_change: edit
covers_criteria: []
covers_tests: []
---

## Why this task exists

SC1 quantifies over task creation universally:

> SC1: On a running node, **creating**, moving (status change), and deleting a task **each produce a
> journaled event** with a monotonic seq...

Task 006 instruments four functions in `crates/db/src/models/task/`.
`task_breakdown::accept_proposal` (`queries.rs:406`) creates **real child tasks** with its own
`INSERT INTO tasks`, is routed at `crates/server/src/routes/breakdown.rs:273`, and is
user-initiated. Without this task, accepting a breakdown proposal creates tasks that emit nothing,
and phase 3 cannot honestly claim SC1.

The plan missed it because `task_breakdown` merged in PR #475 on 2026-08-11, concurrent with this
workstream's `/wai:decompose`. The spec's Design enumerates "task CRUD in
`crates/db/src/models/task/`" — a directory `task_breakdown/` is not in. Escalated to and approved
by the spec owner on 2026-08-15 as a plan gap rather than a spec amendment: SC1 already demands this,
so the Design's site list was incomplete rather than contradicted.

## Failing test (write first)

**File:** `crates/db/src/models/task_breakdown/queries.rs`, extending the colocated
`#[cfg(test)] mod tests` in `crates/db/src/models/task_breakdown/mod.rs` if that is where the
existing acceptance tests live — read them first and follow their setup.

1. `accepting_a_proposal_journals_one_task_created_per_child` — build a proposal with **three**
   items, accept it, and assert `event_journal` contains exactly three `TaskCreated` rows whose
   `task_id`s are exactly the three returned child ids, as a SET. Not a count alone: a count passes
   if the same id is journaled three times.
2. `a_failed_acceptance_journals_nothing` — force the acceptance to abort AFTER the children have
   been inserted and their events APPENDED, and assert `event_journal` has zero `task_created`
   rows. This is the journal-first property that makes the whole design safe: the append rides the
   same transaction, so a rollback takes the events with it.
   (Amended 2026-08-16: the original parenthetical offered "a proposal whose status is not Draft"
   as a mechanism — that contradicts the main clause, because the status check fails BEFORE any
   child insert, so the test never exercises the rollback property at all; the append could sit
   entirely outside the transaction and still pass. Dictated mechanism instead: build a Draft
   proposal whose items carry a dependency (e.g. B depends on A) so `accept_proposal`'s SECOND
   pass must write `task_dependencies` (`queries.rs:480-516`) — which runs after the first pass
   has inserted every child AND appended every event. Before accepting, issue
   `ALTER TABLE task_dependencies RENAME TO task_dependencies_hidden` as a plain statement on the
   pool outside any transaction. Accept → must Err. Rename the table BACK. Then assert BOTH:
   `COUNT(*) FROM event_journal WHERE event_type = 'task_created'` == 0, AND
   `COUNT(*) FROM tasks WHERE parent_task_id = ?` == 0 for the parent — the rollback removed the
   children AND their journaled events together.)

**Query the journal directly** (`SELECT ... FROM event_journal`) rather than subscribing to a bus.
This task is about EMISSION; going through the tailer would make a publication bug look like an
emission bug. That is the same rule task 015 follows — read its `## Change` section for the exact
reasoning before writing your assertions.

## Change

**File:** `crates/db/src/models/task_breakdown/queries.rs`
**Anchor:** inside `accept_proposal`'s first pass, immediately after the child `INSERT INTO tasks`
returns its `task` and alongside the existing outbox enqueue.

`accept_proposal` already owns its transaction (`let mut tx = pool.begin().await?;` at the top, every
statement on `&mut *tx`), and `event_journal::append` is generic over
`E: Executor<'e, Database = sqlx::Sqlite>` (`crates/db/src/models/event_journal/queries.rs:18-21`).
So the append composes directly on the transaction handle — the same shape `Task::delete` uses. **Do
not open a nested transaction and do not append after the commit.**

**After:** for each created child, append a `NodeEvent::TaskCreated { task_id, project_id }` on
`&mut *tx`, using the same field values the row was inserted with (`task.id`, `parent.project_id`).
Propagate the error with `?` exactly as the adjacent outbox INSERT does — the file's own comment
records that acceptance is all-or-nothing and errors abort the accept. An event that cannot be
journaled must abort the acceptance for the same reason.

**Read the sibling first.** `crates/db/src/models/task/queries.rs` is where task 006 instruments
`Task::create`. Read what task 006 did there — event construction, field sourcing, error handling —
and follow it. Record any deliberate divergence in the ledger. The two sites create the same entity
and must produce indistinguishable events; a consumer must not be able to tell whether a task came
from `Task::create` or from a breakdown acceptance.

## Allowed moves

- ONLY the append (plus its tests) in `task_breakdown/queries.rs`. Do NOT restructure
  `accept_proposal`, do NOT touch the outbox enqueue, do NOT alter the proposal state machine.
- **Do NOT write any new `sqlx::query!`/`query_as!` macro query.** `event_journal::append` is already
  in the offline cache from task 004; calling it adds no new SQL. If you believe you need
  `cargo sqlx prepare`, that is a STOP — its `crates/db/.sqlx/query-<hash>.json` output cannot be
  declared in `files:` and would be silently left unstaged (see the ledger for task 004, and
  agent-plugins issue #105).

## STOP triggers

- `accept_proposal` turns out NOT to own its transaction, or a child insert happens outside it.
  Re-read before writing; the whole task depends on this.
- Task 006 has not landed, or landed with a different event shape than this task assumes. This task
  `depends_on` 006 precisely so the two sites match — if 006's `TaskCreated` construction differs
  from what is described above, follow 006 and report the difference rather than inventing a second
  shape.
- The acceptance path turns out to create tasks in more than one place, or a second breakdown route
  creates tasks. Enumerate with `git grep -n "INSERT INTO tasks" -- 'crates/db/**' 'crates/server/**'`
  and STOP with the list.
- Any test needs `#[ignore]` to pass.

## SECONDARY — two remediations inherited from panel 14 on task 019

Unrelated to the emission work; here because CLAUDE.md forbids carrying a finding into a later
session. `crates/services/tests/event_bus_end_to_end.rs` is in this task's `files:` **solely** for
the first of these.

### Fix 1 — a comment task 019 falsified in the file it edited

`crates/services/tests/event_bus_end_to_end.rs:180` still reads:

```text
/// The warm-up commit/receive pair is not part of the property under test — it exists solely to
/// provably exhaust `subscribe_from`'s one-time journal replay window (see the file header)
```

Fourteen lines below it, the comment task 019 added explains the warm-up's *second* purpose — giving
the tailer a non-zero initial mark, which is the entire point of task 019. "Solely" was true before
the restructure and false after it.

Name **both** purposes in that sentence, or drop "solely". Nothing else about the test changes.

### Fix 2 — a coverage residency that is currently accidental

No code change. Panel 14 proved that task 019's restructure moved a defect class rather than only
adding one: the "one-shot publisher" mutation (publishes the first row it ever tails, then never
again, cursor still advancing) was caught by test 1 in the old shape (2/2) and is missed by test 1 in
the new shape (passes in 0.25s). **Suite coverage is retained — test 2 still catches it** (`timed out
... waiting for seq 4`).

Record in the ledger that this class now lives in test 2, so the residency is deliberate and a later
reader does not assume test 1 is strictly stronger than before. If you can construct a cheap
assertion that returns the class to test 1 without disturbing its M3/M7 kills, report it with
evidence — but do NOT change test 1 speculatively; its 4/4 kills are the deliverable of 019 and were
verified three times independently.

## Done when

`WAI_TYPECHECK_CMD="cargo fmt --all -- --check && cargo check --workspace" WAI_TEST_CMD='cargo test -p "$(basename {scope})"' bash <resolved wai>/scripts/task-gate.sh vk-swarm-event-bus 020` exits 0.

(The `~/.claude/wai/scripts/` path other task files cite does not resolve on this machine; the
orchestrator runs the gate with the plugin-cache path. See the ledger note of 2026-08-15.)
