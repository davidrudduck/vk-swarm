# Code Review — Round 1

**Target:** `fix/preexisting-gate-failures` (PR #452)   **Range:** `9b69c455..399ef99f`   **Effort:** high

Two parallel finder subagents reviewed the 32 changed code files (test helpers, source doctests,
Cargo.toml manifests) along correctness and quality axes. Every candidate finding was verified
against the real repo before reporting.

## Findings

| # | File:line | Severity | Category | Finding | Confidence | Actionable? |
|---|-----------|----------|----------|---------|-----------|-------------|
| T1 | `crates/remote/tests/hive_cutover_reingest.rs:37` | medium | correctness | `hive_cutover_reingest` INSERTs into `node_task_attempts` (a TRUNCATE target in `hive_cutover_migration.rs:23`) but was NOT annotated `#[file_serial]`. Cross-binary lock conflict with the cutover TRUNCATE when `DATABASE_URL` is set. `hive_cutover_must_migrate` (touches `activity`, also TRUNCATEd) had the same gap. | high | yes |
| F1 | `crates/remote/src/routes/nodes.rs:39` | low | quality | `api_key_router` doctest marked `rust,ignore` but the fn is `pub`, takes no args, and needs no `AppState` — trivially runnable. Missed revival opportunity. | high | yes |
| F2 | `crates/remote/src/routes/tasks.rs:191` | low | quality | `create_shared_task` doctest marked `rust,ignore` but body only constructs a `CreateSharedTaskRequest` struct literal (all `pub` fields) — runnable with one import. | high | yes |
| F3 | `crates/remote/src/routes/mod.rs:46` | low | quality | `router` doctest marked `rust,ignore` but `router` and `AppState` are `pub`; `no_run` gives compile-coverage without running `unimplemented!()`. | medium | yes |

## Remediation applied in this session

- **T1**: Added `#[file_serial]` + `use serial_test::file_serial;` to `hive_cutover_must_migrate.rs` (1 test, touches `activity`) and `hive_cutover_reingest.rs` (1 test, touches `node_task_attempts`). `lease_partition_e2e.rs` verified NOT to touch any TRUNCATEd table — correctly left unannotated.
- **F1**: `api_key_router` doctest made live — changed `rust,ignore` → ` ``` `, added `use remote::routes::nodes::api_key_router;`.
- **F2**: `create_shared_task` doctest made live — changed `rust,ignore` → ` ``` `, added `use remote::routes::tasks::CreateSharedTaskRequest;`, added missing `label_ids: None` field discovered during verification.
- **F3**: `router` doctest promoted to `no_run` — changed `rust,ignore` → `rust,no_run`, fixed `crate::` → `remote::` imports.

Doctest counts after remediation: `cargo test --doc -p remote` = 9 passed, 27 ignored (was 6/30). `cargo test --doc -p services` = 1 passed, 5 ignored (unchanged). Workstream tracker
(`dev-docs/workstreams/remote-services-doctest-revival/README.md`) and decisions-ledger updated to
reflect 32 remaining ignored doctests (was 35).

## Non-actionable

| # | File:line | Severity | Category | Finding | Confidence | Why non-actionable |
|---|-----------|----------|----------|---------|-----------|---------------------|
| T2 | `crates/services/Cargo.toml:5` | low | quality | Stray blank line; pre-existing unused `serial_test` dev-dependency. | high | Cosmetic / pre-existing; removing the unused dep expands the diff without fixing a gate failure. |
| — | `crates/remote/src/nodes/service.rs:804` | — | — | Codex round-2 claimed `unlink_swarm_project` doctest breaks gate (`crate::` private). | — | Already dismissed in adversarial round 2 — `cargo test --doc -p remote` confirms `line 804 ... ok`. Doctest defines `async fn` but never calls it; `crate::` resolves in rustdoc context. False positive. |

## Verdict: Approve

Gate verification (all green, no `--skip` flags):
- `cargo clippy --all --all-targets --all-features -- -D warnings` ✅
- `cargo test --workspace` ✅ 0 failed
- `cd frontend && npm run lint` ✅
- `cd frontend && npx tsc --noEmit` ✅

Actionable: []
