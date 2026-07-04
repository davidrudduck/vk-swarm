---
workstream: remote-services-doctest-revival
doc_type: readme
status: active
title: "Bring 32 rust,ignore'd doctests in remote + services crates back to live"
originated_in: fix/preexisting-gate-failures
originated_commit: 9e20efb4
adrs: []
staging_pointers: []
---

# remote-services-doctest-revival

## Context

The `fix/preexisting-gate-failures` branch remediated all pre-existing gate failures so the
mandatory AGENTS.md gate passes. During that work, 35 doctests in the `remote` (30) and
`services` (5) crates were marked `rust,ignore` at the source level because they referenced
`crate::` paths, types not re-exported at the crate root, or required a live DB/network
environment unavailable in the doctest harness. Three zero-I/O doctests
(`NodeApiKeyError`, `SwarmProjectError`, `HiveSyncConfig::default`) were promoted to live in
the same session; the remaining 35 require real infrastructure or re-export refactoring.

In a subsequent code-review pass, 3 more were promoted: `api_key_router` and
`create_shared_task` (struct literal, no I/O) were made live, and `router` was promoted to
`no_run` (compile-only, since `unimplemented!()` panics at runtime). The remaining 32 require
real infrastructure or re-export refactoring.

This workstream tracks the debt so it is invisible to no future session.

## Inventory

### remote crate (27 ignored doctests)

| File | Line | Symbol | Why ignored | Path to live |
|------|------|--------|--------------|---------------|
| `db/swarm_projects.rs` | 259 | `SwarmProjectRepository::list_with_nodes_count` | Requires live Postgres pool | Integration test harness or `no_run` with mocked pool |
| `db/swarm_projects.rs` | 873 | `SwarmProjectRepository::find_nodes_for_dispatch` | Requires live Postgres pool | Integration test harness or `no_run` with mocked pool |
| `db/tasks.rs` | 239 | `SharedTaskRepository::find_by_id` | Requires live Postgres pool | Integration test harness or `no_run` with mocked pool |
| `db/tasks.rs` | 350 | `SharedTaskRepository::find_by_source_task_id` | Requires live Postgres pool | Integration test harness or `no_run` with mocked pool |
| `db/tasks.rs` | 482 | `SharedTaskRepository::create` | Requires live Postgres pool | Integration test harness or `no_run` with mocked pool |
| `db/tasks.rs` | 585 | `SharedTaskRepository::upsert_from_node` | Requires live Postgres pool | Integration test harness or `no_run` with mocked pool |
| `db/tasks.rs` | 713 | `SharedTaskRepository::bulk_fetch` | Requires live Postgres pool | Integration test harness or `no_run` with mocked pool |
| `db/tasks.rs` | 853 | `SharedTaskRepository::update` | Requires live Postgres pool | Integration test harness or `no_run` with mocked pool |
| `db/tasks.rs` | 961 | `SharedTaskRepository::assign_task` | Requires live Postgres pool | Integration test harness or `no_run` with mocked pool |
| `db/tasks.rs` | 1052 | `SharedTaskRepository::update_status_from_node` | Requires live Postgres pool | Integration test harness or `no_run` with mocked pool |
| `db/tasks.rs` | 1149 | `SharedTaskRepository::delete_task` | Requires live Postgres pool | Integration test harness or `no_run` with mocked pool |
| `nodes/service.rs` | 518 | `NodeServiceImpl::merge_nodes` | Requires live Postgres pool | Integration test harness or `no_run` with mocked pool |
| `nodes/service.rs` | 724 | `NodeServiceImpl::update_node_status` | Requires live Postgres pool | Integration test harness or `no_run` with mocked pool |
| `nodes/service.rs` | 775 | `NodeServiceImpl::list_node_local_projects` | Requires live Postgres pool | Integration test harness or `no_run` with mocked pool |
| `nodes/ws/dispatcher.rs` | 34 | `TaskDispatcher::assign_task` | Requires WS session state | `no_run` or integration test |
| `nodes/ws/dispatcher.rs` | 76 | `TaskDispatcher::assign_task_to_node` | Requires WS session state | `no_run` or integration test |
| `nodes/ws/dispatcher.rs` | 142 | `TaskDispatcher::find_connected_node` | Requires WS session state | `no_run` or integration test |
| `nodes/ws/dispatcher.rs` | 207 | `TaskDispatcher::dispatch_to_available_node` | Requires WS session state | `no_run` or integration test |
| `nodes/ws/session.rs` | 57 | `extract_project_name` | Private function (now covered by 6 unit tests) | Re-export or remove doctest (unit tests suffice) |
| `nodes/ws/session.rs` | 972 | `handle_unlink_project` | Requires WS session state | `no_run` or integration test |
| `nodes/ws/session.rs` | 1102 | `handle_deregister` | Requires WS session state | `no_run` or integration test |
| `nodes/ws/session.rs` | 1220 | `handle_attempt_sync` | Requires WS session state | `no_run` or integration test |
| `routes/error.rs` | 50 | `task_error_response` | Requires axum router context | `no_run` or integration test |
| `routes/nodes.rs` | 243 | `register_node` | Requires axum router + DB pool | `no_run` or integration test |
| `routes/organization_members.rs` | 510 | `ensure_admin_access` | Requires axum extractor + DB pool | `no_run` or integration test |
| `routes/organization_members.rs` | 542 | `ensure_project_access` | Requires axum extractor + DB pool | `no_run` or integration test |
| `validated_where.rs` | 27 | `ValidatedWhere::new` | Compilation issue in doctest context | Fix import paths or `no_run` |

### services crate (5 ignored doctests)

| File | Line | Symbol | Why ignored | Path to live |
|------|------|--------|--------------|---------------|
| `container.rs` | 1185 | `ContainerService::start_attempt` | Requires container runtime | `no_run` or integration test |
| `hive_sync.rs` | 311 | `HiveSyncService::sync_tasks` | Requires live DB + hive endpoint | `no_run` or integration test |
| `remote_client.rs` | 899 | `RemoteClient::list_linked_node_projects` | Requires live HTTP endpoint | `no_run` or integration test |
| `share/processor.rs` | 292 | `ActivityProcessor::process_task_upsert_event` | Requires live DB pool | `no_run` or integration test |
| `share/publisher.rs` | 49 | `SharePublisher::share_task` | Requires live DB + hive endpoint | `no_run` or integration test |

## Acceptance criteria

- [ ] All 32 doctests are either made live, converted to `no_run` (compiles but doesn't run), or removed if the symbol is private and covered by unit tests.
- [ ] `cargo test --doc -p remote` and `cargo test --doc -p services` report 0 ignored.
- [ ] No regression in the mandatory gate (clippy, test, lint, tsc).

## Approach

1. **DB-dependent doctests** (18): Convert to `rust,no_run` so they still compile-check against
   the public API but don't execute. This catches API drift without requiring a live DB.
2. **WS-dependent doctests** (8): Convert to `rust,no_run` — same rationale.
3. **Route-dependent doctests** (6): Convert to `rust,no_run` or move to integration tests.
4. **Private function doctest** (1, `extract_project_name`): Remove the doctest — the 6 unit
   tests added in `fix/preexisting-gate-failures` provide better coverage.
5. **Container/runtime doctests** (5): Convert to `rust,no_run` or integration tests.

## Status

Active — created in the `fix/preexisting-gate-failures` session per the No-Deferred-Remediation
rule. The debt was made visible (source-level `rust,ignore` + this workstream) rather than hidden
(global `doctest = false`).
