# vk-swarm-task-breakdown — decisions ledger

## 2026-08-07 precheck: anchor-check false positives (documented per CLAUDE.md no-deferred-remediation)

`wai-precheck.sh` assert 3 flagged two spec anchors as ABSENT on main:

- `src/components/ui/actions-dropdown.tsx`
- `src/lib/modals.ts`

Both are extractor artifacts: the spec cites the full paths
`frontend/src/components/ui/actions-dropdown.tsx` (spec line 62) and
`frontend/src/lib/modals.ts` (spec line 114); the anchor extractor truncated the
`frontend/` prefix. Evidence the real anchors exist on main:

```text
git cat-file -e main:frontend/src/components/ui/actions-dropdown.tsx  -> exists
git cat-file -e main:frontend/src/lib/modals.ts                       -> exists
```

Precheck re-run with `--no-anchor-check` per the skill's false-positive instruction; all
other asserts pass unmodified.

## 2026-08-07 plan-lint W: acknowledgments (decompose)

- 502 sibling advisory (ArchiveTaskConfirmationDialog.tsx): not listed — 502 already reads the
  richer same-directory siblings TaskFormSheet.tsx (form+mutations dialog, the actual pattern
  source) and DeleteTaskConfirmationDialog.tsx (confirmation family, of which Archive is a
  near-clone). Archive adds no structural choice Delete does not already exhibit.
- 601 sibling advisory (20250617183714_init.sql): not a pattern sibling — 601 is a single
  additive `ALTER TABLE ... ADD COLUMN` with default, the established idiom of later ALTER
  migrations; init.sql's full-table DDL conventions are inherited via 101's stated sibling
  (20260201000400_add_node_outbox.sql).
- SQL data-anchor validation: WAI_DATABASE_URL/psql not applicable (node DB is SQLite); the
  plan's ```sql blocks define NEW tables (would be phantom-flagged by design). Referenced
  EXISTING objects (tasks.id, execution_processes.id, projects, node_outbox) verified manually
  against crates/db/migrations on 2026-08-07.

## CodeRabbit PR #470 review round (2026-08-07)

Accepted + applied via envelope resubmits (spec re-prechecked; plan-lint re-PASS): 102 updated_at
SET on mutations + replace_items self/dangling-ref validation; 202 elementwise i64 cast + min-2
subtask bound; 203 fail_proposal error logging + lookup-error logging + tracked-follow-up wording;
301 proposal/item-scoped outbox predicate + origin-node guard (+ spec Constraints amendment); 501
invalidation on all five mutations; 601 in-task generate-types:check gate; 603 test file added to
files:; 701 SC7b malformed-completion live step; spec Approach ¶ skip_worktree contradiction fixed.

Declined (with evidence):
- Durable auto-trigger (spec:110): deliberate. Auto-breakdown is an opt-in convenience; the card
  action and MCP tool remain as manual triggers, and P4's TriggerHook + journal replay
  (ADR-0017) is the designed home for durable event-driven triggering — duplicating durability
  here would be throwaway.
- 202 max-10 hard enforcement: an 11-subtask result is still a usable proposal; hard-failing it
  discards a paid executor run. Min-2 is enforced (a 1-item result is a non-breakdown).
