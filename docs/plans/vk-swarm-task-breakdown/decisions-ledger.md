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
