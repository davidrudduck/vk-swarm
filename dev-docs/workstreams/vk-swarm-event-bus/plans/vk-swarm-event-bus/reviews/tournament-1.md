# Tournament 1 — adversarial breakdown review (vk-swarm-event-bus)

- **Date:** 2026-08-11
- **Method:** `cross-model` — real external CLI competitors, non-self peer validation
- **Target:** the 12-task breakdown at commit `ae3e807e` (plan + 5 phase files + 12 task files)
- **Spec:** FROZEN at `spec_sha=8b2c864b5b8679acfd0e278d2728731e3b720ba4`
- **Safety:** every seat dispatched `--safety read-only`; no competitor mutated the tree

## Roster

| Seat | Tool | Model | Effort | Status | Findings |
|---|---|---|---|---|---|
| A | codex | gpt-5.x | high | ok | 23 |
| B | opencode | `x-ai/grok-4.5` | high | ok | 13 |
| C | opencode | `moonshotai/kimi-k3` | high | **timeout** (1500s, exit 124) | — |

Seat C never produced a report. The round therefore closed with **two** competitors, which still
satisfies the non-self peer-validation threshold (each submission was judged by the other), but it
is one seat short of the three the skill asks for. Recorded rather than papered over: a third
independent family might have found defects these two share a blind spot on.

### Seat failures and the environment fault behind them

The first dispatch of seats B and C both died instantly with 227-char stubs (below the runner's
400-char review floor, correctly caught by `run_opencode_panel.py` rather than silently accepted).

Root cause was **not** the models: `~/.config/opencode/opencode.json` carried
`"shell": {"env": {...}}`, but OpenCode's schema expects `shell` to be a string. The whole config
failed to parse, so every OpenCode invocation exited before doing any work — OpenCode was
non-functional for all uses, not just panels. Fixed by the operator, verified with a smoke run.

A second, independent fault surfaced on re-dispatch: the DR panel runner's CLI had changed
(`--variant` removed in favour of `--effort`; `--provider` split out from `--model`). The original
argv would have failed even against a valid config.

**Process note:** the stub-floor guard is what made both faults visible. A panel that "returns
something" is not a panel that ran.

## Judge round (stage 2, non-self)

| Judge | Judged | Verdict |
|---|---|---|
| grok-4.5 | codex (23 findings) | 23/23 `issue_real`, 23/23 `fix_ok` |
| codex | grok (13 findings) | 8/13 `issue_real`, 8/13 `fix_ok` — 5 rejected |

Grok's verdict rows each cite an independently-opened `file:line`; it re-ran the SQLite rollback
probe itself rather than taking codex's word for it. Codex judged considerably more harshly,
rejecting five of grok's thirteen. Recorded honestly: a 23/23 pass rate is weaker adjudication than
an 8/13, and the scoreboard below should be read with that asymmetry in mind rather than as a
clean comparison of finder quality.

### Scoreboard

| Competitor | Findings | Validated issues | Validated fixes | Score |
|---|---|---|---|---|
| codex | 23 | 23 | 23 | **46** |
| grok-4.5 | 13 | 8 | 8 | **16** |

### What the peer gate killed (the reason this stage exists)

Codex rejected five grok findings with cited reasoning, including two that would have caused real
rework:

- grok #2 (005 constructors) — ruled *not new*: task 005's STOP trigger already requires enumerating
  every constructor beyond `bootstrap`/`new`. Codex noted the remediation was nonetheless incomplete
  for a different reason grok missed — a direct `DBService { pool, metrics }` struct literal at
  `crates/local-deployment/src/container.rs:168-176`, outside all three constructors, which would
  break compilation when the struct gains a field.
- grok #11 (008 not bite-sized) — ruled a subjective organisation preference the breakdown had
  already considered, not a correctness defect.
- grok #13 (compaction predicate self-contradiction) — ruled a misreading: deletion is strictly
  *below* the sentinel, so the claimed broad contradiction does not follow.

### The decisive argument against a global sender

Judging grok's own proposed fix for the sender collision, codex supplied a defeater neither I nor
grok had: production builds the **bootstrap** service first and only then the live service via
`new_with_after_connect` (`crates/local-deployment/src/lib.rs:155-166`), so a single-assignment
`OnceLock` would capture and permanently retain the *bootstrap* sender. That is independent of, and
stronger than, the test-isolation objection.

