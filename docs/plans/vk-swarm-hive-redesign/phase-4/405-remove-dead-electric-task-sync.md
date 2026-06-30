---
id: "405"
phase: 4
title: Remove the dead ElectricTaskSyncService task-shape path
status: ready
depends_on: []
parallel: false
conflicts_with: []
files:
  - crates/services/src/services/electric_task_sync.rs
  - crates/services/src/services/mod.rs
  - crates/services/src/services/share.rs
irreversible: true
scope_test: "N/A"
allowed_change: delete
forbid_after: ["electric_task_sync", "ElectricTaskSyncService", "sync_project_tasks"]
covers_criteria: [SC7]
covers_tests: []
---
## Failing test (write first)
N/A — this is a pure deletion of dead code with zero runtime callers (verified below). No new behavior to
test; the gate's `forbid_after` greps prove every reference is gone and `cargo check -p services` proves
the crate still compiles. Coverage of the surrounding inbound-collapse behavior is provided by tasks
401–404; this task only removes the third (dead) apply path so it can never be revived (ADR-0007:
"finishing it would re-introduce a third apply path").

> ⚠️ IRREVERSIBLE (deletes a whole source file the executor did not author) — requires a human approval
> token `reviews/405.approved` before the gate runs.

## Verification of deadness (run BEFORE deleting — record in the ledger)
The deletion is safe ONLY if these all hold (they did at authoring; re-confirm):
1. `grep -rn "ElectricTaskSyncService\|sync_project_tasks" crates/ --include="*.rs" | grep -v "electric_task_sync.rs:"`
   → returns NOTHING (no external constructor or method caller). (Authoring run: empty.)
2. `grep -rn "electric_task_sync" frontend/src/` → NOTHING; the frontend Electric collections subscribe
   only to `nodes`/`projects`/`node_projects` shapes, never `shared_tasks` (ADR-0007, `collections.ts`).
3. `extract_uuid_from_key` (refs at electric_task_sync.rs:316/325/332) is used ONLY by that file's own
   `#[cfg(test)]` tests → confirmed by the grep in #1.
4. The SEPARATE `electric_sync` module (Electric Shape NDJSON client, mod.rs:7/57) is NOT this file and
   STAYS — only `electric_task_sync` (the task-shape poll) is dead. Do NOT touch `electric_sync`.

If ANY of 1–3 returns a real caller → STOP (it is not dead; escalate — do not delete).

## Change

### 1. `crates/services/src/services/electric_task_sync.rs` — delete the file
- **Command:** `git rm crates/services/src/services/electric_task_sync.rs`

### 2. `crates/services/src/services/mod.rs` — drop the module declaration + its doc lines
- **File:** `crates/services/src/services/mod.rs`
- **Anchor A:** the module declaration (~L59).
- **Before:**
```rust
// === Electric SQL Integration (New) ===
pub mod electric_sync;
pub mod electric_task_sync;
pub mod log_migration;
```
- **After:**
```rust
// === Electric SQL Integration (New) ===
pub mod electric_sync;
pub mod log_migration;
```
- **Anchor B:** the crate-doc bullet (~L8) — must go so `forbid_after: ["electric_task_sync"]` passes.
- **Before:**
```rust
//! - [`electric_sync`] - Electric Shape API client for parsing NDJSON responses
//! - [`electric_task_sync`] - Task sync service using Electric shapes
//! - [`log_migration`] - Migration from legacy JSONL logs to row-based log_entries
```
- **After:**
```rust
//! - [`electric_sync`] - Electric Shape API client for parsing NDJSON responses
//! - [`log_migration`] - Migration from legacy JSONL logs to row-based log_entries
```
- **Anchor C:** the deprecated-modules bullet (~L15) references the now-deleted module as a migration
  target — fix the stale guidance so `forbid_after` passes.
- **Before:**
```rust
//! - [`share`] - **DEPRECATED** - Use `electric_task_sync` instead
```
- **After:**
```rust
//! - [`share`] - **DEPRECATED** - WebSocket activity stream is the single live inbound channel (ADR-0007)
```

### 3. `crates/services/src/services/share.rs` — drop the stale "See Also" pointer (~L15)
- **File:** `crates/services/src/services/share.rs`
- **Anchor:** the `## See Also` doc block.
- **Before:**
```rust
//! ## See Also
//!
//! - `crates/services/src/services/electric_task_sync.rs` - Electric-based task sync from Hive

mod config;
```
- **After:**
```rust
mod config;
```

## Allowed moves
ONLY: `git rm` the one source file; remove its `pub mod` line + the two doc-bullets in `mod.rs`; remove
the `## See Also` Electric pointer in `share.rs`. Do NOT touch `electric_sync` (the live NDJSON client),
`log_migration`, `node_cache`, or any other module. Do NOT delete `share`/`processor` (that is the LIVE
channel tasks 401–404 build on).

## STOP triggers
- The deadness verification (above) finds ANY real caller of `ElectricTaskSyncService` /
  `sync_project_tasks` outside the deleted file → STOP; it is not dead. Escalate.
- Removing `pub mod electric_task_sync;` leaves a dangling `use …::electric_task_sync…` elsewhere
  (compile error) → there should be none (grep #1); if one appears, STOP — the file is referenced.
- `electric_sync` (without `_task`) gets caught by an over-broad edit → it must STAY; only the
  `electric_task_sync` token is removed. Re-check the diff.
- `reviews/405.approved` is absent when the gate runs → IRREVERSIBLE tasks require the human approval
  token first (schema invariant #4).

## Manual verification (record in decisions-ledger)
`scope_test` is `N/A` — a pure dead-code deletion has no behavior to unit-test; verify mechanically and
record each command's output in the ledger:
1. `cargo check -p services` → clean (the crate still compiles with the module gone; no dangling `mod`/
   `use`/re-export).
2. `git grep -nF electric_task_sync -- crates/ ':!docs/'` → ZERO hits (the `forbid_after` enforces this
   in the gate; this is the by-hand confirmation, doc-comments included).
3. `git grep -nF ElectricTaskSyncService -- crates/` and `git grep -nF sync_project_tasks -- crates/`
   → ZERO hits each.
4. `git status --porcelain crates/services/src/services/electric_task_sync.rs` shows `D` (deleted), and
   `git show HEAD:crates/services/src/services/mod.rs | grep -c electric_sync` is `2` (the SEPARATE
   NDJSON `electric_sync` module — at the doc bullet + the `pub mod` — is UNTOUCHED).

## Done when
`WAI_TYPECHECK_CMD="cargo check -p services" WAI_TEST_CMD="cargo test -p services --no-run" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-hive-redesign 405` exits 0
(the `forbid_after` greps for `electric_task_sync`/`ElectricTaskSyncService`/`sync_project_tasks` must return zero hits in the validated commit; `reviews/405.approved` must exist — IRREVERSIBLE.)
