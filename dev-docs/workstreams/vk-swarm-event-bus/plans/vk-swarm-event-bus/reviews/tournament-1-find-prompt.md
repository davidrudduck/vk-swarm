ADVERSARIAL TOURNAMENT — FIND + REMEDIATE. You are ONE competitor against 2 peers. Find every way
this BREAKDOWN will FAIL an implementer, and for EACH finding propose a concrete, applicable fix.

Scoring: +1 per REAL cited problem, +1 per correct fix — BUT every finding is judged by a PEER (not
you); a finding the peer rules not-real scores 0, and a hand-wavy fix scores 0. Quality beats
quantity: a padded or pedantic nit LOSES points. An honest `FINDINGS: 0` beats a rejected nit.

You are reviewing the BREAKDOWN (the plan and task files), NOT code — no implementation exists yet.

## Files under review (paths relative to --cwd = repo root)

- Spec (FROZEN — see below): `docs/superpowers/specs/2026-08-07-vk-swarm-event-bus.md`
- ADR: `dev-docs/adr/0017-durable-event-journal-bus.md`
- Plan: `docs/plans/vk-swarm-event-bus/plan.md`
- Phase files: `docs/plans/vk-swarm-event-bus/phase-*.md`
- Task files (12): `docs/plans/vk-swarm-event-bus/phase-*/0*.md`
- Decisions ledger (context — records why the spec was amended):
  `docs/plans/vk-swarm-event-bus/decisions-ledger.md`

## Context you need

This spec was AMENDED on 2026-08-11 after decomposition found four contradictions between the
original spec and merged `main`. The amendment is authorised and re-frozen
(`spec_sha=8b2c864b5b8679acfd0e278d2728731e3b720ba4`). Read the ledger first — it explains:

- `/api/events` was already taken by a consumer-less record-patch SSE route; task 001 deletes it so
  task 010 can create the bus route on that path.
- The board is NOT react-query backed and does NOT poll — UI is out of scope; SC7 was deleted and
  its number left vacant on purpose (ids are append-only, never renumbered).
- Connectivity anchors were repointed from `hive_sync.rs` to `hive_client.rs` / `node_runner.rs`.
- `emit(&mut tx, …)` was replaced by "the DB model function owns a transaction around its own
  discrete write statement", because no call site had a transaction and wrapping
  `ContainerService::start_execution` would hold SQLite's single writer lock across git I/O.

A finding that merely restates one of these four is NOT new and scores 0. Attack what the
breakdown does with them.

## Attack axes

For each, cite the task id AND the contradicting repo `file:line`:

1. **Not bite-sized** — two concerns fused into one task.
2. **Wrong / non-existent anchor, symbol, or Before-text.** VERIFY every anchor against the real
   repo. Line numbers in task bodies were taken from `main` at commit `c5cc16d0`.
3. **Ambiguous instruction** — a place the implementer must guess.
4. **`allowed_change` mismatch** — e.g. a task marked `edit` that actually creates a file, or
   `create` where the target already exists (the gate rejects that outright).
5. **Dependency or ordering error** — a task that needs something an earlier task has not built, a
   cycle, or a missing `depends_on`.
6. **Unmarked irreversible** — deletes of code we did not author, dependency removal, or a public
   contract change that is not flagged `irreversible: true`.
7. **Untestable or HOLLOW test** — a "failing test" that would pass without the implementation, or
   that asserts something trivially true.
8. **CONTROL-FLOW GROUNDING** — open the real code. A plausible-but-inverted call path is a
   finding. Symbol existence is NOT control-flow correctness. Specifically worth checking: does
   `Task::update_status` (`crates/db/src/models/task/hierarchy.rs`) really sit on the path
   `ContainerService::start_execution` uses? Are there OTHER writers of task status that tasks
   006-008 would miss?
9. **Fidelity** — an SC or TS clause that no task truly delivers (covered-but-hollow). Walk EVERY
   SC id (SC1, SC2, SC3, SC4, SC5, SC6, SC8 — there is deliberately no SC7) and every TS id
   (TS1..TS6) to the single task that claims it, and check the claim is real. Exactly one task
   claims each id; if a claim is only partially delivered, that is a finding.

Also worth attacking specifically:

- The `subscribe_from` five-step algorithm in task 005 — is the step ORDER actually gap-free? Is the
  `Lagged(n)` branch specified well enough to implement correctly?
- Task 004's compaction predicate — can it delete a row a trigger cursor still needs?
- Task 007's claim that `ExecutionProcess::create` is a discrete statement with the git I/O outside
  the transaction span. Verify it.
- Whether the D8 layering (broadcast Sender in `crates/db`, wrapper in `crates/services`) actually
  compiles as described — does `crates/db` really have what it needs?

## TOURNAMENT RULES (non-negotiable)

- You INSPECT and REPORT. You NEVER mutate the repo.
- NEVER revert or discard working-tree state: no `git checkout`, `restore`, `stash`, `reset`, or
  `clean` in ANY form, with or without a path argument.
- Propose fixes as TEXT inside your findings. Do not apply them yourself.
- The spec is FROZEN. If the correct fix would contradict the spec, say so explicitly and mark the
  finding `SPEC-COLLISION` — do not propose silently diverging a task from the spec.

## Output format

One Markdown table row per finding:

| severity | task | file:line | issue | remediation |

Severity is one of BLOCKING / MAJOR / MINOR. Then, on their own lines:

```text
FINDINGS: <n>
```

followed by a one-line self-assessment of why your findings will survive peer review.