## Orchestrator's own verification

Per the skill, findings are applied only after the orchestrator independently verifies them. The
following were confirmed by direct repo inspection, quoted below — this is stronger evidence than a
peer verdict and does not depend on the judge round:

| Claim | Verification |
|---|---|
| Third `DBService` constructor is the production path | `crates/db/src/lib.rs` has three `Ok(DBService { pool, metrics })` sites (`:337`, `:446`, `:461`); `crates/local-deployment/src/lib.rs:165` calls `new_with_after_connect`. `bootstrap()` is used only to build the EventService hook (`:159`). |
| `Task::delete` cannot own a transaction | `crates/db/src/models/task/queries.rs:369-376` is generic over `E: Executor`; `crates/server/src/routes/tasks/handlers/core.rs:655-670` opens the txn, nullifies children, calls `Task::delete(&mut *tx, …)`, then commits. |
| `HiveClient` has no DB handle | `crates/services/src/services/hive_client.rs:767-772` — struct is `{config, state, event_tx, command_tx}`; `new()` takes only `HiveClientConfig`. |
| Disconnect anchor is inverted | `hive_client.rs:808-824` — the `Ok(())` clean-close arm emits **nothing**; the `Err(e)` arm emits on every failed attempt, and `connected = false` is set after both arms with no `was_connected` gate. |
| SQLite reuses a rolled-back `seq` | Direct probe: committed `seq=1`; allocation `2` inside a rolled-back txn; next committed row got `2`; `sqlite_sequence` read back `('j', 2)`. |
| Task 004 `allowed_change` is wrong | Task 004 declares `allowed_change: create` with only the two new files; task 009 correctly declares `mixed` and lists `crates/db/src/models/mod.rs`. Self-inconsistent within the same breakdown. |

## Cross-family convergence

Codex and grok are different model families running from independent contexts. They converged on
the same defect at the same anchor in **seven** places: 004 module registration, 005 third
constructor, 006 delete-transaction collision, 007 `mark_orphaned_as_failed`, 008 `HiveClient`
reachability, 009/011 missing startup wiring, and 005 `subscribe_from`/`Lagged` under-specification.
Independent convergence is the strongest signal this round produced.

## Round CLOSED — 2026-08-11

Per the termination rule, the round closes when every peer-validated finding is remediated AND a
focused re-check passes — not when a round finds zero. Both conditions hold:

- **All 23 peer-validated codex findings and all 8 peer-validated grok findings are remediated.**
  The five grok findings codex rejected were dropped, not re-litigated.
- **Focused re-check green:** `PLAN-LINT PASS`, `decompose-guard: OK`, and coverage verified
  exactly-one per SC id (SC1-SC6, SC8) and per TS id (TS1-TS6). Every `W:` sibling advisory is
  acknowledged in the decisions ledger.

Two findings were resolved better than by remediation — they were dissolved. Codex's #4 (task 005
wires the wrong `DBService` constructors) became moot because `DBService` now gains no field at all;
and grok's delete finding needed no separate fix because an executor-generic `append` composes with
the caller-owned transaction. A deliberate deviation from codex's corrected delete fix is recorded in
the ledger with its reasoning.

No further round was launched. Launching one purely to confirm silence is the infinite loop the
termination rule exists to prevent.

## Outcome

The breakdown is **substantially defective** and does not proceed to `/wai:execute` as committed.
Remediation is tracked in the section below and applied by amending the submit envelope and
resubmitting via `wai-submit-plan.sh` — never by hand-editing promoted files under
`docs/plans/vk-swarm-event-bus/`.

### Spec-owner decisions taken (2026-08-11)

Both escalations below were decided by the spec owner. The spec and ADR-0017 are amended and
re-frozen via a second `/wai:precheck` run; tasks were NOT patched to diverge (ADR-0001).

