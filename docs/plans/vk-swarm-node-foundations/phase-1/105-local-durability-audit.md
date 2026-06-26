---
id: "105"
phase: 1
title: Local-durability audit (findings note)
status: ready
depends_on: ["102"]
parallel: false
conflicts_with: []
files:
  - docs/plans/vk-swarm-node-foundations/notes/105-local-durability-audit.md
irreversible: false
scope_test: "N/A"
allowed_change: create
covers_criteria: [SC4]
---
## Failing test (write first)
N/A — this is an audit task producing a findings document, not code. Verified via the Manual
verification section below.

## Change
- **File:** `docs/plans/vk-swarm-node-foundations/notes/105-local-durability-audit.md` (NEW)
- **Anchor:** new findings note.
- **Content:** An enumerated audit (SC4) of every run/management state element, classifying each as
  **durable (on disk)** or **volatile (in-memory, lost on crash)**. At minimum inspect and record a
  verdict for each of:
  1. `task_attempts` / `execution_processes` / `executor_sessions` (DB — durable; confirm).
  2. The follow-up message queue (`MessageQueueStore`) — now DB-backed by task 102 (the first known
     hole, CLOSED; confirm).
  3. `child_store` + `msg_stores` in-memory HashMaps (`crates/local-deployment/src/container.rs:84-85`)
     — runtime handles, intentionally volatile; confirm recovery (Phase 3) reconstructs from disk
     rather than needing these.
  4. `EntryIndexProvider` / normalization handles / log batcher buffers — flushed on graceful shutdown
     (CLAUDE.md §Stopping); record crash behaviour.
  5. Any other `Arc<RwLock<HashMap…>>` / `Mutex` holding submitted-by-operator state (grep
     `crates/local-deployment/src` and `crates/services/src` for in-memory stores).
  For each: state element · where it lives · durable? · lost-on-crash impact · verdict.
- **For any NEWLY-FOUND durability hole** (operator-submitted state that is volatile and NOT covered by
  Phase 1): do NOT expand Phase 1. Record it as a backlog item — append to `dev-docs/BACKLOG.md` if it
  exists (per the backlog-ledger contract), else list it under a clear "## New holes → backlog"
  heading in this note with enough detail to file later. The audit's job is to find and record, not to
  silently grow this workstream.

## Allowed moves
ONLY create the findings note (and, if `dev-docs/BACKLOG.md` exists, append conformant backlog rows).
Do NOT change any source code — fixes for newly-found holes are separate future work.

## STOP triggers
- The audit finds a hole that BLOCKS crash-resume correctness (i.e. Phase 3 cannot be correct without
  it) → STOP and escalate to the user; this is a spec-scope question, not a silent Phase-1 addition.

## Manual verification (record in decisions-ledger)
- `test -f docs/plans/vk-swarm-node-foundations/notes/105-local-durability-audit.md` → exists.
- The note contains a verdict line for every element in the Change list (1–5) → confirm by reading.
- `grep -rn "Arc<RwLock<HashMap" crates/local-deployment/src crates/services/src` was run and every hit
  is accounted for in the note (durable or volatile-with-impact) → record the grep count vs note count.
- Record in the ledger: total elements audited, holes found, and where each hole was filed (backlog).

## Done when
The findings note exists, every audited element has a verdict, the in-memory-store grep is reconciled,
and any new hole is filed to backlog — all recorded in the decisions-ledger. (No gate script; this is a
`## Manual verification` task per schema.)
