# Breakdown review — round 1 (Opus + Codex + Gemini)

All three challengers ran against the pushed branch and verified anchors/control-flow against the
real tree. **All three: VERDICT: REVISE.** Strong consensus that the **SC1/SC8 recovery spine, the
schema/view work, and the anchor discipline are sound**; defects are localized and surgically fixable.

## Verified-sound (independently confirmed by ≥2 reviewers)
- Trap 4: `cleanup_orphan_executions` (239), `start_execution_inner` (445), `start_execution` (1064)
  all on the `ContainerService` trait in `services/container.rs`; no cross-crate inversion. 303/304
  correctly co-located + symmetric conflict.
- Dead-code-on-crash premise (ADR-0001): confirmed by the code's own comment at `container.rs:263`.
- 303/304 type contract: `CodingAgentFollowUpRequest { prompt, session_id: String, executor_profile_id }`
  + `ExecutorAction { typ, next_action }` + `ExecutorActionType::CodingAgentFollowUpRequest` all match.
- 104 view columns all exist; migration timestamps sort after `20260131000000_add_webhooks.sql`.
- 401 discriminator (`remote_last_synced_at IS NULL OR EXISTS local attempt`) + all test helpers valid.
- 403↔404 split (keep `MergedProject` TS struct) consistent; 502 `futures` dep + 302 `sysinfo` confirmed.

## Findings actioned (real)
| id | task(s) | finding | source | fix |
|----|---------|---------|--------|-----|
| R1 | 102 / SC2 | **No boot-drain**: `try_consume_queued_message` fires only at `container.rs:738` (live-exit monitor); persisted queue is NOT drained on crash-restart | Gemini F1 | new **task 305** (phase-3) boot-drains non-resumed attempts; SC2→{101,102,305}; soften P1-shippable note |
| R2 | 302 | **`process_inspector` already exists** (`ProcessInspector` trait: `process_exists`/`kill_process`/`get_process_tree`/`find_processes_by_cwd_prefix` + mock) — 302 reinvented it | Gemini F4 | rewrite 302 to build the fence atop `SysinfoProcessInspector`; cwd-prefix match replaces the cmdline heuristic (stronger) |
| R3 | 201 | enum is `#[enum_dispatch]` **bare variants** (`ClaudeCode,`↔`struct ClaudeCode`); `QaMock(QaMockExecutor)` wrong; `default_profiles.json` (needs `QA_MOCK`) missing from `files:`; `mcp_config.rs` spurious (fork's match is in `mod.rs` w/ wildcard) | Gemini F2/F3, Codex F1 | bare `QaMock` variant + `struct QaMock`; add `default_profiles.json`; drop `mcp_config.rs` |
| R4 | 403 | "pure proxy / no local feature" **false**: `/available-nodes` + `/stream-connection-info` have live callers (MCP `list_nodes` `task_server.rs:1390`; frontend `CreateAttemptDialog`/`AttemptHeaderActions`) | Opus 1 | narrow 403: keep those two routes (defer to `vk-swarm-node-ui-localize`); correct the claim |
| R5 | 403 | deleting `resolve_remote_project_id` breaks its 4 colocated tests; gate `cargo check -p server` doesn't compile `#[cfg(test)]` → green-but-broken | Opus 2, Codex 2 | include the tests in scope; gate → `cargo test -p server --no-run` |
| R6 | 405 | spec `:151` requires `last-synced`; agent's 405 used counts only | Codex 5 | add `last_synced_at` (aggregate `MAX(hive_synced_at)`) |
| R7 | 304 | missing the Trap-2 `cargo sqlx prepare`/migrate step in `## Done when` | Gemini F5, Codex 6 | add it (102/104 already have it) |
| R8 | 102 | `QueuedMessage.position: usize` has no sqlx SQLite `Decode` → `query_as!` won't map | Gemini F6 | guidance: untyped `query!` + `position as usize` |
| R9 | plan.md | SC-map cites non-existent task 305 (fallback was folded into 304) | Opus 3, Gemini F7 | `301/305` → `301/304` |
| R10 | 301 | variant-count via `grep -c '('` broken for bare-variant enum | Gemini F8 | count by line |
| R11 | 405 | `depends_on: 403` has no mechanical basis (different files; 403 human-gated) | Gemini F9 | drop the dep |
| R12 | 401/402 | `conflicts_with` declared but edit different files | Gemini F10 | remove conflicts (keep 402 dep 401) |

## Discarded (false positives — verified against `task-gate.sh`)
- Codex 3/4 (105/406 must list `decisions-ledger.md` in `files:`): **false** — `task-gate.sh:104`
  `docs/plans/$TOPIC/*) continue;;` and `:187` `:(exclude)docs/plans/$TOPIC/*` exclude the whole plan
  dir from both the file-allow-list and the forbid scan. Ledger writes need no `files:` entry.

## Low-value notes (acknowledged, not actioned)
- Spec §1 prose cites `local-deployment/container.rs:1617`; real impl L1582 — harmless (tasks use the
  correct `services/container.rs:445`). Spec is frozen; not edited.
- 201 `mcp_config.rs:410` was an explicit *upstream* line ref; resolves correctly. (Superseded by R3.)
