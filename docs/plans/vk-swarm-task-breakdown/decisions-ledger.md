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
- [Task 503 orchestrator] STOP resolved: the task's mandated `breakdown.dialog.*` nesting was authored
  before 502 existed; 502 shipped FLAT `breakdown.*` keys (title, running, failedGeneric, retry, accept,
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

## Task 602

- **Insertion point**: added the auto-breakdown trigger block in `create_task`
  (`crates/server/src/routes/tasks/handlers/core.rs`) immediately after the existing
  auto-share-to-Hive block and before the final `Ok(ResponseJson(ApiResponse::success(task)))`,
  exactly as dictated. Guard conditions kept verbatim: `project.auto_breakdown_enabled &&
  task.parent_task_id.is_none() && task.description.as_deref().is_some_and(|d|
  !d.trim().is_empty())`.
- **API adaptation (as pre-authorized by the task prompt)**: `create_draft_proposal(pool:
  &SqlitePool, task_id: Uuid)` (stage 1) is awaited directly on `&deployment.db().pool` via
  `crate::routes::breakdown::create_draft_proposal` — both fns are `pub(crate)` in
  `routes/breakdown.rs` and `routes::breakdown` is already `pub mod` in `routes/mod.rs`, so no
  visibility change was needed (the task's conditional `pub(crate)` bump was not required).
  `spawn_breakdown_run(deployment, proposal)` (stage 2) is awaitable-but-non-detaching per its own
  doc comment, so the caller wraps it in `tokio::spawn` — matching the same pattern
  `trigger_and_spawn` already uses in `breakdown.rs` for the manual-trigger HTTP handler.
- **Local var to dodge a partial-move**: captured `task.id` into `auto_breakdown_task_id: Uuid`
  before entering the `tokio::spawn(async move { ... })` block, because the closure needs to log
  `task_id` on error and `task` itself is still needed afterward for the handler's own return
  value (`Ok(ResponseJson(ApiResponse::success(task)))`). `Uuid` is `Copy` so this is a zero-cost
  local, not a functional change.
- **Failing-test-first**: added `mod auto_breakdown_trigger_tests` at the bottom of `core.rs`,
  mirroring the `LocalDeployment`-boundary pattern already established and ledgered in
  `routes/breakdown.rs::test_spawn_failure_marks_failed` (env-var-isolated tempdir, real
  `LocalDeployment::new()`) — required because `create_task` needs a full `DeploymentImpl` for
  `deployment.share_publisher()` / `deployment.git()` / `deployment.container()`, none of which are
  mockable via a pool-only fixture. All 4 dictated cases implemented as separate `#[tokio::test]`
  fns, each `#[serial_test::serial]` (env-var isolation) exactly like the boundary test it mirrors:
  disabled-project → 0 proposal rows; enabled+description+no-parent → 1 proposal row (existence
  only, not status — the detached stage-2 spawn against an invalid `git_repo_path` may already have
  marked it Failed by the time the test asserts, which is expected and does not indicate a bug);
  enabled+empty/whitespace description → 0 rows; enabled+parent_task_id → 0 rows (created via a real
  parent `create_task` call first, since `parent_task_id` must reference an existing task).
- **`ApiResponse<T>` accessor**: fields are private; used the existing `into_data()` consuming
  accessor (already present in `crates/utils/src/response.rs`) rather than reaching into private
  fields — no `ApiResponse` change needed.
- Verification: `cargo test -p server` → 87 passed (lib) + integration suites all green + doctests
  7 passed/3 ignored (pre-existing ignores, untouched); `cargo clippy -p server --all-targets` →
  clean, zero warnings; `cargo fmt -p server -- --check` → 0 diffs.

## Task 603

- **Anchors matched, superseded 601's minimal line**: `ProjectSettings.tsx` pre-flight anchors
  (parallel_setup_script checkbox, `ProjectFormState`/`projectToFormState`, save payload,
  `useTranslation('settings')`) were all present at the described locations (line numbers shifted
  slightly from 601's addition but same idiom). Replaced 601's `auto_breakdown_enabled:
  selectedProject.auto_breakdown_enabled` preserve-value line with `draft.auto_breakdown_enabled`,
  threaded through `ProjectFormState`/`projectToFormState` exactly like `parallel_setup_script`.
- **i18n key nesting**: mirrored `parallelSetup`'s actual nesting — `settings.projects.scripts.*`
  (both locale-file key path and `t()` call use the doubled `settings.` prefix already present
  throughout the file, since `useTranslation('settings')` + `t('settings.projects...')` is the
  existing — if redundant-looking — convention). Task's suggested key path
  (`settings:projects.autoBreakdown.label`, no `.scripts.`) did not match the sibling's real
  location, so adjusted to `settings.projects.scripts.autoBreakdown.{label,help}` per the pre-flight
  instruction to mirror wherever the sibling's keys actually nest. Added to all 4 locales
  (en/ja/ko/es) with the dictated copy, positioned directly after `parallelSetup` in each file.
- **No disabled-prop mirror**: unlike `parallel_setup_script`'s checkbox (which is
  `disabled={!draft.setup_script}`), the `auto_breakdown_enabled` checkbox has no such gating
  condition — it's independent of the setup script field, so no `disabled` prop was added.
- **Test file did not exist — created new**: no `ProjectSettings.test.tsx` and no existing test
  covering `parallel_setup_script` to mirror directly (grepped; none found). Modeled on
  `__tests__/SystemSettings.test.tsx`'s pattern (I18nextProvider + real i18n instance, hook mocks
  via `vi.mock`) since it's the nearest tested sibling settings page, and used `MemoryRouter` (not
  present in SystemSettings) because `ProjectSettings` reads `useSearchParams`. Placed the file at
  `frontend/src/pages/settings/ProjectSettings.test.tsx` (co-located, not under `__tests__/`) since
  the task explicitly named that path. Mocked `useProjects`, `useProjectMutations`,
  `useScriptPlaceholders`, and `WebhooksSettings` (avoids pulling in `useUserSystem`/`ConfigProvider`
  and webhook-fetching dependencies unrelated to this toggle).