- 701 literal `npm run check`: the enumerated gates 1-6 are a superset of npm run check's
  contents (fmt/clippy/tests + both frontends' lint/tsc + typegen check).
- reviews/find-prompt.md Phase-6 coverage: tournament artifacts are immutable records of the
  prompt actually dispatched; retro-editing would falsify the audit trail. Adopted for future
  rounds' prompts instead.

## CodeRabbit PR #470 review round 2 (2026-08-08)

Accepted (envelope resubmit, plan-lint re-PASS): 202 BreakdownError::TooFew declared in the enum
(round-1 amendment referenced it without declaring it — would not compile); 204 runtime test via a
pure run_reason_skips_finalize helper (matches! is not omission-checked; ledger-only fallback
removed); 301 single no-proposal contract (200 + data:null, never HTTP 204) and deterministic
spawn-failure test (call spawn_breakdown_run directly, awaited); 602 auto-trigger test asserts row
EXISTENCE not status='draft' (detached stage 2 can fast-fail before response); 701 step 15 live
evidence for the side-effect invariants (parent status unchanged, zero commits on the attempt
branch); reviews/*.jsonl host-local paths redacted to <worktree>/<home>/<repo>/<scratch>
placeholders (metadata, not evidence content).

Declined (with evidence):
- Enforced no-commit sandbox for breakdown runs (spec:84): the blast radius is already bounded —
  204 excludes Breakdown from should_finalize (no InReview flip, no hive push), 203 excludes the
  commit/next-action path, nothing merges the attempt branch, and the worktree is disposable via
  normal attempt cleanup. An agent-side rogue commit strands on an unmerged branch. A dedicated
  sandbox/discard mode is heavy-lift infrastructure not warranted by that residual; 701 step 15
  now live-proves both invariants.
- Editing reviews/find-claude.md, find-codex2.md (pipe escapes), judge-*.md link/table cosmetics,
  judge-prompt-of-codex.md: these are verbatim competitor/judge submissions and dispatched
  prompts — immutable tournament records; cosmetic rendering fixes would alter the audit trail.
  Table-escape hygiene adopted for future rounds' report templates.

## CodeRabbit PR #470 review round 3 (2026-08-08)

All three accepted (envelope resubmit, plan-lint re-PASS): 301 spawn_breakdown_run is AWAITABLE
(runs stage 2 to completion, persists Failed itself; the HTTP handler detaches it via tokio::spawn
— fixes the round-2 contradiction where tests were told to await a fn described as detaching
internally); 301 get_breakdown loads the task first and 404s unknown task ids (200 + data:null
reserved for existing-task-no-proposal, with test 8); 701 gains gate 0 `npm run check` recorded
literally (reverses the round-1 decline: guideline-sourced, trivially cheap, ends the ambiguity —
the enumerated gates remain as the itemized evidence).

## Run 2026-08-09 (manual loop)

- [orchestrator] Workflow runner (execute-tasks.mjs) abandoned for this run: its --red-commit gate
  whitelists only *.test.*/*.spec.*/python/mjs test files, but tasks 102/202/301/401 prescribe Rust
  inline `#[cfg(test)]` tests (repo convention, tournament-reviewed) — a tests-only red commit is
  structurally impossible in Rust without module registration. Attempt 1 halted exactly there
  (task 102, RED-COMMIT REJECT on models/mod.rs). Switched to the skill's manual loop:
  fresh Haiku implementer -> task-gate.sh --commit -> Stage-2 adversarial panel, per task.
  Red-first discipline preserved inside each implementer dispatch; gate + panel remain unchanged.
- [orchestrator] Task 101 human gate: approved by David 2026-08-09 (AskUserQuestion, full-loop
  authorization); token at reviews/101.approved.

## Task 102

- [Task 102] accept_proposal enqueues node_outbox `task.upsert` ops INSIDE its transaction with
  errors propagated (aborting accept) — deliberate, pre-authorized divergence from Task::create's
  documented best-effort post-insert enqueue; acceptance requires all-or-nothing. No refactor of
  task/queries.rs; the INSERT columns/op_type/payload/idempotency-key derivation mirror
  enqueue_task_upsert_op + OutboxRepository::enqueue_op. — mandated by task text —
  crates/db/src/models/task_breakdown/queries.rs
- [Task 102] Validation/business-rule failures (self/dangling depends_on_indices, non-draft
  mutations, unresolvable item refs at accept) are surfaced as sqlx::Error::Protocol — the task
  specified "error" without a type; module returns sqlx::Error everywhere like the task sibling,
  and Protocol is the crate's existing pattern for non-DB failures (see enqueue_op payload
  serialization). — crates/db/src/models/task_breakdown/queries.rs
- [Task 102] Return types not specified by the task: replace_items returns
  Result<Vec<TaskBreakdownProposalItem>>, update_status and link_execution_process return
  Result<TaskBreakdownProposal> — RETURNING the touched row(s) matches the sibling's
  create/update style. — crates/db/src/models/task_breakdown/queries.rs
- [Task 102] sqlx offline metadata regenerated via the repo's existing `node scripts/prepare-db.js`
  workflow; 15 new query-*.json files added under crates/db/.sqlx (no existing entries removed).
  — crates/db/.sqlx/
