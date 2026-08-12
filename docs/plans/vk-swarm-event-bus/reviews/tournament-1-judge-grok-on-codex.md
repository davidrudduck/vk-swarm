ADVERSARIAL TOURNAMENT — STAGE 2, PEER JUDGING. You are the JUDGE. You did NOT write the
submission under review. Your job is to rule on it, per finding, against the REAL repository.

## The submission you are judging

`docs/plans/vk-swarm-event-bus/reviews/tournament-1-codex.md`

You are NOT judging your own work. Rule without deference: a confidently-worded finding from a
strong model is still wrong if the cited `file:line` does not say what the finding claims.

## What is under review

The submission reviews a BREAKDOWN (plan + task files), NOT code — no implementation exists yet.

- Spec (FROZEN, `spec_sha=8b2c864b5b8679acfd0e278d2728731e3b720ba4`):
  `docs/superpowers/specs/2026-08-07-vk-swarm-event-bus.md`
- ADR: `dev-docs/adr/0017-durable-event-journal-bus.md`
- Plan: `docs/plans/vk-swarm-event-bus/plan.md`
- Phase files: `docs/plans/vk-swarm-event-bus/phase-*.md`
- Task files (12): `docs/plans/vk-swarm-event-bus/phase-*/0*.md`
- Decisions ledger: `docs/plans/vk-swarm-event-bus/decisions-ledger.md`

## Rule on EVERY finding in the submission

For each, produce two independent verdicts:

1. **`issue_real`** — YES/NO. **Open the cited `file:line` yourself.** Is the defect genuine?
   Rule NO if the finding is pedantic, already handled elsewhere in the breakdown, misreads the
   cited code, or merely restates one of the four known collisions listed below.
2. **`fix_ok`** — YES/NO. Is the proposed remediation concrete, correct, and free of NEW defects?
   Rule NO if it is hand-wavy ("consider refactoring"), introduces a worse bug, or contradicts
   the frozen spec. **If you rule `fix_ok: NO` but `issue_real: YES`, you MUST supply the correct
   fix yourself** — that is the most valuable thing you can produce here.

### Already-known, scores 0 (not new findings)

The spec was amended on 2026-08-11 to resolve four spec-vs-`main` contradictions. A finding that
merely restates one of these is NOT new:

- `/api/events` was already taken by a consumer-less record-patch SSE route; task 001 deletes it
  so task 010 can create the bus route on that path.
- The board is not react-query backed and does not poll; UI is out of scope; SC7 was deleted and
  its number deliberately left vacant (ids are append-only, never renumbered).
- Connectivity anchors were repointed from `hive_sync.rs` to `hive_client.rs` / `node_runner.rs`.
- `emit(&mut tx, …)` was replaced by "the DB model function owns a transaction around its own
  discrete write statement".

## Judging rules (non-negotiable)

- You INSPECT and REPORT. You NEVER mutate the repo.
- NEVER revert or discard working-tree state: no `git checkout`, `restore`, `stash`, `reset`, or
  `clean` in ANY form, with or without a path argument.
- Do not apply fixes. Write them as text.
- The spec is FROZEN. If the correct fix would contradict the spec, mark the finding
  `SPEC-COLLISION` rather than proposing a silent divergence.
- Judge only what is in the submission. Do NOT add findings of your own — this is the judging
  round, not a second finding round.

## Output format

One Markdown table row per finding in the submission, in the submission's own order:

| # | severity claimed | issue_real | fix_ok | reasoning (cite the file:line you opened) | corrected fix (required when issue_real=YES and fix_ok=NO) |

Then, on their own lines:

```text
VALIDATED_ISSUES: <count of issue_real=YES>
VALIDATED_FIXES: <count of fix_ok=YES>
```

followed by one line naming the single strongest finding in the submission and why it survives.
