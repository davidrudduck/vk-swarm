---
topic: vk-swarm-hive-redesign
doc_type: plan
status: draft
spec: docs/superpowers/specs/2026-06-26-vk-swarm-hive-redesign.md
---

# Plan — vk-swarm-hive-redesign

## Approach

Rebuild the node↔hive reconciliation contract as a hub-and-spoke control plane, keeping the storage
engines (Postgres hive store, SQLite node-of-record, WS transport). The work spans **both ends of a
network protocol** and is sequenced so the **foundation flows end-to-end before anything rides it**:
the ordered, acknowledged op-log (SC2) is the channel every later guarantee depends on, so it ships
first as a tracer bullet — one op type flowing node→hive with a durable ack — then the remaining op
types, then the mechanisms that ride it (fencing, status machine, anti-entropy). The inbound-channel
collapse (SC7) and the cutover migration (SC6) are comparatively independent and ship in their own
phases.

Every task is Rust (or a migration/CI/frontend task). **This repo is a Cargo workspace, so the WAI
gate's TypeScript type-check is skipped and its `scope_test` runner has no native `cargo test` path** —
every Rust task carries explicit `WAI_TYPECHECK_CMD`/`WAI_TEST_CMD` overrides (decisions-ledger Trap 1).
**SQLx is offline-mode** (committed `.sqlx` cache, `DATABASE_URL` unset): schema/query tasks execute
against a live migrated dev DB and never run `cargo sqlx prepare` in a gated task (Trap 2). Adding any
`NodeMessage`/`ServerMessage` WS variant (the op-log/ack/heartbeat/lease messages) hits the
**enum-exhaustiveness trap on BOTH ends** (Trap 3) — every match arm in the same commit. Anchors were
authored against current `main`; the adversarial breakdown tournament re-verifies each before any code.

**This decompose covers the protocol + data plane (SC2–SC7 and the no-fan-out, data-plane half of
SC1).** The *hive central-management web UI* half of SC1 is a carve candidate (see Scope note) decided
with the user before task authoring, mirroring the node-foundations → `vk-swarm-node-ui-localize` split.

Phase dependency spine: **P1 (op-log) → P2 (fencing) → P3 (status machine)**; **P1 → P5 (anti-entropy)**;
**P4 (inbound collapse)** is independent of the spine; **P7 (cutover migration)** depends on the
rebuilt schema (P1–P3); **P6 (no fan-out)** is data-plane and independent.

## Phases

1. **phase-1-oplog** — node→hive single ordered, acknowledged op-log/outbox: outbox table, op append on
   local writes, WS stream in seq order, hive idempotent apply + durable ack, cursor-advance-on-ack,
   parent-before-child parking. Replaces the five push paths (SC2). *Foundation.*
2. **phase-2-lease-fencing** — assignment via atomic checkout + lease (`lease_expires_at`), heartbeat
   renewal, monotonic fencing token on grants and on op-log ops, hive stale-token rejection, node
   self-fencing (reuses the ADR-0001 process fence). Discharges node-foundations D7 (SC3). Depends P1.
3. **phase-3-status-machine** — explicit `task.status` transition matrix, single-author enforcement
   (hive vs node, gated on lease+token), status enum value canonicalization (SC4). Depends P1, P2.
4. **phase-4-inbound-collapse** — WS activity stream as the single live inbound channel; bulk snapshot
   demoted to reconcile-only; remove the dead Electric task-shape path; one delete semantic
   (soft-unlink + tombstone); node dirty-guard conflict policy; handle `task.reassigned` (SC7).
5. **phase-5-anti-entropy** — digest exchange (per-entity version/hash + outbox high-water) over the
   op-log, gap heal replacing manual `reset_*` migrations (SC5). Depends P1 (+P4 reconcile leg).
6. **phase-6-no-fanout** — **verify + guard** that nodes receive only assignment/ack traffic, never
   pushed shared-task state (SC1 data-plane half). Investigation found this is *already* true today —
   the node-facing channel carries only `ProjectSync`/`NodeRemoved` — so phase-6 asserts and fences the
   invariant against regression; it is NOT a large fan-out removal. Browser-facing shared-task fan-out
   (electric_proxy/ActivityBroker) is the hive-UI data source and is OUT of scope. Depends P1, P2.
7. **phase-7-cutover** — one-time hive-only-state migration (migrate / regenerate / discard per
   ADR-0011), preserving the node↔hive id bridge and remapping status enum values (SC6). Depends P1–P3.

## Scope note — SC1 central-management UI CARVED to `vk-swarm-hive-ui`

SC1 has two halves: (a) the **data-plane** guarantee "no node↔node / node↔hive↔node fan-out" (covered
here, phase-6), and (b) the **hive web UI manages all** nodes/projects/tasks/attempts/executions.
**User decision (2026-06-30): half (b) is carved into `vk-swarm-hive-ui`** (tracker seeded; rehost +
rewire + net-new cross-node views — see that README and the decisions ledger). This plan covers SC2–SC7
+ SC1's data-plane half. SC1 stays in the frozen spec, claimed by phase-6; no spec edit was made.

## Task table

> **PROVISIONAL — finalized after the hive-side anchor investigation and the user scope decision.**
> `dep:`/`conflicts:` will mirror each task's frontmatter (wai-plan-lint enforces equality) once the
> task files are authored. The phase shape and SC mapping below are the committed structure.

(Per-phase task rows are authored into the phase files after anchor verification; this table is
completed in lockstep so the lint's plan==frontmatter equality holds.)

## Execution preconditions & closeout (READ — affects whether the gate passes)

- **Rust gate overrides:** every Rust task sets `WAI_TYPECHECK_CMD="cargo check -p <crate>"` (or
  `--workspace`) and `WAI_TEST_CMD="cargo test -p <crate> <test>"` in its `## Done when` (Trap 1).
- **SQLx offline:** tasks adding/changing a `query!`/`query_as!` export
  `DATABASE_URL=sqlite://<repo>/dev_assets/db.sqlite` against a migrated dev DB; **no task runs
  `cargo sqlx prepare`** (Trap 2). The hive (Postgres) side has its own offline considerations — see
  the trap ledger's remote-crate analogue.
- **Closeout (NOT a gated task):** after all schema/query tasks land, regenerate the `.sqlx` cache once
  and commit it as a standalone housekeeping commit at `/wai:close`.

## SC coverage map (enforced ids SC1–SC7)

SC1→{phase-6 no-fanout (data-plane half); UI half carve-gated} · SC2→{phase-1} · SC3→{phase-2} ·
SC4→{phase-3} · SC5→{phase-5} · SC6→{phase-7} · SC7→{phase-4}. (Clause sub-ids SC2a/b/c, SC3 fencing,
SC5d-style negatives are mapped to specific tasks at authoring time.)