1. **Emission/publish → journal-tailer.** DB model functions only *append* to the journal, using the
   executor they already hold; a per-`DBService` background tailer reads `seq > last` and publishes
   to the broadcast channel. Chosen over `DBService`-owned emitting wrappers (real churn at every
   emission call site — exactly what D2 was written to avoid) and over a process-global `OnceLock`
   (which captures the bootstrap sender and cross-publishes between tests).

   This dissolves **both** escalations at once: because `append` takes `E: Executor`, it composes
   with a caller-owned `&mut *tx` exactly as `Task::delete` already does, so the delete-transaction
   collision disappears rather than needing its own resolution. It also makes "journal-first,
   broadcast-second" structural instead of a convention an implementer can forget, and eliminates
   the broadcast-before-commit bug class outright. Accepted cost: publish latency becomes
   poll-bounded (~50-100ms) rather than immediate, and one background task per node. Latency is
   immaterial for the named consumers — P6 triggers, P7 MCP observability, and the SSE endpoint are
   none of them interactive.

2. **Retention → hard cap with a rebootstrap signal.** `VK_EVENT_MAX_ROWS`; above it, compaction
   deletes below the cursor floor regardless and marks the affected hook as needing rebootstrap.
   Chosen over cursor-staleness (bound is only eventual; a merely-slow hook loses events silently)
   and over weakening the Constraint (a dead hook then fills the disk on a long-lived node — the
   exact failure the Constraint exists to prevent). This is the only option that guarantees the
   Constraint absolutely, and it makes the failure explicit and observable rather than silent.

### Originally escalated to the spec owner (ADR-0001 — not patched into tasks)

1. **Emission/publish architecture.** The spec's Design requires, simultaneously: the publish
   happens inside the DB model function after its own commit; the `broadcast::Sender` lives in the
   db layer held alongside the pool; and caller signatures stay `&SqlitePool`. These are jointly
   unsatisfiable — a `&SqlitePool` has no route to its owning `DBService`. A process-global sender
   would resolve it only by breaking test isolation (every test builds its own pool in-process).
2. **Retention vs a stuck consumer.** Constraints require the journal cannot grow unbounded, while
   Design requires compaction never delete at or above the minimum persisted trigger cursor. A hook
   whose cursor stops advancing pins every later row forever. Task 009's cursor bug (cursor advances
   only after a *fire*, so non-matching events never advance it) makes this reachable immediately.

### Applied as task-level fixes (no spec change)

Recorded in full in the decisions ledger; the notable ones:

- Task 004: `allowed_change: create` → `mixed`, add `crates/db/src/models/mod.rs`, make
  `pub mod event_journal;` a first-class step rather than a conditional aside.
- Task 004: invert the rollback assertion — SQLite **does** reuse the value. The spec already
  defers this to a test ("asserted by a test rather than assumed"), so no amendment is needed.
- Task 004: `append` cannot return `Result<i64, sqlx::Error>` while using `serde_json::to_string?`
  — `serde_json::Error` has no `From` into `sqlx::Error`. Pin an explicit error type.
- Task 003: the `event_type_matches_serde_tag` test covers only `TaskCreated`; table-drive it across
  all nine variants, or derive the tag from serialization so there is no parallel hand-written match.
- Task 003/007: SC2 requires executor identity on start **and** terminal events; the prescribed
  schema carries `executor` only on `AttemptStarted`.
- Task 007: add `mark_orphaned_as_failed` (`execution_process/queries.rs:110-130`) — a real terminal
  failure path that emits nothing as written.
- Task 008: split connectivity emission from the TS3 cross-site suite (two concerns, one task);
  re-anchor disconnect on a `was_connected` transition gate covering both the `Ok` and `Err` arms.
- Task 009: advance the cursor on **every** consumed event, not only on a fire.
- Task 010: declare a concrete test file under `crates/server/tests/`; the file-set gate would
  otherwise reject the required TS5 commit. Correct the SSE precedent — `stream_raw_stream` does not
  exist (the symbol is `stream_raw_logs`, and `routes/logs.rs` is REST/WebSocket, not SSE).
- Task 012: it cannot both accept feature-branch deployment evidence and require post-merge `main`
  evidence in a commit that is part of the pre-merge PR.
- New wiring task(s): nothing in the current file sets can register the trigger hook or spawn the
  compaction loop on a live node, so SC6 and the bounded-journal behaviour never run in production.

### Coverage-gate consequence

Splitting task 008 and re-homing SC1's observability clause both change the coverage map.
`submit_coverage_check` requires **exactly one** claiming task per SC id and per TS id, so the
split must land TS3 on exactly one of 008/008b, and SC1 must end up claimed by exactly one task —
not two.