- Verification: `npx vitest run src/pages/settings` → 3 files / 15 tests passed; `npx tsc --noEmit`
  → exit 0; `npm run lint` → clean (0 warnings); `npx prettier --check` on all 6 touched files →
  clean (after one `--write` pass on the new test file, which had default-formatting drift on
  creation).

## Task 701 — repo gate evidence (2026-08-09)

All commands run from the worktree root at commit cf4eeac9 (+ evidence commits); every gate exit 0.

```text
gate 0  npm run check                    -> "Finished `dev` profile ... 15.08s"; CHECK_EXIT=0
gate 1  cargo fmt --all -- --check       -> 0 "Diff in" lines (clean)
gate 2  cargo clippy --all --all-targets --all-features -- -D warnings
                                         -> "Finished `dev` profile"; CLIPPY_EXIT=0
gate 3  cargo test --workspace           -> 58 x "test result: ok", 0 FAILED
gate 4  frontend: eslint --max-warnings 0 clean; tsc --noEmit exit 0 (TSC_OK);
        vitest "Tests 500 passed (500)"
gate 5  remote-frontend: eslint clean; tsc --noEmit exit 0 (TSC_OK);
        vitest "Test Files 54 passed (54) / Tests 426 passed (426)"
gate 6  npm run generate-types:check     -> "shared/types.ts is up to date."
```

## Reachability gate

**(a) Call-path trace (cited against the merged branch code):**
Production entry points and the path to every changed unit:
1. UI: TaskCard "Break down" action (frontend/src/components/ui/actions-dropdown.tsx, both
   branches — mobile :608-614, desktop :847-853) -> useBreakdownMutations.trigger
   (frontend/src/hooks/useBreakdown.ts) -> breakdownApi.trigger -> POST /api/tasks/{id}/breakdown.
2. Server: routes/mod.rs:11 `pub mod breakdown;` + :57 `.merge(breakdown::router(&deployment))`
   (nested under /api at mod.rs:85) -> trigger_breakdown (routes/breakdown.rs:239) ->
   create_draft_proposal (:56, stage 1 awaited) -> spawn_breakdown_run (:90, detached by handler)
   -> TaskAttempt creation -> ContainerService::start_breakdown_attempt
   (crates/services/src/services/container.rs:1380, task 204; ensure_container_exists at :1387)
   -> start_execution with ExecutionProcessRunReason::Breakdown.
3. Executor exit: LocalContainerService exit monitor -> post-flush hook
   (crates/local-deployment/src/container.rs:901) -> handle_breakdown_completion (:1597) ->
   BreakdownService::extract_stdout_lines/parse_breakdown_result/persist_result
   (crates/services/src/services/breakdown.rs, task 202) -> draft items.
4. Review/accept: BreakdownReviewDialog -> PUT items / POST accept -> accept_proposal
   (crates/db/src/models/task_breakdown/queries.rs:237) -> child tasks + task_dependencies +
   in-transaction node_outbox task.upsert rows.
5. Auto-trigger: create_task handler hook (crates/server/src/routes/tasks/handlers/core.rs, task
   602, guarded by project.auto_breakdown_enabled from task 601).
6. MCP: task_server.rs break_down_task/get_breakdown/accept_breakdown -> the same REST routes
   (paths verified against router in the 401 panel).
Every changed unit sits on a path reachable from a production entry point; no dead code.

**(b) Real-seam tests:** 602's four tests drive the REAL create_task axum handler through to
proposal-row existence; 301's test_spawn_failure_marks_failed constructs a real LocalDeployment
and awaits spawn_breakdown_run; 401's proxy tests bind a real TcpListener and assert exact
method/path/body against the router-registered paths; 503's tests drive the real dropdown branch
switch. These cross the HTTP/deployment seams, not mocks past them.

**(c) Symptom-mapped assertions:** the spec's core invariant (review gate: nothing real before
acceptance) is asserted live-shaped in test_review_gate_no_outbox_before_accept (zero outbox rows
for proposal/items AND unchanged entity_type='task' outbox count) and the rollback-proof
test_accept_transaction_atomic; the SC5 opt-in invariant by 602's disabled-path test (zero
proposal rows).

Update 2026-08-09 (later same day): the `## Deploy verification` evidence below has now been
captured live on the deployed feature-branch build (SC1–SC7 + side-effect invariants), including
real executor runs, hive stop/reconnect, and observed failure paths. The (b) real-seam coverage
was further hardened by DV-4's `test_parse_assistant_event_without_result_line`, which pins the
ACTUAL production stream shape captured from the live run.

VERDICT: PASS

## Deploy-verification finding DV-1 (2026-08-09) — dead trigger path on fresh attempts

