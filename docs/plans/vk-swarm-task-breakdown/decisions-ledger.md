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
