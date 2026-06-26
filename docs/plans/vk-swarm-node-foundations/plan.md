---
topic: vk-swarm-node-foundations
doc_type: plan
status: draft
spec: docs/superpowers/specs/2026-06-26-vk-swarm-node-foundations.md
---

# Plan — vk-swarm-node-foundations

## Approach

Make a single vk-swarm node correct, durable, and crash-resumable **standalone**, by changing four
areas that never touch the node↔hive sync *protocol* (that is `vk-swarm-hive-redesign`). The work is
sequenced so each phase is independently shippable and so the **test keystone exists before the code
that needs it**: durable state schema lands first (recovery reads the resume-intent it adds), the
`qa_mock` deterministic executor lands next (crash-resume cannot be tested against a live agent CLI),
then fence-then-resume recovery is built and tested against it. UI strip-back and the remaining
stability forward-ports are independent and ship in their own phases.

Every task is Rust (or a CI/script/frontend task). **This repo is a Cargo workspace, so the WAI gate's
TypeScript type-check is skipped and its `scope_test` runner has no native `cargo test` path** — every
Rust task therefore carries explicit `WAI_TYPECHECK_CMD`/`WAI_TEST_CMD` overrides in its `## Done when`
line (see decisions-ledger trap #1). Anchors were authored against current `main`; the adversarial
breakdown review re-verifies each against the live tree before any code is written.

Phase dependency spine: **P1 (schema) → P3 (recovery)** and **P2 (qa_mock) → P3 (recovery)**; **P4
(UI)** and **P5 (forward-ports)** are independent of the spine and of each other.

## Phases

1. **phase-1-durable-state** — persist the message queue, add the resume-intent marker + assembling
   view, audit local durability (SC2, SC3, SC4).
2. **phase-2-qa-mock** — forward-port the `qa_mock` deterministic executor and wire it into the
   `CodingAgent` enum; the test keystone for recovery (SC7d). Independent of P1.
3. **phase-3-recovery** — fence-then-resume crash recovery, ordered before failure-marking, with the
   executor-capability fallback policy (SC1, SC1-fallback, SC8). Depends on P1 (resume-intent column)
   and P2 (qa_mock for tests).
4. **phase-4-ui-local-only** — scope the node UI to local work + read-only hive-sync view; delete
   remote-display surfaces (SC5, SC6). Independent.
5. **phase-5-forward-ports** — ACP bounded channels, WAL-monitor panic supervision, npm runtime-vuln
   CI gate (SC7a, SC7b, SC7c). Independent.

## Task table

`dep:`/`conflicts:` mirror each task's frontmatter (wai-plan-lint enforces equality). `-` = none.

### Phase 1 — durable state

| id  | title | dep | conflicts | SC |
|-----|-------|-----|-----------|----|
| 101 | Add `queued_messages` table migration | dep: - | conflicts: none | SC2 |
| 102 | Back MessageQueueStore with `queued_messages` + boot drain | dep: 101 | conflicts: none | SC2 |
| 103 | Add resume-intent column migration on `execution_processes` | dep: - | conflicts: none | SC3 |
| 104 | Add assembling view over attempts+processes+sessions | dep: 103 | conflicts: none | SC3 |
| 105 | Local-durability audit (findings note) | dep: 102 | conflicts: none | SC4 |

### Phase 2 — qa_mock

| id  | title | dep | conflicts | SC |
|-----|-------|-----|-----------|----|
| 201 | Forward-port `qa_mock` executor + wire into `CodingAgent` (single commit) | dep: - | conflicts: none | SC7 |

> **Why one `mixed` task, not two:** Rust will not compile an unreferenced `qa_mock.rs` (`mod qa_mock;`
> with no consumer trips `-D warnings`), and the `CodingAgent` match arms (capabilities, mcp_config,
> follow-up, initial dispatch) must be **exhaustive** to compile. A "create file" half and a "wire it"
> half cannot each be green in isolation, so they are one cohesive forward-port (mirror the upstream
> registration diff verbatim).

### Phase 3 — recovery

| id  | title | dep | conflicts | SC |
|-----|-------|-----|-----------|----|
| 301 | Executor resume-capability audit (capability map) | dep: - | conflicts: none | SC1 |
| 302 | Process fence primitive built on existing `ProcessInspector` | dep: - | conflicts: none | SC1 |
| 303 | Reconstruct ExecutorAction + resume re-entry helper | dep: - | conflicts: 304 | SC1 |
| 304 | Rewrite `cleanup_orphan_executions` to fence-then-resume incl. fallback (before mark-failed) | dep: 301 302 303 104 | conflicts: 303 | SC1, SC8 |
| 305 | Boot-drain persisted message queue for non-resumed attempts | dep: 102 304 | conflicts: none | SC2 |

> **Fallback folded into 304 (no separate task):** the non-resumable-executor fallback (SC1-fallback)
> is a *branch* of the recovery routine, not a separate edit — splitting it would mean a second task
> re-editing the same function 304 just wrote. 304 covers fence → resume → cold-respawn-or-mark-failed
> → fail-last in one cohesive rewrite, informed by 301's capability map.
>
> **305 closes the SC2 drain-on-resume gap** (breakdown-review R1): 102 makes the queue *persist*, but
> the live drain (`container.rs:738`) only fires on a process exit — a crash emits none. 305 runs after
> recovery (304) and starts queued messages for attempts that are NOT being resumed. So SC2 spans P1
> (durability) + P3 (drain), an honest P1↔P3 coupling.

### Phase 4 — UI local-only

| id  | title | dep | conflicts | SC |
|-----|-------|-----|-----------|----|
| 401 | Visibility discriminator in `find_by_project_id_with_attempt_status` | dep: - | conflicts: none | SC5 |
| 402 | Remove request-time remote merge in `get_tasks` | dep: 401 | conflicts: none | SC5 |
| 403 | Remove pure-proxy remote API modules (nodes, swarm_*, merged-projects) | dep: - | conflicts: none | SC5 |
| 404 | Delete frontend Nodes-management feature | dep: - | conflicts: none | SC5 |
| 405 | Read-only hive sync-status view | dep: - | conflicts: none | SC5 |
| 406 | Standalone-run + UI-always-on verification | dep: 402 404 405 | conflicts: none | SC6 |

> **Phase 4 is the clean local-only subset (user-approved scope split, 2026-06-26).** It makes the node
> local-only at the data / API / dedicated-feature layer. The *entangled* remote-display removal
> (`useMergedProjects` repoint, remote card badges, dual-purpose stream/diff hooks) is a separate
> frontend refactor carved into the **`vk-swarm-node-ui-localize`** workstream — see decisions-ledger
> and `dev-docs/workstreams/vk-swarm-node-ui-localize/README.md`.

### Phase 5 — forward-ports

| id  | title | dep | conflicts | SC |
|-----|-------|-----|-----------|----|
| 501 | ACP bounded channels (drop-on-full) | dep: - | conflicts: none | SC7 |
| 502 | WAL-monitor panic supervision | dep: - | conflicts: none | SC7 |
| 503 | npm runtime-vuln CI gate | dep: - | conflicts: none | SC7 |

## Irreversible tasks (🚧 human gate)

- **403** — removes public API routes (`/api/nodes*`, `/api/swarm/*`, `/api/merged-projects`) → contract change.
- **404** — deletes frontend code (Nodes feature, remote hooks/components) we did not author standalone.

(101/103/104 add forward-only migrations but are additive — not destructive — so they run as normal
`create` tasks; their irreversibility is documented in ADR-0003, not gated per-task.)

## SC coverage map (enforced ids SC1–SC8)

SC1→{301,302,303,304} · SC2→{101,102,305} · SC3→{103,104} · SC4→{105} · SC5→{401,402,403,404,405} ·
SC6→{406} · SC7→{201,501,502,503} · SC8→{304}. (`SC1-fallback` is covered substantively by 301/304
under the SC1 umbrella; the lint cannot parse its hyphenated id and does not enforce it separately.)