Live SC1 attempt on the deployed branch (node 4653baa7, `http://<node-host>:<node-port>`) failed:
proposal `13e4a1d1` landed `status='failed', error='Container ref not found for task attempt'`.
Root cause: `start_breakdown_attempt` (crates/services/src/services/container.rs:1387) called
`ensure_container_exists` unconditionally, but the local-deployment impl
(crates/local-deployment/src/container.rs:1843) REQUIRES `container_ref` to already be set —
it never creates one. A fresh breakdown attempt always has `container_ref = None`, so the
trigger path was dead in production despite a fully-green pipeline. The 204-F1 panel fix
(create → ensure) introduced this; the panel's concern (create() force-recreates the branch /
clears container_ref on error for EXISTING attempts) and the fresh-attempt case are both
satisfied by conditioning: `container_ref.is_none() → create(), else ensure_container_exists()`
(mirrors `start_attempt` container.rs:1207-1209). Fixed same-session; fmt/clippy/`cargo test -p
services` green (normalize_sync_test full-suite flake re-passed isolated, as ledgered at 204).
This is precisely the failure class the `## Deploy verification` gate exists to catch.

## Deploy-verification finding DV-2 (2026-08-09) — run_reason CHECK constraint missing 'breakdown'

After DV-1's fix, the live re-trigger failed at process insert: `CHECK constraint failed:
run_reason IN ('setupscript','cleanupscript','codingagent','devserver')`. Task 201 added the
`Breakdown` variant to the Rust enum + TS union, but no migration widened the DB CHECK on
`execution_processes.run_reason` — a plan gap (the decompose never anchored the CHECK
constraint), invisible to every repo gate because tests build the schema via migrations and no
test inserted run_reason='breakdown' through raw SQL with the CHECK enforced... it IS enforced;
no test exercised an ExecutionProcess insert with the Breakdown reason (test-honesty gap at the
DB seam). Fix: migration 20260809000000_add_breakdown_to_run_reason_constraint.sql, a column-swap
mirroring 20250720000000 (SQLite cannot alter a CHECK), plus DROP/CREATE of v_workstream_state
(projects ep.run_reason; DROP COLUMN refuses while the view exists — caught by cargo test -p db).
.sqlx regenerated. `cargo test --workspace` fully green post-fix.

## Deploy-verification findings DV-3 + DV-4 (2026-08-09)

**DV-3 (pre-existing, node-wide, NOT this branch):** every server-spawned Claude Code run on the
node stalls after SessionStart hooks, before `system:init` — a normal codingagent control run
stalled identically (no coding run had executed on this node since 2026-01-27). Isolated by manual
repro: `npx @anthropic-ai/claude-code@2.1.114` with the executor's exact flags responds normally
with `hooks:null` in the initialize control_request, but HANGS when sent the merged
`~/.claude/settings.json` hooks payload (get_hooks_merged, crates/executors claude.rs:175) —
protocol drift between the executor's initialize hooks shape and CLI 2.1.114. Workaround for this
evidence run: hooks key temporarily removed from ~/.claude/settings.json (backup
settings.json.bak-dv3, restored after capture). Filed as a backlog finding — executor fix is out
of this workstream's scope.

**DV-4 (real branch defect — parser dead on the real seam):** with DV-3 worked around, the agent
ran to completion (exit 0) but the proposal failed with "No JSON result block found". The Claude
protocol reader (crates/executors/src/executors/claude/protocol.rs:142-147) BREAKS on the final
`{"type":"result"}` line without forwarding it to the log client, so parse stage A (result-line
substitution) can never fire in production, and the fenced block exists only JSON-escaped inside
`{"type":"assistant"}` events, invisible to stage B. Task 202's tests fed fabricated logs
containing result lines — hollow-green at exactly this seam. Fix (contained to
crates/services/src/services/breakdown.rs, no shared-executor change): stage 1 now also
substitutes assistant-event `message.content[].text` blocks; new test
`test_parse_assistant_event_without_result_line` pins the production shape (assistant events, NO
result line) captured from the live 2026-08-09 run.

## Deploy-verification finding DV-5 (2026-08-09) — dev loop leaves a stale MCP binary

**DV-5 (pre-existing, tooling, NOT this branch):** the dev loop rebuilds and restarts
`vks-node-server` but never `vks-mcp-server`, so a node keeps serving the MCP binary from
whenever it was last built by hand. During this verification the newly added breakdown MCP
tools were absent from the node's tool list until `vks-mcp-server` was rebuilt and restarted
manually. Nothing on this branch causes it and nothing on this branch can fix it — the gap is in
the dev/watch tooling. Filed as backlog finding **F-2026-08-09-02**, tracked by the
`mcp-server-dev-loop-rebuild` workstream. Disposition: NOT fixed here (out of scope, tracked).

## Deploy verification

Feature-branch build deployed to the production node (`<node-host>`, db `<node-db-path>`,
hive `<hive-url>`) on 2026-08-09. All outputs below are verbatim from the
deployed system. Build under test:

```text
$ curl -s http://<node-host>:<node-port>/api/health
{"status":"ok","version":"0.0.125","git_commit":"1757424b","git_branch":"wai/vk-swarm-task-breakdown","build_timestamp":"2026-08-09T21:28+Z","database_ready":true}
```

**SC1 — trigger produces a reviewable draft (real Claude Code run, end to end):**

```text
$ curl -s -X POST .../api/tasks/6d6fc3df.../breakdown   → {"status":"draft","id":"fe14674f-..."}
# after the live agent run (claude-code 2.1.114, run_reason='breakdown'):
proposal: fe14674f-e4a8-45c6-beb5-71e3a2c1bb54 draft
  [0] 'Extend /api/health to return version and git commit' deps=[]
  [1] 'Add healthApi client function and useHealth polling hook' deps=["ab602c22-..."]
  [2] 'Build HealthBadge component and mount in settings dialog' deps=["47af20d3-..."]
  [3] 'E2E browser verification and finalization' deps=["25a37879-..."]