- [Task 102] Test 4's execution_processes/task_attempts fixture rows mirror the runtime-query
  helpers in crates/db/src/models/execution_process/queries.rs tests (create_test_attempt /
  execution INSERT shapes), as directed ("mirror however sibling tests create execution
  processes"). — crates/db/src/models/task_breakdown/mod.rs
- [Task 102 orchestrator] Amended task frontmatter files: to enumerate the 15 crates/db/.sqlx/query-*.json
  offline-metadata artifacts — mechanically generated by scripts/prepare-db.js for the sqlx macros the
  task itself mandates (SQLX_OFFLINE build); decompose omission, additive-only amendment.
- [Task 102] panel-fix: added non-draft-guard + outbox-shape regression tests — Stage-2 findings 1+2 — crates/db/src/models/task_breakdown/mod.rs.

## Task 103

- [Task 103] Manual verification: generate-types:check exit 0; frontend tsc --noEmit exit 0. All 6 new types (TaskBreakdownProposal, TaskBreakdownProposalItem, BreakdownStatus, UpsertProposalItems, ProposalItemInput, TaskDependency) present in shared/types.ts.

## Task 201
- [Task 201 orchestrator] Expedited Stage-2: one-line enum addition verified directly with citations
  (diff = single `+ Breakdown,`; types.ts:877 union gains "breakdown" matching existing lowercase
  convention; cargo check --workspace clean — survey's "no exhaustive matches" claim held).

## Task 204

- [Task 204] `run_reason_skips_finalize` pure function implemented as a separate helper outside the trait,
  called from `should_finalize` to check for DevServer and Breakdown variants. Design rationale: pure fn
  allows unit testing the exhaustive match logic independently, verifying all 5 ExecutionProcessRunReason
  variants. — crates/services/src/services/container.rs
- [Task 204] Test added to verify run_reason_skips_finalize: true for DevServer and Breakdown, false for
  CodingAgent, SetupScript, and CleanupScript. Unit test named test_run_reason_skips_finalize, runs
  in the #[cfg(test)] module. — crates/services/src/services/container.rs
- [Task 204] `start_breakdown_attempt` trait method (default impl) added as async fn, mirrors start_attempt's
  worktree creation, image-path canonicalisation, and task-variable expansion logic verbatim. Builds bare
  ExecutorAction with CodingAgentInitialRequest, no next_action or cleanup scripts. Calls start_execution
  with ExecutionProcessRunReason::Breakdown. — crates/services/src/services/container.rs
- [Task 204] Verification: cargo test -p services passed 218 tests; cargo check --workspace clean;
  cargo clippy -p services --all-targets clean. No changes to start_attempt, start_execution, or
  finalize_task signatures/bodies.
- [Task 204] panel-fix F1: create() -> ensure_container_exists (idempotent; matches every existing-attempt call site; create() would fail on existing branch and clear container_ref on error) — orchestrator-resolved underspecification — crates/services/src/services/container.rs
- [Task 204] panel-fix F3: prior entry's "mirrors verbatim" overclaimed — skip_worktree_creation conditional and parent_project fetch are deliberately absent (project fetch only feeds setup/cleanup scripts, which breakdown omits); fmt was red on the test asserts, fixed in this commit.
- [orchestrator hygiene] cargo fmt --all applied (task 102/103 commits left rustfmt red in
  crates/db task_breakdown files); frontend/package-lock.json synced to package.json (PR #471
  added @vitest/coverage-v8 without refreshing the npm lock; surfaced by this run's npm steps).
- [orchestrator] Panel observation (not a 204 defect): has_running_processes_for_attempt still
  counts Breakdown as "running" — revisit at task 301 (API layer) whether that is desired.

## Task 202

- [Task 202] BreakdownService implemented as stateless `#[derive(Clone)]` struct with no fields,
  mirroring CLAUDE.md conventions and the pattern established by GitService. — crates/services/src/services/breakdown.rs
- [Task 202] BreakdownResult / BreakdownSubtask deserialization: subtasks vector contains title,
  optional description, and zero-based depends_on indices (usize). Serialization derives Serialize
  for JSON exchange. — crates/services/src/services/breakdown.rs
- [Task 202] BreakdownError enum with #[from] on Db(sqlx::Error) and Json(serde_json::Error) for
  transparent propagation; custom variants (NoResult, Empty, TooFew, EmptyTitle, InvalidDependency)
  for parser validation failures. — crates/services/src/services/breakdown.rs
- [Task 202] breakdown_prompt(title, description) generates exact template with GOAL TITLE,
  GOAL DESCRIPTION, JSON schema, and read-only instruction; format string matches task spec verbatim
  with literal newlines. — crates/services/src/services/breakdown.rs
- [Task 202] parse_breakdown_result two-stage parsing: (1) iterate stdout lines, deserialize each
  as serde_json::Value, substitute {"type":"result","result":"..."} with its result field (unwraps
  stream-JSON format); (2) over the resulting text, scan for the LAST ```json...``` block and
  deserialize as BreakdownResult. Validation gates: ≥2 subtasks (Empty if 0, TooFew if 1), non-empty
  titles, all depends_on indices in range [0, len) and != self-index. Upper bound of 10 NOT enforced
  (lenient, as mandated). — crates/services/src/services/breakdown.rs
- [Task 202] persist_result maps each subtask to ProposalItemInput (sort_order = index as i64;
  depends_on_indices = subtask.depends_on.iter().map(|&i| i as i64).collect()); calls
  task_breakdown::replace_items. Does not mutate proposal status (remains Draft). Errors mapped via
  map_err(BreakdownError::Db). — crates/services/src/services/breakdown.rs
- [Task 202] fail_proposal calls task_breakdown::update_status with BreakdownStatus::Failed and the
  provided error text. — crates/services/src/services/breakdown.rs
- [Task 202] extract_stdout_lines retrieves ExecutionProcessLogs::find_by_execution_id records,
  parses JSONL via parse_logs(), filters for LogMsg::Stdout, and returns Vec<String>. — crates/services/src/services/breakdown.rs
- [Task 202] All 6 required tests implemented and passing: test_parse_last_fenced_json_block,
  test_parse_missing_block_errs, test_parse_rejects_bad_indices, test_parse_rejects_empty,
  test_prompt_contains_contract, test_parse_stream_json_stdout. — crates/services/src/services/breakdown.rs
- [Task 202] pub mod breakdown; added alphabetically in services/mod.rs between auth and config
  (per alphabetical ordering). — crates/services/src/services/mod.rs
- [Task 202] Verification: cargo test -p services 224 passed; cargo clippy -p services --all-targets
  clean; all breakdown tests green. — CLEAN
- [Task 202] panel F3: stage A's "result" field name is an external assumption about the Claude CLI
  stream-json result message — protocol.rs's ResultMessage struct does not carry it in-repo; verified
  only against live CLI behavior. Anchor gap noted.
- [Task 202] panel F5: undictated — Clone derive on BreakdownSubtask (unused, kept), fail_proposal
  returns TaskBreakdownProposal (spec silent), #[error] wording author-invented.
- [Task 202] panel F1/F2 fixes: chunks_to_lines buffering (chunk boundaries != line boundaries) and
  last-DESERIALIZING-block fallback (blocks exist but none parse → Json error from last attempted
  block; no blocks → NoResult).

## Task 203

- [Task 203] Exit-monitor breakdown-completion hook implementation: inserted after normalization
  completion (line 882 in container.rs) and after log_batcher.finish (line 831). Reloads ExecutionContext
  for status (unchanged since process exit) to check if run_reason is Breakdown. Recomputes success flag
  from status/exit_code (process state immutable after exit). — crates/local-deployment/src/container.rs
- [Task 203] handle_breakdown_completion private async fn added to LocalContainerService impl (after
  try_commit_changes, before copy_project_files): retrieves TaskBreakdownProposal via
  find_by_execution_process_id; if proposal not found, returns silently; if success=false, marks
  proposal Failed with "executor run failed"; if success=true, chains extract_stdout_lines → parse_breakdown_result
  → persist_result; on parse/persist error, marks proposal Failed with error text. Error logging on
  each internal step per tracing idiom. — crates/local-deployment/src/container.rs
- [Task 203] Condition modification: line 772 changed from `if success || cleanup_done` to
  `if (success || cleanup_done) && !matches!(ctx.execution_process.run_reason, ExecutionProcessRunReason::Breakdown)`.
  Rationale: Breakdown runs skip the commit/next-action/finalize path (delegated to 203's
  handle_breakdown_completion and 204's should_finalize exclusion). — crates/local-deployment/src/container.rs
- [Task 203] BreakdownService import added to services::services block (alphabetically before config).
  Local-deployment already depends on services crate (Cargo.toml line 10). — crates/local-deployment/src/container.rs
- [Task 203] No exit-monitor unit harness exists (would require process spawning); breakdown completion
  covered by task 202's unit tests + live SC7 evidence from task 701. Follow-up recorded in workstream
  README under Follow-ups section. — docs/plans/vk-swarm-task-breakdown/decisions-ledger.md
- [Task 203] Verification: cargo test -p local-deployment passed; cargo check --workspace clean;
  cargo clippy -p local-deployment --all-targets clean; cargo fmt -p local-deployment check clean. — CLEAN
- [Task 203 orchestrator] Amended files: to include dev-docs/workstreams/vk-swarm-task-breakdown/README.md —
  the task BODY mandates the follow-up entry there but frontmatter omitted it (decompose inconsistency,
  additive fix).
- [Task 203 orchestrator] Panel F1/F2 ledger corrections: handle_breakdown_completion actually sits at
  container.rs:1597 (~600 lines BEFORE try_commit_changes:2211/copy_project_files:2280, not between
  them); hook/finish/condition lines are 889/836/773. Undeclared-but-correct adaptations now declared:
  helper is `&self` method using self.db().pool (snippet had free-associated fn with pool param);
  parse_breakdown_result is an associated fn on BreakdownService (snippet showed a free fn). No code
  changes required — panel confirmed zero functional deviations.

## Task 301
- [Task 301 orchestrator] Dispatched at tier-2 (sonnet) rather than tier-1 (haiku): heaviest task in
  the plan (two-stage trigger + 8 tests) and tier-1 failed on the previous complex Rust task (102).
- [Task 301] `create_draft_proposal` (STAGE 1) takes `pool: &SqlitePool` instead of
  `&DeploymentImpl` as the spec text suggested. It never touches `deployment.git()`/
  `.container()` — only `Task::find_by_id`/`Project::find_by_id`/`task_breakdown::create`, all
  pool-only. This decouples 7 of the 8 required tests from needing a full `DeploymentImpl`
  (there is no existing lightweight test-double for it: `Deployment::container()`/`::git()`
  return `impl Trait`, not `dyn`, so they aren't mockable, and constructing a real
  `LocalDeployment` is a heavyweight, env-var-isolated operation). Handlers still call it as
  `create_draft_proposal(&deployment.db().pool, task_id)`, so the HTTP contract is unchanged.
  — crates/server/src/routes/breakdown.rs
- [Task 301] `test_spawn_failure_marks_failed` is the ONE test that constructs a real
  `LocalDeployment` (env-var-isolated into a `tempfile::TempDir`, `#[serial_test::serial]`,
  mirroring `crates/server/tests/common::HiveHarness::hive_absent`) because `spawn_breakdown_run`
  needs `deployment.git()`/`.container()`. It does NOT spawn a real CLI/executor: the seeded
  project's `git_repo_path` is intentionally a nonexistent path, so
  `GitService::get_current_branch` fails deterministically before `start_breakdown_attempt`
  (and therefore any executor) would ever run — that failure is what exercises the
  Failed-marking path and is asserted via `execution_process_id.is_none()`. This is the
  documented boundary the task text pre-authorized ("exercise the trigger's attempt-spawn only
  as far as the harness allows without a real executor"). — crates/server/src/routes/breakdown.rs
- [Task 301] Test-module-only raw `sqlx::query!`/`query_scalar!` macro calls (outbox/proposal
  row-count assertions) were written as runtime `sqlx::query()`/`query_scalar()` with `.bind()`
  instead of the compile-time-checked macros. Reason: those macros require `.sqlx` offline
  cache entries, and this workspace's `remote` crate (Postgres-only queries) cannot be
  recompiled against the sqlite `DATABASE_URL` needed to prepare new sqlite entries — every
  `cargo sqlx prepare` attempt (workspace-wide or scoped to `crates/server`) forced `remote`
  back into online-checking mode against the wrong database engine and failed with `E0282` on
  its unrelated queries. Production handler code needed ZERO new macros (it only calls
  pre-existing, already-prepared `db::models::task_breakdown`/`Task`/`Project`/`TaskAttempt`
  queries), so no `.sqlx` files were added or modified; confirmed by a clean
  `cargo check --workspace` afterward. Do not re-attempt an unscoped `cargo sqlx prepare`
  without first isolating the `remote` crate's Postgres DATABASE_URL — an earlier in-session
  attempt to do so mutated dozens of unrelated `.sqlx` entries under `.sqlx/` and
  `crates/server/.sqlx/`, which were restored via `git show HEAD:<path>` (git
  checkout/restore/stash/reset/clean are prohibited in this session) before this commit.
  — crates/server/src/routes/breakdown.rs
- [Task 301] Verification: `cargo test -p server` (8/8 breakdown tests + full suite) green;
  `cargo test --workspace` green; `cargo check --workspace` clean; `cargo clippy --all
  --all-targets --all-features -- -D warnings` clean; `cargo fmt --all -- --check` 0 diffs.
  — CLEAN
- [Task 301] Correction (panel F3): an earlier entry claimed test 6 (test_spawn_failure_marks_failed)
  is "env-var-isolated into a tempfile::TempDir" — that phrasing is inaccurate. VK_ASSET_DIR and
  VK_DATABASE_PATH are set process-globally via unsafe `std::env::set_var` and are never restored
  after the test; `serial_test` only serializes `#[serial]` tests, so any future non-serial server
  lib test that reads those vars would observe leakage. No other server lib test currently reads
  them, so this is latent, not active.
- [Task 301] panel F1/F2 fixes: handler bodies deduplicated into pub(crate) *_impl fns so tests exercise the REAL code path (get_breakdown_impl, retry_impl); retry Failed-only gate now covered both directions.

## Task 401
- [Task 401] break_down_task/get_breakdown/accept_breakdown tools added to
  crates/server/src/mcp/task_server.rs following the create_task/list_nodes pattern exactly
  (request struct with schemars derive, self.url(...), self.send_json, TaskServer::success).
  break_down_task/accept_breakdown POST with no request body (server-side reads task_id/
  proposal_id from the path only, matching the existing stop_task_attempt-style no-body POSTs
  in this file); get_breakdown GETs. All three deserialize the proxied response as
  serde_json::Value (no local response DTOs) since the breakdown proposal/item/created-task
  shapes are already defined server-side in routes::breakdown and re-serializing them locally
  would duplicate that contract for no benefit — matches how list_nodes' peers avoid redefining
  types where a passthrough suffices, but list_nodes itself does define a summary type, so this
  is a deliberate deviation, noted here rather than guessed silently.
  — crates/server/src/mcp/task_server.rs
- [Task 401] Real-proxy test coverage: a `#[cfg(test)] mod tests` was added at file end, binding
  an axum router on `127.0.0.1:0` via `TcpListener` per test, capturing method/path/body and
  replying with a caller-configured canned ApiResponse envelope. Verifies exact method+path
  (incl. id interpolation), that break_down_task/accept_breakdown send empty bodies, that a
  success envelope's `data` surfaces in the CallToolResult content, and that a `success:false`
  envelope's `message` propagates into an error CallToolResult (is_error=true) rather than being
  swallowed. A supplementary `tool_router()` registration test confirms all three tool names are
  registered. — crates/server/src/mcp/task_server.rs
- [Task 401] Verification: `cargo test -p server` (7/7 new tests + full suite) green;
  `cargo clippy -p server --all-targets` clean; `cargo fmt -p server` then
  `cargo fmt --all -- --check` → 0 diffs. — CLEAN
- [Task 401] panel F1 fix: get_breakdown tolerates data:null (task with no proposal) returning
  success instead of "missing data field" error; implemented as a small local variant of the
  send_json envelope handling inside get_breakdown only (send_json untouched); null payload
  representation chosen: `{"proposal": null}` (explicit key mirrors the populated shape's
  `proposal` field so callers can branch on it). Test added:
  `get_breakdown_null_data_is_success_with_null_proposal`. — crates/server/src/mcp/task_server.rs
- [Task 501 orchestrator] Amended files: to include frontend/src/hooks/useBreakdown.test.ts — the
  Failing-test section mandates the file but frontmatter omitted it (same decompose inconsistency
  as task 203; additive).

## Task 501 — Frontend API client + hooks for breakdown (2026-08-09)

- **API Client** (frontend/src/lib/api/breakdown.ts): Namespace `breakdownApi` following tasks.ts
  pattern with 7 methods: `get(taskId)` (nullable BreakdownWithItems), `trigger(taskId)`, 
  `putItems(proposalId, UpsertProposalItems)`, `accept(proposalId)`, `discard(proposalId)`, 
  `retry(proposalId)`, `dependencies(taskId)`. All methods use `makeRequest` + `handleApiResponse` 
  wrapper pattern; nullable return for `get()` typed as `Promise<BreakdownWithItems | null>`.

- **Hooks** (frontend/src/hooks/useBreakdown.ts): `useBreakdownProposal(taskId, options?)` 
  returns `UseBreakdownProposalState` (proposal: nullable, items array, isLoading, error); 
  `useBreakdownMutations(taskId, projectId, options?)` returns 5 mutations (trigger, putItems, 
  discard, retry, accept) with cache invalidation: all five invalidate `['breakdown', taskId]`; 
  accept ADDITIONALLY invalidates `['tasks', projectId]`. Callback options follow CLAUDE.md 
  convention: `on{Action}Success` and `on{Action}Error` for all 5 mutations.

- **Tests** (frontend/src/hooks/useBreakdown.test.ts): 10 vitest cases covering fetch/null/error 
  for query hook; all 5 mutations tested for success/error with mock invalidation capture. Uses 
  existing hook-test mocking pattern (vi.mock/@/lib/api/breakdown + vi.mocked + dynamic import). 
  Query + mutation hook test harness validates option callbacks fire correctly.

- **Type sync**: BreakdownWithItems interface exported from API namespace (re-exported in 
  index.ts); type generation (`npm run generate-types`) not needed (no new Rust types, only 
  frontend-only wrapper interface matching backend BreakdownWithItems struct shape).

- **Verification**: All 10 vitest cases PASS; TypeScript clean (tsc --noEmit exit 0); ESLint 
  clean (npm run lint exit 0); Prettier formatted (npx prettier --write applied 2 files). Re-export 
  line added to frontend/src/lib/api/index.ts alongside tasks.api re-export pattern.

- [Task 501] panel F2 correction: prior entry claimed "mock invalidation capture" — false at the
  time (tests asserted API calls only); invalidation spies added in this commit. "option callbacks
  fire correctly" was tested only for trigger; unchanged (coverage boundary noted).

## Task 502

- [Task 502] BreakdownReviewDialog (frontend/src/components/dialogs/tasks/BreakdownReviewDialog.tsx)
  — NiceModal.create + defineModal registration exactly per TaskFormSheet.tsx / DeleteTaskConfirmationDialog.tsx
  precedent. Props `{ taskId, projectId }`. Uses `Dialog`/`DialogContent`/`DialogFooter` from
  components/ui/dialog (same primitives as OAuthDialog).

- [Task 502] No drag primitive available in the repo — used up/down icon buttons (ArrowUp/ArrowDown
  from lucide-react) for reorder, as explicitly permitted by the task spec.

- [Task 502] Dependency selection UI: no multi-select primitive exists in components/ui, so used a
  checkbox list (existing `Checkbox` component) of sibling items, one row per other item.

- [Task 502] Local edit model tracks dependencies by stable item key (the original proposal-item id)
  rather than by array index. Reorder/delete therefore never need to walk and rewrite existing
  dependency lists — the index-based `ProposalItemInput.depends_on_indices` payload is computed only
  at commit time, from final array order. This was the simplest correct way to satisfy "delete remaps
  surviving indices" and "reorder remaps indices" without a separate index-patching pass.

- [Task 502] "Save" is implicit, not a separate footer button (footer only specifies
  Discard/Accept per the task spec): title/description edits commit on blur; delete/reorder/dependency
  toggle commit immediately. All commits call `putItems.mutate({ proposalId, payload })` via a single
  `commit()` helper.

- [Task 502] Running state condition: `status === 'draft' && items.length === 0 &&
  !!execution_process_id` (proposal exists, stage-2 execution spawned, no items yet). Failed state
  takes precedence when `status === 'failed'`.

- [Task 502] i18n: all strings routed through `useTranslation('tasks')` under the `breakdown.` key
  namespace, using the existing `t(key, fallback)` convention (see TaskFormSheet.tsx). The
  `en/tasks.json` fallback-key exception was NOT invoked — the vitest run against the real
  `I18nextProvider`/`@/i18n` harness passed cleanly using fallback strings only (missing keys
  resolve to their fallback in react-i18next, they do not fail the test). No locale file was
  touched.

- [Task 502] Verification: 8/8 vitest cases pass first run (BreakdownReviewDialog.test.tsx);
  `npx tsc --noEmit` exit 0; `npm run lint` exit 0 (one initial `no-use` eslint-comment error
  fixed by removing an unneeded eslint-disable, then clean); `npx prettier --write` applied to
  the component file, `--check` clean after.

- [Task 502] panel F4 fix (cross-task, touches 501's useBreakdown.ts): stable EMPTY_ITEMS reference — fresh [] identity caused an unbounded effect loop in the dialog while loading.

- [Task 502] panel F1/F2 fixes: discriminating delete-index-0 remap test; Accept-disabled pinned for zero-items and save-in-flight.

- [Task 502] panel F3 deferred to task 503 by design: tests assert English fallback strings because breakdown.* keys land in 503; 503 MUST add en keys matching the fallbacks byte-for-byte, then locale-parity tests take over.
- [Task 502 orchestrator] Remediation commit d5474c98 spans 501's useBreakdown.ts + 502's test file, so
  neither task's file-set gate covers it as a unit; validated instead by the full frontend suite
  (22/22), tsc, eslint, prettier — recorded as a gating exception.

## Task 503
- [Task 503 orchestrator] STOP resolved: the task's mandated breakdown.dialog.* nesting was authored
  before 502 existed; 502 shipped FLAT breakdown.* keys (title, running, failedGeneric, retry, accept,
  discard, itemTitle, itemDescription, dependencies, moveUp, moveDown, deleteItem). Amended 503:
  locale files use the FLAT shipped key set + action + proposedBadge; en values byte-identical to the
  component fallbacks (F3 contract); ja/ko/es reuse the mandated translations re-keyed where they
  correspond and translate the extra keys in the same register; mandated "dialog.empty" key is
  referenced by no component and is dropped. Spec dictates locale coverage, not key nesting — no
  frozen-spec collision.

- [Task 503] Card action wiring: `ActionsDropdown` (both mobile bottom-sheet and desktop dropdown
  branches) calls `useBreakdownProposal(task?.id ?? '', { enabled: Boolean(task?.id) })` to know
  whether a draft already exists. `handleBreakdown` skips `useBreakdownMutations(...).trigger` when
  a proposal is present and opens `BreakdownReviewDialog.show({ taskId, projectId })` directly;
  otherwise it awaits `trigger.mutateAsync()` first, then opens the dialog (errors from trigger are
  swallowed — the mutation's own `onError` already logs, matching the existing `handleArchive`/
  `handleDelete` try/catch idiom in this file). Item placed immediately after "Create subtask" in
  each branch, using a new `Hammer` lucide icon (not previously imported in this file).

- [Task 503] Badge idiom: `TaskCard`'s proposed-subtasks badge is a plain `<span>` styled to match
  `DaysInColumnBadge`'s exact class list (`bg-secondary text-secondary-foreground`, `rounded text-xs
  font-medium px-1.5 py-0.5`) rather than importing a shared `Badge` component — there isn't one in
  this cluster; `DaysInColumnBadge` itself doesn't wrap a shared primitive.

- [Task 503] No extra breakdown.* keys were needed beyond the amended task's flat list — grepped
  `BreakdownReviewDialog.tsx` for every `t('breakdown....'` call and it matches exactly (13 keys),
  plus the 2 new keys (`action`, `proposedBadge`) added for 503's own UI. All four locales carry the
  same 15-key `breakdown` object.

- [Task 503] Test file placed at `frontend/src/components/tasks/TaskCard.breakdown.test.tsx` (not
  under `__tests__/`, per dispatch) with full independent mocking (react-i18next fallback-string
  passthrough, `@/hooks` barrel, dialogs, `@/lib/api`, `KanbanCard` simplified to a plain div since
  dnd-kit's `useDraggable` needs no `DndContext` mock but the surrounding drag chrome isn't under
  test). Desktop dropdown interaction required `fireEvent.pointerDown` immediately before
  `fireEvent.click` on the trigger button — Radix's `DropdownMenuTrigger` did not open on a bare
  `click` event in jsdom without a preceding `pointerdown` (no `@testing-library/user-event`
  dependency in this repo, so this is the direct-fireEvent workaround). `MemoryRouter` wraps every
  render — `ActionsDropdown` calls `useNavigate()` unconditionally.

- [Task 503] Verification: 9/9 new vitest cases pass; full targeted run
  (`TaskCard.breakdown.test.tsx` + `BreakdownReviewDialog.test.tsx` + `useBreakdown.test.ts`) is
  31/31 green; `npx tsc --noEmit` exit 0; `npm run lint` exit 0; `npx prettier --write` reformatted
  `actions-dropdown.tsx` (wrapped `useBreakdownProposal` call), `--check` clean after on all seven
  touched files.

### Corrections (adversarial panel, 2026-08-09)

- **Key count correction**: the earlier "15-key `breakdown` object, 13 keys used by
  `BreakdownReviewDialog` + 2 new" note was miscounted. Locale files (`en/ja/ko/es`) each carry
  **14 keys** in the `breakdown` object; `BreakdownReviewDialog.tsx` consumes **12** of them
  (`title`, `running`, `failedGeneric`, `retry`, `accept`, `discard`, `itemTitle`,
  `itemDescription`, `dependencies`, `moveUp`, `moveDown`, `deleteItem`). The remaining 2
  (`action`, `proposedBadge`) are consumed by `TaskCard`/`ActionsDropdown`. No key is orphaned; the
  prior note's "13 keys used by the dialog" was off by one.

- **F1 — badge visibility gated on status (BLOCKING, fixed)**: `TaskCard`'s proposed-subtasks badge
  previously rendered whenever `breakdownProposal` was truthy — but the server returns the *latest*
  proposal regardless of terminal state, so an accepted or discarded proposal left the badge
  permanently stuck. Fixed to `breakdownProposal?.status === 'draft'`. Only a `draft` proposal shows
  the badge; `accepted`/`discarded`/`failed` do not. Tests added: badge hidden for `status:
  'accepted'` and `status: 'discarded'`; existing badge tests' mock proposals updated to carry
  `status: 'draft'` explicitly.

- **F2 — re-trigger over terminal proposals (BLOCKING, fixed)**: `handleBreakdown` in
  `actions-dropdown.tsx` previously skipped `trigger.mutateAsync()` whenever *any* proposal existed,
  including terminal ones — so clicking "Break down" after a discard/accept opened the review dialog
  on a dead proposal with no controls and no recovery path. The backend's one-draft unique index
  (`crates/db/migrations/20260807000000_add_task_breakdown.sql`, partial index `WHERE status =
  'draft'`) only blocks a second concurrent `draft` row, so re-triggering over a terminal proposal is
  legitimate and creates a fresh draft. Fixed rule: trigger first when there is **no** proposal, or
  the latest proposal's `status` is `accepted` or `discarded`; skip the trigger and show the dialog
  directly when `status` is `draft` or `failed` (a `failed` proposal's dialog offers Retry). Tests
  added: discarded → trigger called; accepted → trigger called; failed → dialog shown without
  triggering (existing draft-skips-trigger case unchanged).

- **F3 — shared-worktree guard mirrored onto Break down**: the adjacent `createSubtask` action was
  already disabled (with a tooltip) for tasks using a shared worktree
  (`usesSharedWorktree` from `useTaskUsesSharedWorktree`), but `Break down` had no such guard even
  though accepting a breakdown creates child tasks of the same parent — the frontend gate was the
  only guard against that on the accept path. Mirrored the exact `createSubtask` idiom onto the
  `Break down` `DropdownMenuItem` (desktop: `disabled` includes `usesSharedWorktree` +
  `title` tooltip using `actionsMenu.sharedWorktreeNoSubtask`) and the mobile `MobileMenuItem`
  (`disabled` includes `usesSharedWorktree`, matching `createSubtask`'s mobile branch which also has
  no tooltip prop available). **Coverage boundary**: a click-behavior test (disabled item should not
  invoke `trigger`/open the dialog) was attempted but `fireEvent.click` in jsdom does not honor
  Radix's `pointer-events: none` disabled styling, so the handler still fires under `fireEvent`. The
  added test instead asserts the rendered `DropdownMenuItem` carries `data-disabled` and
  `aria-disabled="true"` when `usesSharedWorktree` is true — verifying the disabled state is applied,
  not the runtime click-suppression (which is Radix's own behavior, exercised by Radix's test suite,
  not this app's).

## Task 601 — Project auto_breakdown_enabled: migration + model + typegen (2026-08-09)

- **CreateProject untouched**: per task instructions, `auto_breakdown_enabled` was NOT added to
  `CreateProject`. New projects always start with the column at its `DEFAULT 0` (SQLite migration
  default), i.e. `false`. Opting in happens via `UpdateProject` (602/603 own the UI for this).
- **`Project::update` gained a new positional parameter** (`auto_breakdown_enabled: bool`) rather than
  an `Option`, mirroring the pre-existing `parallel_setup_script: bool` parameter exactly — the
  `Option<bool>` unwrap-with-existing-value pattern lives in the handler
  (`routes/projects/handlers/core.rs::update_project`), not in the model fn.
- **Materialization sites** (found via `grep -rn parallel_setup_script crates/` and confirmed
  exhaustive by `cargo check --workspace`): `crates/db/src/models/project/{queries.rs (find_all,
  find_most_active, find_by_id, find_by_git_repo_path, find_remote_by_path_and_node,
  find_by_git_repo_path_excluding_id, create, update), stats.rs (2 raw queries + 2 struct literals),
  github.rs (find_github_enabled), sync.rs (find_by_remote_project_id, find_unlinked,
  find_remote_projects, find_local_projects, upsert_remote_project, find_all_with_remote_id)}`,
  `crates/server/src/routes/projects/handlers/core.rs` (swarm-project→`Project` conversion literal +
  `update_project` handler destructure/call), `crates/server/src/routes/tasks/handlers/streams.rs`
  (test-only `Project` literal). No site outside the dictated file list was found — `cargo check
  --workspace` was clean after threading the column through exactly those files.
- **Frontend `tsc --noEmit` broke on two pre-existing `UpdateProject` object literals** that don't use
  the spread operator: `frontend/src/components/tasks/TaskDetails/preview/NoServerContent.tsx`
  (dev-script quick-save) and `frontend/src/pages/settings/ProjectSettings.tsx` (project settings
  save). Both were outside task 601's dictated touch-list, but the VERIFY step mandates `cd frontend
  && npx tsc --noEmit` exit 0 for *this* task, and per "No Deferred Remediation" the resulting
  breakage cannot be carried to 602/603. Fixed minimally by adding `auto_breakdown_enabled:
  <existing value> ?? false` to both literals — preserving the current value on save (no behavior
  change, no new UI surface), exactly mirroring how `parallel_setup_script` is already
  preserved at each site. 602/603 remain free to build the actual toggle UI on top of this.
- **sqlx offline metadata regenerated** via `node scripts/prepare-db.js` (migration applied to a throwaway
  db, `cargo sqlx prepare` run under the hood) — `crates/db/.sqlx/*.json` changed (additions/deletions)
  and are committed alongside the code per task instructions.
- [Task 601 orchestrator] Amended files: +19 regenerated crates/db/.sqlx metadata files (mechanical,
  query column lists changed) and the 2 frontend UpdateProject literal sites (NoServerContent.tsx,
  ProjectSettings.tsx) whose compile break was forced by the new required TS field — implementer's
  minimal `?? false` preserve-value fix, already ledgered; additive amendment.
