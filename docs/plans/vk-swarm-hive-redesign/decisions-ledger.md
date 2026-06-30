---
topic: vk-swarm-hive-redesign
doc_type: decisions-ledger
---

# Decisions ledger — vk-swarm-hive-redesign

Appended during spec/precheck and (later) decompose/execute. Pre-empted traps for implementers are
added at decompose time.

## Pre-empted traps (read before executing ANY task)

Carried forward from `vk-swarm-node-foundations` (same repo, same gate) + hive/Postgres analogues. The
node-foundations ledger is the authority for the originals; condensed here.

### Trap 1 — WAI gate is TypeScript-shaped; this is a Cargo workspace
Every Rust task sets BOTH in its `## Done when`: `WAI_TYPECHECK_CMD="cargo check -p <crate>"` (or
`--workspace` when arms span crates) and `WAI_TEST_CMD="cargo test -p <crate> <test_name>"`. Omitting
them runs the wrong/no check — a hollow pass.

### Trap 2 — SQLx OFFLINE on the node (SQLite) side; build against a live migrated DB
Committed `.sqlx` cache + unset `DATABASE_URL` → `query!` validates against the cache, not a live DB.
Tasks adding a migration + new query export `DATABASE_URL=sqlite://<repo>/dev_assets/db.sqlite` against
a migrated dev DB. **Never run `cargo sqlx prepare` in a gated task** (it rewrites tracked `.sqlx/*.json`
the gate's file-allow-list rejects). Regenerate once at `/wai:close`.

### Trap 2b — Hive (Postgres / `crates/remote`) query validation + tests need a LIVE Postgres (confirmed)
Confirmed in code: the remote crate's `query!`/`query_as!` validate against the **shared root `.sqlx`
offline cache** (same cache as the node side). Adding/changing a Postgres query therefore needs a live
`DATABASE_URL`/`SERVER_DATABASE_URL` pointed at a **migrated Postgres** (`crates/remote/src/config.rs:45`)
OR a cache regen — it will NOT validate against the SQLite dev DB. **And hive integration tests SKIP
without a DB:** `crates/remote/tests/backfill_e2e.rs` uses `skip_without_db!` (returns early if
`DATABASE_URL` unset); the hermetic SQLite `db::test_utils::create_test_pool()` does **not** apply to
the hive. **Consequence for every hive-side task (102, 107, and all of P2/P3/P6/P7 hive work):** its
`scope_test` is a `#[tokio::test]` that REQUIRES `DATABASE_URL=postgres://…` against a migrated PG, or
the WAI gate runs a SKIPPED test and reports a **hollow pass**. The executor MUST stand up a Postgres
(migrated via `sqlx::migrate!("./migrations")`, `crates/remote/src/db/mod.rs:51`) and export
`DATABASE_URL` before running any hive task. A hive task with no Postgres precondition + a skip-guarded
test is NOT verifiable — flag such tasks. (This is genuinely harder than node-foundations, which never
touched the remote crate. If a CI Postgres is not readily available, raise it before executing P-hive
tasks.)

### Trap 3 — `enum_dispatch` / WS-message exhaustiveness on BOTH ends
Adding a `NodeMessage` or `ServerMessage` variant (op-log op, durable ack, heartbeat, lease grant)
forces editing **every** match arm on that end in the SAME commit or the workspace won't compile under
`-D warnings`. The node side (`hive_client.rs`/`hive_sync.rs`/`node_runner.rs`) AND the hive side
(`crates/remote` WS dispatch) each have their own exhaustive match sites — a new variant is a single
cohesive `mixed` task per end (or one cross-cutting task), not a create+wire split. Anchor list of arms
is filled from the hive-side anchor investigation.

### Trap 4 — Know which crate owns the logic (no cross-crate inversion)
`services` does not depend on `local-deployment`; trait methods dispatch into the impl. Confirm the
owning crate/trait for each touched function before authoring (the node-foundations recovery work lived
in the `services` trait, not `local-deployment`). For hive logic, confirm `crates/remote` ownership.

### Trap 5 — Anchor-checker strips `crates/*/` (precheck false-positive) — already hit
See Precheck notes below. Recorded so a precheck re-run is not mistaken for a real contradiction.

### Trap 6 — Frozen spec; escalate scope discoveries to the user
The spec is frozen at precheck (ADR-0001). A spec-vs-reality contradiction or an entangled/oversized
area discovered during decompose is **escalated to the user**, never papered over with a vague task or
a silent spec edit — exactly how node-foundations carved out `vk-swarm-node-ui-localize`. The SC1
central-UI carve (see plan Scope note) follows this protocol.

## Decompose decisions (2026-06-30)

### USER-APPROVED SCOPE SPLIT — SC1 central-management UI → `vk-swarm-hive-ui`
Decompose found SC1 has two separable halves: (a) the **data-plane** guarantee "no node↔node /
node↔hive↔node fan-out", and (b) the **hive web UI manages all** cross-node tasks/attempts/executions.
Half (b) is a workstream-scale frontend build (the hive UI today is a 4-page auth stub; the
management UI lives in the *node* frontend; cross-node task/attempt/execution views are net-new). **User
decision (2026-06-30): carve half (b) into `vk-swarm-hive-ui`** (tracker seeded, no spec yet). This
workstream covers SC2–SC7 + SC1's data-plane half (phase-6). SC1 stays in the frozen spec, claimed by
phase-6; the UI deferral is recorded here — **no spec edit** (mirrors the node-foundations →
`vk-swarm-node-ui-localize` carve).

### Finding — SC1 "no fan-out of shared task state" is LARGELY ALREADY SATISFIED
Verified in code: the hive's node-facing broadcast (`ConnectionManager::send_to_nodes`/`broadcast_to_org`,
`crates/remote/src/nodes/ws/connection.rs:138,189`) carries **only** `ProjectSync` and `NodeRemoved` —
it does **not** push shared-*task* state to nodes today. The shared-task fan-out that exists
(`electric_proxy.rs` `GET /v1/shape/shared_tasks` + `ActivityBroker`, `crates/remote/src/activity/broker.rs:48`)
targets **browsers** (which the hive UI needs). **Therefore phase-6 (SC1 data-plane) is "verify + guard
against regression + keep the node-facing channel assignment-only," not "remove a large fan-out."** This
is surfaced (not a contradiction): SC1's data-plane intent is met by construction; the task asserts and
fences it. Browser-facing fan-out (electric_proxy/broker) is OUT of scope for "no fan-out" — it is the
hive-UI data source.