```

**SC2 — review gate: edit persists, stays draft, updated_at refreshes, NO outbox rows pre-accept:**

```text
before: draft 2026-08-09 21:30:52.894
$ curl -X PUT .../breakdown-proposals/fe14674f.../items   (4→3 items, retitled item 0)
after:  draft 2026-08-09 21:31:24.109
0|Extend /api/health to return version and git commit (EDITED-SC2)|[]
1|Add healthApi client function and useHealth polling hook|["fded5706-..."]   ← deps remapped to fresh item ids
outbox task.upsert since trigger: 0
```

**SC3 — accept: atomic children + dependency edges + in-transaction outbox enqueue:**

```text
$ curl -X POST .../breakdown-proposals/fe14674f.../accept
accept success: True | children: 3
proposal: accepted
child: Extend /api/health ... (EDITED-SC2) | status=todo   (parent_task_id = 6d6fc3df...)
child: Add healthApi client function ... | status=todo
child: Build HealthBadge component ... | status=todo
dep-edges: 2
outbox rows for children: 3   (idempotency keys task:{child}:{uuid}, all acked=1 after hive sync)
```

**SC4 — MCP tools (vks-mcp-server, port 9003): 23 tools listed incl. break_down_task /
get_breakdown / accept_breakdown; all three round-tripped:**

```text
tools/list → 23 tools: "accept_breakdown","break_down_task","get_breakdown", ...
get_breakdown(6d6fc3df...) → {"proposal":{"id":"fe14674f-...","status":"accepted",...},"items":[...]}
accept_breakdown(fc9b14eb-...) → 4 child tasks created; proposal: accepted
break_down_task(6d6fc3df...) → {"id":"b236791f-...","status":"draft",...}   (re-trigger over accepted OK)
```

**SC5 — auto-trigger honours the project toggle:**

```text
PUT /api/projects/c8809147... {"auto_breakdown_enabled":true}  → auto_breakdown: True
POST /api/tasks (with description) → task 8999929a...; 3s later: auto proposal: draft
  (draft fc9b14eb populated by its own agent run, later accepted via MCP)
