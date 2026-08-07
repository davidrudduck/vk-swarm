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