### Decompose decision — WS protocol enums are hand-duplicated; DUAL-EDIT, do not deduplicate (this workstream)
`NodeMessage`/`HiveMessage` are defined **twice** — hive `crates/remote/src/nodes/ws/message.rs:15,91`
and node `crates/services/src/services/hive_client.rs:82,123` — and have **already drifted** (`Deregister`
exists hive-side only). Every new variant (op-log op, durable ack, heartbeat, lease grant) is added to
**both** copies in the same commit (Trap 3). **Decision:** keep the dual-definition convention and add
variants to both — deduplicating into a shared protocol crate is hygiene OUT of scope here (it would be
a large cross-crate move touching every match site). Each protocol task is a single cohesive `mixed`
task editing both crates + the exhaustive hive match (`session.rs:512`) + an explicit node arm **before
the `_ =>` wildcard at `hive_client.rs:1062`** (the wildcard silently drops unhandled hive→node variants
— the #1 easy-to-miss bug here).

### Decompose decision — durable-ack emission requires threading `ws_sender` into apply handlers
Hive apply handlers `handle_attempt_sync`/`handle_execution_sync`/`handle_logs_batch`
(`session.rs:1188,1307,1388`) currently take `node_id`/`pool` only — **not** the WS sender. Emitting a
durable per-op ack (the SC2c fix) requires threading `ws_sender` into those signatures. `handle_task_sync`
(session.rs:1547, replies `TaskSyncResponse`) and `handle_heartbeat` (replies `HeartbeatAck`,
session.rs:607) are the existing templates. The op-log apply is a NEW `handle_op_batch` handler beside
them; acks reuse the `HeartbeatAck` send shape.

### Decompose decision — PHASE-BY-PHASE authoring (user-approved)
This decompose authors **Phase 1 (the op-log foundation) only**; later phases are authored in subsequent
`/wai:decompose` passes as each prior ships (user decision 2026-06-30). Consequence for the gate:
`wai-plan-lint` reads ALL spec SC ids from the token and will report SC1,SC3–SC7 **unclaimed** until
their phases are authored — this is **expected-pending**, not a lint failure to paper over. Phase 1's
internal consistency (plan↔frontmatter deps/conflicts, verification present, failing-tests-first) IS
enforced now. A full PLAN-LINT PASS is achieved only when the final phase is authored.

## Precheck notes

### Anchor-check false positive (resolved — `--no-anchor-check` used)
`wai-precheck.sh` assert #3 (path anchors) flagged `src/db/tasks.rs` and `src/activity/broker.rs` as
"ABSENT on main". This is a **false positive**: the extractor regex
`(src|extensions|ui|packages|apps)/…` truncates any `crates/*/src/*` path to its `src/…` substring and
tests it at repo root. The real files **exist on main** — verified directly:
`git cat-file -e main:crates/remote/src/db/tasks.rs` and `…/crates/remote/src/activity/broker.rs` both
succeed. These are the only two path tokens the regex extracts from this spec; both were manually
grounded. The sibling `vk-swarm-node-foundations` spec (shipped) produces the identical truncation
pattern. Precheck was therefore re-run with `--no-anchor-check`; the spec is frozen against
`spec_sha` in `.precheck.passed`.