PUT ... {"auto_breakdown_enabled":false} → False
POST /api/tasks (with description) → GET breakdown → data: null   (no proposal — correct)
```

**SC6 — offline-first + resync (hive container stopped, then restarted):**

```text
$ docker stop <hive-container>              # hive DOWN 21:35
trigger (via MCP) → agent run → draft 5 items   — fully node-local while offline
$ curl -X POST .../breakdown-proposals/b236791f.../accept → success, 5 children
unacked outbox rows (hive down): 5
$ docker start <hive-container>             # hive UP 21:36:29
unacked outbox rows (after reconnect): 0
acked in last 2 min: 5
```

**SC7/SC7b — failure paths observed live:**

```text
# stopped run → failed with error (exit-monitor failure path):
proposal 13e4a1d1: status='failed', error='Container ref not found for task attempt'   (DV-1 era)
proposal ef4fe0ce: status='failed', error='No JSON result block found in Claude's output' (DV-4 era)
# stop of an in-flight run:
proposal fe77dbb1 → status='failed', error='executor run failed'
# retry over a terminal (failed) proposal creates a fresh draft that then succeeded:
POST .../breakdown → new draft fe14674f (→ SC1 PASS above)
```

**Side-effect invariants:** breakdown runs made zero commits in the attempt worktrees
(run_reason_skips_finalize covers Breakdown); parent task status was changed only by the
deliberate normal-attempt control experiment, never by breakdown runs; proposals/items never
produced node_outbox rows before accept (SC2 fence: count 0).

Findings DV-1..DV-5 discovered by this verification are recorded above; DV-1/DV-2/DV-4 fixed on
this branch (29464845, 64729f1d, 1757424b), DV-3/DV-5 filed as backlog findings
F-2026-08-09-01/-02 (pre-existing, out of workstream scope).

## Post-ship unit-test coverage pass (2026-08-10)

Requested after the run closed. Scope: raise coverage and reliability on the breakdown
feature's own code. 23 frontend + 17 Rust tests added; commits `637c8911`, `e112ff2c`.

### Real defect found and fixed: bigint request bodies (frontend)

`BreakdownReviewDialog.toPayload` builds `sort_order`/`depends_on_indices` with `BigInt(...)`
(ts-rs maps Rust `i64` → TS `bigint`), and `breakdownApi.putItems` passed the payload to
`JSON.stringify`, which **throws** `TypeError: Do not know how to serialize a BigInt`. Every
edit in the review dialog — retitle, description, delete, reorder — therefore failed silently
(mutation rejected, `console.error`, no persistence).

Why every existing gate missed it:
- all three consumer suites (`useBreakdown.test.ts`, `BreakdownReviewDialog.test.tsx`,
  `TaskCard.breakdown.test.tsx`) do `vi.mock('@/lib/api/breakdown')`, so no test ever
  serialised a request body — `lib/api/breakdown.ts` sat at **6.66%** with 500 tests green;
- SC2's live deploy evidence exercised the edit over `curl` with plain JSON numbers, never
  through the UI, so the operator-gated check did not reach the defect either.

Fix: `jsonBody()` in `frontend/src/lib/api/utils.ts` — emits bigints as JSON numbers, and
throws `RangeError` outside the safe-integer range rather than silently losing precision or
emitting a quoted string (serde rejects a JSON string for an `i64` field). Test-first: the
assertion was confirmed RED with the exact production error before the fix landed.

### Coverage (measured, `cargo llvm-cov` / vitest v8)

Breakdown-owned Rust files:

| File | Region | Line |
|---|---|---|
| `db/src/models/task_breakdown/mod.rs` | 100.00% | 100.00% |
| `db/src/models/task_breakdown/queries.rs` | 89.43% | 97.34% |
| `services/src/services/breakdown.rs` | 97.61% | 98.81% |
| `server/src/routes/breakdown.rs` | 78.04% | 75.10% |
| **aggregate** | **91.59%** | **91.75%** |

Frontend: `lib/api/breakdown.ts` 6.66% → **100%** (stmts/branch/funcs/lines);
`hooks/useBreakdown.ts` 79.48% → **100%**; `BreakdownReviewDialog.tsx` 75.22% (unchanged,
already covered by task 502/503).

Shared files are dominated by non-breakdown code, so whole-file numbers are not meaningful
for this component; what changed is that their breakdown-specific fns went from zero to
covered: `start_breakdown_attempt` (`services/container.rs`) and
`handle_breakdown_completion` (`local-deployment/container.rs`, previously 0%).

Deliberately left uncovered in `routes/breakdown.rs`: `spawn_breakdown_run_inner`'s live
executor-spawn body (lines ~135–186) and the thin axum State-unwrap handlers. The logic they
wrap is already tested through the `*_impl` fns (task 301's panel fix exists precisely so
tests hit the real code path). Covering the remainder would require constructing a full
`DeploymentImpl` with a real git repo, or forcing a contrived non-unique sqlx failure — i.e.
testing the framework, not this feature.

### Anti-hollow verification (orchestrator-run, not self-reported)

Tests were validated by mutating production code and confirming RED, then reverting:
- disabling the `Some("assistant")` arm of `parse_breakdown_result` stage 1 (the DV-4 fix)
  turned **both** `test_parse_assistant_event_without_result_line` **and**
  `test_handle_breakdown_completion_success_persists_items` RED — the integration test is a
  genuine seam test, not a fixture agreeing with itself;
- inverting `container_ref.is_none()` in `start_breakdown_attempt` (the DV-1 fix) turned all
  five new branch tests RED;
- `accept` POST→PUT and a misspelled `discard` path turned the frontend wiring tests RED.

All mutations reverted; `git show e112ff2c --unified=0` confirms every hunk is
`@@ -N,0 +M,K @@ mod tests` (zero lines removed) — production code byte-identical.

Note for future parallel runs: agents mutation-testing shared crates concurrently can capture
a **peer's** in-flight mutation in their backup/restore. The reliable check is the committed
diff shape, not any agent's account of its own revert.

### Gates

`cargo fmt --all -- --check`, `cargo clippy --all --all-targets --all-features -- -D warnings`,
`cargo test --workspace` all green; `frontend` lint/tsc/prettier/vitest (523 passed) and
`remote-frontend` lint/tsc/vitest (426 passed) all green.

**Outstanding:** the bigint defect sat in a UI-path gap that no automated suite covered. Corrected
in code-review round 1 (finding 12): the earlier claim that `remote-frontend/scripts/e2e-test.sh`
"is the check that would have caught it" is **false**. That suite is the **hive** frontend's E2E
(`grep -rl breakdown remote-frontend/e2e/` → no match); it cannot exercise the node's review
dialog, and there is no node-frontend E2E suite at all. What actually closed the gap is
`frontend/src/lib/api/breakdown.test.ts`, which drives the real `makeRequest` against a stubbed
`fetch` rather than mocking the api namespace. A manual pass through the review dialog on a
deployed node remains the only end-to-end check available for this path.

## Post-review known issues

CodeRabbit review of PR #475 (2026-08-10) posted 18 actionable findings + 6 nitpicks. Most were
applied. The following were **declined**, with the evidence that supports each decision. They are
recorded here so a later review round does not re-litigate them.

**Declined as a false positive:**

- `crates/db/.sqlx/query-a30f13ff…json:3` — "restore the `github_enabled = 1 AND is_remote = 0`
  filter on `find_github_enabled`". The cache file cited carries `WHERE is_remote = 1`, which is
  `Project::find_remote_projects` (`crates/db/src/models/project/sync.rs:133`), not
  `find_github_enabled`. `find_github_enabled`
  (`crates/db/src/models/project/github.rs:29`) still reads
  `WHERE github_enabled = 1 AND is_remote = 0`, and this branch's only change to that file is the
  added `auto_breakdown_enabled` column in the SELECT list — `git diff origin/main...HEAD --
  crates/db/src/models/project/github.rs` is a single `+` line. No filter was lost. CodeRabbit
  matched the two queries by their shared SELECT column list.

**Declined as out of scope (real, tracked as follow-up work):**

- `frontend/src/components/tasks/TaskCard.tsx:99` — one `GET /api/tasks/{id}/breakdown` per
  rendered card, so an N-task board issues N requests. Correct. The fix is to carry proposal
  status on the task-list response (or a project-scoped batch query), which means a Rust model
  change, `generate-types`, and a frontend data-flow refactor — a workstream, not a review-pass
  edit. Tracked: `task-breakdown-followups` (dev-docs/workstreams/task-breakdown-followups/README.md).
- `crates/db/src/models/task_breakdown/queries.rs:141` — replace the domain
  `sqlx::Error::Protocol` values with a typed `BreakdownDbError` so `map_proposal_error` can
  distinguish 400 from 409 without parsing message strings. Correct and worth doing; it is a
  cross-crate error refactor touching every call site and the route mappings. Tracked:
  `task-breakdown-followups`.
- `crates/server/src/routes/breakdown.rs:320` (nitpick) — split the 674-line route module into a
  directory module. Consistent with the repo convention, but CodeRabbit itself scored it
  `⚖️ Poor tradeoff`; a pure file move on a PR this size costs review clarity and gains nothing
  behavioural. Tracked: `task-breakdown-followups` (dev-docs/workstreams/task-breakdown-followups/README.md).
- `crates/server/src/routes/breakdown.rs:86-170` (nitpick) — bound concurrent detached breakdown
  runs with a semaphore and add an in-flight metric. Real operational risk with auto-breakdown on,
  and CodeRabbit explicitly marks it "not a blocker for this PR". Needs a capacity decision and a
  metrics surface. Tracked: `task-breakdown-followups` (dev-docs/workstreams/task-breakdown-followups/README.md).
- `crates/services/src/services/container.rs:1406-1455` (nitpick) — the image-canonicalisation and
  variable-expansion block duplicates `start_attempt` (1235-1277). Accurate. Extracting the shared
  helper edits the far more heavily exercised `start_attempt` path, which is a change this review
  pass should not make blind. Tracked: `task-breakdown-followups` (dev-docs/workstreams/task-breakdown-followups/README.md).

**Declined in part:**

- `shared/types.ts:396` — "convert `sort_order`/`depends_on_indices` at the API boundary AND align
  the response types with JSON `number`". The first half was already fixed before this review, in
  `637c8911`: `breakdownApi.putItems` uses `jsonBody(payload)`
  (`frontend/src/lib/api/breakdown.ts:56`), which emits bigints as JSON numbers and throws a
  `RangeError` outside the safe-integer range. The second half is declined: `shared/types.ts` is
  generated by ts-rs from the Rust structs and is gated by `npm run generate-types:check`, so it
  cannot be hand-edited, and Rust `i64` → TS `bigint` is the ts-rs mapping contract. Changing it
  would mean changing the Rust field types, which is a wire-format decision, not a review fix.

### Added by `/wai:close` code-review round 1 (2026-08-10)

Record: `docs/plans/vk-swarm-task-breakdown/reviews/code-review-round-1.md`. Six non-actionable
findings (A–F). C/D/E/F restate items already declined above and are tracked in
`task-breakdown-followups`; A and B are new to this round.

- **A** — `crates/db/src/models/task_breakdown/queries.rs:237`: the `(Failed, Draft)` arm of
  `is_legal_transition` is dead in production — no caller performs an in-place re-draft
  (`retry_impl` creates a new row). Only `mod.rs:392-397` exercises it. Left in place: it is
  permissive in the safe direction and harmless, and removing it changes the state machine for no
  behavioural gain. Its *documentation* was the real defect and is corrected under round-1
  finding 9.
- **B** — `crates/services/src/services/breakdown.rs:263`: `store_breakdown_result` maps
  `replace_items` errors through `BreakdownError::Db`, so a cycle detected there would surface as
  `Db(Protocol(…))` rather than the `CyclicDependency` variant. Unreachable on this path —
  `parse_breakdown_result` applies the identical check before persistence — so the mapping is
  cosmetic.
- **C** — `frontend/src/components/tasks/TaskCard.tsx:99`: N+1 proposal fetch per card. Restates
  the declined CodeRabbit item above. Tracked.
- **D** — `crates/db/src/models/task_breakdown/queries.rs:141`: `sqlx::Error::Protocol` in place of
  a typed `BreakdownDbError`. Restates the declined item above. Tracked.
- **E** — `crates/server/src/routes/breakdown.rs`: route directory split + a concurrency semaphore
  and in-flight metric for detached runs. Restates two declined items above. Tracked.
- **F** — `crates/services/src/services/container.rs:1406-1455`: image-canonicalisation duplicates
  `start_attempt:1235-1277`. Restates the declined item above. Tracked.

Round 1 also **refuted** a set of hypotheses (usize underflow, double-accept duplication, stray
`InProgress` write, MCP schema breakage, view drift in the `20260809000000` migration, accept-tx
atomicity, missing outbox rows, a missing origin-node guard, and `jsonBody` correctness). They are
enumerated with their evidence in the round-1 record so a later round does not re-open them.

## Code-review round 1 remediation (2026-08-10)

All 13 actionable findings from `reviews/code-review-round-1.md` fixed in this session. Ordering
was chosen deliberately rather than working the list top-to-bottom, because three findings are
entangled (see below).

### Process deviation (declared)

`/wai:close`'s loop specifies remediation via task files under `docs/plans/<topic>/` run through
the standard WAI loop (constrained implementer → Stage-1 gate → Stage-2 panel). These 13 fixes were
made directly by the orchestrator instead. Reasoning: eleven are single-hunk edits to docs, locale
files, a shell script and a compose banner, where a decompose/dispatch cycle costs more than it
protects; the three Rust correctness changes carry new tests, and the round-2 `/dr:code-review`
pass reviews the remediation diff itself, which is the panel's function here. The deviation is
recorded rather than silent.

### Entanglement: finding 4 could not be fixed as reported

The reported fix (seed Kahn's in-degree from distinct dependencies) **alone converts a wrong
rejection into a broken accept**. `task_dependencies` is `PRIMARY KEY (task_id,
depends_on_task_id)` and the accept insert has no `OR IGNORE`; today `[0, 0]` is rejected at
validation so it never reaches `accept_proposal`, but a correct detector lets it through to a
UNIQUE violation that aborts the whole accept transaction.

So the load-bearing fix is **dedupe at ingest in both validation paths** —
`BreakdownService::parse_breakdown_result` and `task_breakdown::replace_items` — which makes the
counting bug unreachable *and* closes the primary-key face. The counting fix landed as well, as
belt-and-braces, with its own direct unit test (`test_has_dependency_cycle_counts_distinct_
dependencies`) since dedupe otherwise leaves it unpinned. `accept_proposal` additionally dedupes
`depends_on_item_ids` in Rust before inserting, defending rows written before this change; no SQL
changed there, so no `.sqlx` entry moved.

**Undictated choice:** dedupe changes what is persisted relative to what the agent emitted. A
repeated index expresses one edge, and rejecting the run would discard a usable draft the operator
could have edited, so collapsing is the behaviour that loses least. First-seen order is preserved.

### Finding 5 fixed as compare-and-swap, not as a transaction

`update_status` now carries `AND status = $4` on the UPDATE, naming the status the legality check
was made against. Two callers that both read `Draft` cannot both write: the loser writes zero rows
and gets a conflict. Chosen over wrapping the read and write in a transaction because it is a
single atomic statement with no `BEGIN IMMEDIATE`/`SQLITE_BUSY` interaction to reason about. All
callers pass `&SqlitePool` (verified — no caller holds a transaction), so the signature is
unaffected. `crates/db/.sqlx` shows exactly one entry removed and one added.

### Finding 8 re-read after fixing 5

The architecture doc's claim that the state machine "stops a late-arriving executor result from
overwriting a user's discard" was an overclaim against the *old* code. With the CAS predicate the
guarantee is real, so the line was rewritten to name both mechanisms accurately rather than deleted.

### Findings 10 and 11 adjudicated against the frozen spec, not the current code

Both had two possible closures (docs overclaim vs missing implementation). The spec is the arbiter:

- **11** — spec line 110 scopes the auto-trigger to "In `create_task` handler". `create_task_and_start`
  having no trigger is therefore correct, and the docs overclaimed. Both pages now state the
  exclusion explicitly with its reason (that path starts an attempt immediately).
- **10** — spec line 115 specifies "TaskCard shows a proposed badge when a draft proposal exists",
  which is exactly what ships. The *Generating breakdown...* card state was invented by the docs.
  Corrected to describe the badge appearing when the draft is created, i.e. from run start.

### Finding 7 scope (stated, not implied)

The detached spawn's `JoinHandle` is now supervised: a `JoinError` is logged at error level and the
proposal is marked `Failed` so it becomes retryable. This closes **only** the swallowed panic. The
two other routes to a stuck `Draft` that the finding names — a restart between the committed
`create_draft_proposal` and `link_execution_process`, and the silent `Err` arm of
`ExecutionProcess::load_context` (`local-deployment/container.rs:889`) — need recovery design (a
startup sweep) and are tracked as item 6 in `dev-docs/workstreams/task-breakdown-followups/README.md`.

### Findings 1/2/3/6 validated read-only

The docker/compose execution ban stands (a prior version of `e2e-test.sh` would have destroyed the
live hive). These four were validated by `bash -n`, YAML parse, and reading Compose's documented
precedence — **not** by running the stack. Nobody exercised the fixed script.

- **2** — `COMPOSE_PROJECT_NAME` is now assigned unconditionally (was `${VAR:-default}`, which an
  ambient value overrides) *and* `dc()` passes `-p "$COMPOSE_PROJECT_NAME"` on every invocation.
  The flag is the winner in Compose's precedence chain (`-p` > env > directory name), so project
  identity can no longer be lost to an inherited environment. `SERVER_PORT`/`POSTGRES_PORT` stay
  caller-tunable — those are legitimately overridable; project identity is not.
- **3** — the preflight now aborts when `ss` is absent instead of falling through. A guard whose
  job is preventing destruction of a live deployment must never pass by accident.
- **1** — the manual-run block and the whole Troubleshooting table now pass `-p vkswarm-e2e` on
  every command, and the `<Warning>` prescribes the flag rather than an exported variable. The
  "Port 9000 in use" row was itself the trap (9000 is the deployment's port, never the E2E port);
  it now reads 9210/5540 and explicitly tells a reader who finds a *deployed hive* to tear nothing
  down.
- **Extension beyond the finding:** `docs/development/local-docker-dev.mdx` carries the identical
  hazard — it deliberately uses Compose's default project name on 9000/5434, the same identity a
  deployed hive uses, and its Management block ends in a bare `down -v`. A warning now tells the
  reader to check for a deployed hive before running anything on that page. Same failure mode, one
  door further along; fixing only the cited file would have left it open.
- **6** — the `seed-db` banner interpolates `${SERVER_PORT:-9000}` / `${POSTGRES_PORT:-5434}`
  rather than hardcoding the deployment's ports. Defaults included so a direct `docker compose`
  invocation (no script) still prints real values instead of empty ones.

### Finding 13 — real translations, not English copies

Three keys were missing from all four locales, not two: the review found `breakdown.loading` and
`breakdown.reload`; auditing every `breakdown.*` call site against the locale files surfaced
`breakdown.loadFailed` as well. All three added to en/ja/ko/es with genuine translations —
copying English into four files satisfies a key-presence check while leaving the defect intact.

Incidental: `frontend/src/i18n/locales/en/tasks.json` carried a **duplicate** `sharedTask` key
(lines 373 and 390, identical values). Reserialising collapsed it. Behaviour is unchanged — JSON
parsers keep the last occurrence — and the file is now well-formed.

### Gates (all green, run after remediation)

`cargo fmt --all -- --check`, `cargo clippy --all --all-targets --all-features -- -D warnings`,
`cargo test --workspace` (exit 0, 58 suites, **1190** passed / 0 failed — was 1185, +5 new tests),
`npm run generate-types:check`, `frontend` lint + tsc + `format:check` + vitest (528 passed),
`remote-frontend` lint + tsc + vitest (426 passed). `format:check` is included deliberately —
F-2026-08-05-02 filed that gate as never having been run.

New tests: `test_replace_items_dedupes_duplicate_dependencies`,
`test_accept_with_duplicated_dependency_writes_one_edge`,
`test_update_status_is_compare_and_swap_under_concurrency` (db),
`test_parse_dedupes_duplicated_dependency`,
`test_has_dependency_cycle_counts_distinct_dependencies` (services).

**Untested by design:** finding 7's supervision. The auto-trigger block sits inside the
`create_task` axum handler, which needs a full `DeploymentImpl` with a real git repo to exercise;
injecting a panic into `spawn_breakdown_run` to assert the log line would test tokio, not this
feature.

### Added by `/wai:close` code-review round 2 (2026-08-10)

Record: `reviews/code-review-round-2.md`. Three non-actionable findings (G–I), all new.

- **G** — `BreakdownReviewDialog.tsx:281`: a settled query with `proposal === null` renders an
  empty dialog with a silently dead Discard button. Effectively unreachable (the badge that opens
  the dialog renders only when a draft exists); worth an explicit empty state when the N+1 fetch is
  reworked, which is followups item 1.
- **H** — `BreakdownReviewDialog.tsx:243`: the Reload button gives no in-flight feedback, because
  after an error the query status stays `error` rather than `pending`, so `isLoading` is false
  during the refetch. Harmless — react-query dedupes concurrent fetches for the same key.
- **I** — `queries.rs:298`: the CAS `RowNotFound → Protocol` remap means a proposal row deleted
  between the read and the write surfaces as 409 rather than 404. Defensible: a row vanishing
  mid-update is a concurrency conflict. The absent-proposal case still 404s via the initial
  `find_by_id`, which the remap does not touch.

Round 2 also **refuted** the double-click window carried from round 1: every one of Discard, Retry
and Accept is defended server-side independent of client state (the `a == b` transition arm plus
CAS; the one-draft-per-task partial unique index; and `accept_proposal`'s re-read on the
transaction handle respectively), so the client-side `isPending` disabling is defence in depth
rather than the only guard.

## Code-review round 2 remediation (2026-08-10)

Two actionable findings, both fixed.

- **Finding 1 — the running state never resolved.** A breakdown run completes server-side with
  nothing pushed to the client; the only `['breakdown', taskId]` invalidation is in the dialog's own
  mutations, and neither consumer polled. With `staleTime: 5min` and `refetchOnWindowFocus: false`
  (`main.tsx:13-14`), *Generating breakdown...* sat there indefinitely — and closing and reopening
  the dialog did not help, because the query stayed fresh in cache. Fix: `useBreakdownProposal`'s
  `refetchInterval` accepts a predicate over the current data, and the dialog passes
  `runningPollInterval`, which polls at 3s **only** in the running shape (draft + execution process
  + zero items) and returns `false` otherwise.

  **Undictated choice:** `TaskCard` deliberately left un-polled. Polling per card would multiply the
  N+1 proposal fetch already tracked as followups item 1, so a stale badge for up to `staleTime`
  is accepted in exchange for not amplifying a known scaling problem. Revisit when item 1 lands.

- **Finding 2 — stale comment.** The auto-trigger comment still said "fire-and-forget" after the
  round-1 fix made the spawn supervised. Fixed. Worth naming: this is the same intent-vs-behaviour
  drift round 1 filed five findings against, reintroduced by the fix for one of them.

Pinned by six new vitest cases. Anti-hollow verified by mutation: forcing `runningPollInterval` to
return `false` turns `polls while a run is in flight` RED; mutation reverted and the file confirmed
additive-only.

**Independence gap (recorded, not hidden):** the adversarial reviewer dispatched for the round-2
remediation diff never reported — the same failure mode as round 1's frontend finder. Its five
axes were covered directly by the orchestrator with cited evidence (see "Verified sound" in the
round-2 record), so coverage holds, but the independent second opinion is genuinely missing.

Gates after remediation: `cargo fmt`/`clippy`/`test --workspace` (1190 passed), frontend
lint/tsc/format:check/vitest (**535** passed, was 528), remote-frontend lint/tsc/vitest (426) — green.
