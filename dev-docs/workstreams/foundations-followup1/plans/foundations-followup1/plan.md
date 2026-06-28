---
topic: foundations-followup1
doc_type: plan
status: draft
spec: docs/superpowers/specs/2026-06-27-foundations-followup1.md
---

# Plan — foundations-followup1

## Approach

Close the three test-coverage gaps Phase 2a (`vk-swarm-node-foundations`) shipped deliberately:
the SC1 end-to-end crash-resume path, the D-state escalation path, and the SC2 boot-drain call
path. Every change is additive — no Phase 2a production logic is altered except the two prescribed
exceptions (a `fence_attempt_count` column + its CouldNotKill escalation in `cleanup_orphan_executions`,
both covered by ADR-0005).

The work is sequenced so the test seams exist before the tests that need them. Phase 1 lands the DB
column + scalar accessors (the escalation in Phase 2 calls them). Phase 2 adds the
`make_process_inspector` injection hook + a from-scratch `TestContainerService` (the only way to drive
the real `cleanup_orphan_executions` trait method from the `services` crate — the sole real impl,
`LocalContainerService`, lives downstream and cannot be imported), then reuses that harness for the
D-state escalation test. Phase 3 is fully independent: it constructs a real `LocalContainerService`
in-test and observes the boot-drain call path via a `#[cfg(test)]` spy channel.

Every task is Rust. This repo is a Cargo workspace, so the WAI gate's TypeScript type-check is
skipped and its `scope_test` runner has no native `cargo test` path — every task carries explicit
`WAI_TYPECHECK_CMD`/`WAI_TEST_CMD` overrides in its `## Done when` line (decisions-ledger Trap 1).
Task 102 adds `query!` macros and therefore requires `DATABASE_URL` pointed at a live migrated dev
DB at compile time, and a single `cargo sqlx prepare` regeneration at `/wai:close` (decisions-ledger
Trap 2). Task 101 is migration-only; its verification is the migration run plus the db test gate. Anchors were authored against current `main` and re-verified by reading
the live tree during decompose.

Phase dependency spine: **P1 (DB) → P2 (services tests)**; within P2, **201 → 202** (202 reuses
201's `TestContainerService` and 102's accessors). **P3 (boot-drain)** is independent of the spine.

## Phases

1. **phase-1-db-foundation** — add the `fence_attempt_count` column (migration) and its
   increment/read scalar accessors (SC2b).
2. **phase-2-services-tests** — add the `make_process_inspector` injection hook + `TestContainerService`
   and the SC1 end-to-end resume test (201, SC1a–c); then the CouldNotKill escalation logic + the
   stubborn-PID escalation test (202, SC2a/SC2c/SC2d/SC2e). Depends on P1.
3. **phase-3-boot-drain-test** — add a `#[cfg(test)]` drain spy + minimal real-instance constructor
   and the full boot-drain call-path test (SC3a–d). Independent.

## Task table

`dep:`/`conflicts:` mirror each task's frontmatter (wai-plan-lint enforces equality). `-` = none.

### Phase 1 — db foundation

| id  | title | dep | conflicts | SC |
|-----|-------|-----|-----------|----|
| 101 | Add `fence_attempt_count` column migration on `execution_processes` | dep: - | conflicts: none | SC2b |
| 102 | Add `increment`/`get` fence_attempt_count scalar accessors + test | dep: 101 | conflicts: none | SC2b |

### Phase 2 — services tests

| id  | title | dep | conflicts | SC |
|-----|-------|-----|-----------|----|
| 201 | Add `make_process_inspector` hook + `TestContainerService` + SC1 resume test | dep: - | conflicts: 202 | SC1a, SC1b, SC1c |
| 202 | CouldNotKill escalation (counter + warn) + stubborn-PID escalation test | dep: 102, 201 | conflicts: 201 | SC2a, SC2c, SC2d, SC2e |

### Phase 3 — boot-drain test

| id  | title | dep | conflicts | SC |
|-----|-------|-----|-----------|----|
| 301 | Drain spy + `new_for_drain_test` + boot-drain call-path test | dep: - | conflicts: none | SC3a, SC3b, SC3c, SC3d |

## SC coverage map

Every spec success-criterion id is claimed by exactly one task:

- **SC1a, SC1b, SC1c** → 201
- **SC2a** → 202 (satisfied by existing `MockProcessInspector::set_unkillable`; 202's test exercises it)
- **SC2b** → 101 (column) + 102 (accessors)
- **SC2c, SC2d, SC2e** → 202
- **SC3a, SC3b, SC3c, SC3d** → 301
- **CI gate** (clippy/test/lint/tsc) → enforced by every task's `## Done when` gate; no frontend
  changes so lint/tsc are unaffected.
