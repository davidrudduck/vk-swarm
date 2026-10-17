# Challenger report: proposal.md (M2 `local-execution-lifecycle-correctness` bundle)

Verified from `cosmic-aardvark` worktree, `main` @ `492e8e94`, 2026-09-03. All citations below were
re-run against the live tree in this session; commands/greps are reproducible.

## 1. Overload attack — task count

**Core-table anchors (§2) all check out exactly as claimed:**
- `core.rs:486-511` — confirmed. `start_attempt(...).is_ok()` folded into `has_in_progress_attempt`
  at line 495/504; handler returns `Ok(ApiResponse::success(...))` regardless (lines 502-511).
- `container.rs:714-748` (`try_stop`) — confirmed. `unwrap_or_else` → `debug!` at 727-734, timeout
  → `warn!("...proceeding anyway")` at 741-746.
- `msg_store.rs:55-71` (`push`) and `:125-135` (`history_plus_stream`) — confirmed. Line 128:
  `let (history, rx) = (self.get_history(), self.get_receiver());` — tuple-literal eval order in
  Rust is left-to-right, so `get_history()` really does run before `get_receiver()`; a push landing
  between them is lost from both the snapshot and the live stream. Race is real.
- `crates/local-deployment/src/container.rs:657` vs `:836-858` — confirmed, and I traced the
  **full function body** (exit-monitor path, ~L600-880): `migrate_execution_logs` at 657 genuinely
  executes before `log_batcher.finish` (837) and before `push_finished`/normalization-await
  (848-864).
- `execution_process/mod.rs:56-61` — confirmed, enum is exactly `Running, Completed, Failed,
  Killed`; `grep -rl "generation" crates/db/migrations/` returns nothing — no generation column
  anywhere. Confirmed.

**[INFO] New evidence the proposal didn't cite, which cuts the estimate both ways:**

`crates/local-deployment/src/container.rs` has a **second, near-identical block** implementing the
same flush/finish/migrate sequence in `stop_execution` (~L2020-2145). There, the order is already
**correct**: `log_batcher.finish` (2086) → `push_finished` + awaited normalization (2097-2132) →
`migrate_execution_logs` (2136). This is a materially useful fact the proposal's estimate doesn't
use: the target invariant for AC5/AC7/AC9 already has a **working reference implementation in the
same file** (the manual-cancel path), so this is "make the exit-monitor path match the existing
correct pattern," not green-field design — likely *smaller* than the proposal's "comparable to
`vk-swarm-node-ui-localize`" framing implies for this specific sub-area. [SHOULD-FIX]: cite this in
the PRD so precheck doesn't over-scope AC5/7/9.

**[SHOULD-FIX] Uncounted caller fan-out.** The roadmap's Design section (and the proposal's §2
table) name one call site per bug. The real fan-out is wider and cuts both ways:
- `start_attempt` has **3** callers: `tasks/handlers/core.rs:488` (buggy — silently swallows
  failure), `task_attempts/handlers/core.rs:529` (**already correct** — propagates `Err`, deletes
  the broken attempt, returns `ApiError::Container`), `assignment_handler.rs:143` (**already
  correct** — sends `TaskExecutionStatus::Failed` with the error message). Two of three call sites
  already implement the target behavior. This *shrinks* AC1/AC2 effort (mirror an existing pattern)
  but also raises a question the PRD must answer explicitly: why does the third (buggy) handler
  diverge, and is it a legacy/duplicate endpoint that should be consolidated rather than patched
  in place? Neither the roadmap nor the proposal asks this.
- `stop_execution` has **6** callers (`container.rs:725`, `local-deployment/container.rs:2434`,
  `execution_processes.rs:210`, `task_attempts/handlers/{core.rs:902,git_ops.rs:114}`,
  `assignment_handler.rs:222`) — AC3/AC4 (verified quiescence) must be audited against all of them,
  not just the one path traced above.
- `history_plus_stream` has **12** usages (`msg_store.rs` itself + 5x in `container.rs` + 1x in
  `executors/claude.rs`). Fixing the atomicity bug in `msg_store.rs` is one change; verifying every
  consumer still gets the right ordering guarantee is separate work not reflected in the 13 roadmap
  ACs as a line item.
- `cleanup_orphan_executions` is invoked from `main.rs:131` at **boot**, which means §7.6's
  "reconcile before accepting mutations" requirement is a `main.rs` startup-sequencing change, not
  purely a `container.rs` change. Confirmed via `grep -n "cleanup_orphan_executions" crates/server/src/main.rs` → lines 131, 137.

Net assessment: the proposal's 20-28 estimate is a defensible order of magnitude, but it is not
actually derived from a caller-fan-out audit — it's derived from counting roadmap sub-areas. Given
the fan-out above (some of it favorable, some unfavorable) and the new DB migration + generation
column + 4 new enum states + TS regen, I'd expect the real number to land in the same band **only
if** the PRD explicitly scopes "audit all callers, fix the outlier" rather than "rewrite the
pattern" — worth stating in the PRD seed, not left implicit.

**[INFO] Verified real: the ADR-0001/D8 tension is not cosmetic.** I read the full body of
`cleanup_orphan_executions` (`container.rs:326-553`, not 322-469 as cited — see §4 below). It
unconditionally calls `self.resume_execution(...)` at line 501 whenever a fenced orphaned
coding-agent process has a `session_id` (comment at line 469: "task 301 audit: all 9 executors
do [support session recovery]"), and marks `resume_state = "resumed"` on success (line 515: `tracing::info!(... "Successfully resumed execution")`). This is exactly the automatic-replacement
behavior roadmap §7.6 point 3 forbids pre-M6 ("launch no automatic replacement"). The proposal is
right to flag this as the first `/wai:spec` decision fork; it is real, material, load-bearing scope,
not present in the 13 ACs as a discrete line item, and gating it wrong risks silently reintroducing
two lifecycle models (violates D5/SC7 — the exact failure mode Q4 asks about).

**Verdict on Q4 (is M2 overloaded / where's the split line):** No clean split exists that avoids
two lifecycle-identity models, and the proposal's own reasoning (§2 design-tension note) already
demonstrates why: the D8/ADR-0001 fork, the generation-identity schema (SC7), and the
create/cancel/retry/durability ACs all read and write the *same* `execution_processes` row and the
same state enum. Splitting by roadmap sub-area (7.1 vs 7.3 vs 7.5) would require picking a partial
state-machine mid-PR, which is exactly the two-model risk D5/SC7 forbid. **One workstream is
correct**; the fan-out above argues for a *more thorough precheck*, not a split.

## 2. Wrong-target attack

Recomputed the backlog tallies programmatically rather than trusting the proposal's summary:

```
$ grep -c "^| F-" dev-docs/BACKLOG.md          → 48
$ severity dist                                → 8 high, 20 medium, 20 low
$ status dist                                  → 27 fixed, 15 open, 2 promoted, 2 triaged, 2 wontfix
$ high-severity rows                           → 4 fixed, 2 triaged, 1 promoted, 1 open (F-2026-08-09-01)
$ open rows by severity                        → 1 high, 5 medium, 9 low
```

All match the proposal's §1 table exactly. **No release-gating item outranks M2**: the only open
high-severity finding (F-2026-08-09-01, Claude hooks hang) is explicitly assigned to M5 by the
roadmap itself (`§11.1`), which is *after* the M0-M3 first-release gate (`§9`) — so it is correctly
out of scope by the roadmap's own text, independent of the proposal's preference. Checked every
`draft`/`ready`/`active` workstream README status line (`grep -l "status:" dev-docs/workstreams/*/README.md`) — nothing else claims release-gating priority over M2. `dev-docs/2026-08-05-next-session-handoff.md`
is indeed fully consumed: cross-checked every WS-A..E finding it lists (`F-2026-07-30-01`,
`F-2026-07-31-04/05/06/08`, `F-2026-08-01-01/02`, plus WS-A/B/D items) against current `BACKLOG.md`
— all are `fixed` or promoted into shipped workstreams. No wrong-target finding here; M2 is the
correct next unit. [INFO]

## 3. Bundle-coherence attack

Read the caveat in full (`dev-docs/2026-08-05-next-session-handoff.md:26-30`):
> "One WAI workstream = one spec = one reachability gate, and that gate is per-incident... Three
> unrelated bugs under one spec makes that gate meaningless... The speedup comes from parallel
> worktrees, not a combined spec."

**Lost-log race + `services-normalize-flaky-test` closure — bundling is correct, not a violation.**
The roadmap's own Design section (`2026-08-21-...-design.md:96-97`) names this exact defect
(`msg_store.rs` broadcast-before-history / subscribe-after-snapshot) as something M2 must fix, and
AC6 (§7 Acceptance criteria, item 6: "History/live boundary stress loses and duplicates zero
messages") is a direct regression test for it. This is in-spec text, not a ride-along — folding it
in does not add a second reachability gate. I independently confirmed the tracker contradiction is
real: `services-normalize-flaky-test/README.md` still states hypothesis H1 ("discarded timeout
races the assertion... test-harness race") as the live diagnosis, while
`normalize-fast-execution-lost-logs-flake/README.md` explicitly disproves it 2026-08-14 ("This
disproves H1 as the mechanism for this failure mode... a REAL LOST-LOG RACE, not test slowness",
evidenced by sub-second completion times that rule out the 5s timeout). Closing the stale tracker as
superseded is correct and low-risk (docs-only).

**[BLOCKING — evidence-based, drives VERDICT: REVISE] F-2026-08-19-01/-02 as "M2 phase 0" is a
caveat violation; it should be a separate, sequenced, preceding micro-workstream instead.**

Three independent pieces of evidence:

1. F-2026-08-19-01 is **not a bare backlog finding** — it already has its own dedicated workstream:
   `dev-docs/workstreams/local-deployment-test-orphan-cleanup-safety/README.md`, `status: ready`,
   with its own "Required outcome" checklist (4 items) already written. Absorbing an existing,
   independently-ready workstream into another workstream's "phase 0" is structurally the "combine
   two incidents under one spec" pattern the caveat warns against — the reachability question for
   this finding ("does the fix execute on the orphan-sweep-during-tests path?") is unrelated to
   M2's reachability question ("does the fix execute on the create/cancel/retry/durability path?").
2. **Direct historical precedent in the same document argues for sequencing, not bundling.** The
   2026-08-05 handoff's own WS-E ("test infrastructure — unblocks confidence in everything above")
   is *exactly* the same shape of argument the proposal makes for F-2026-08-19-01/-02 ("M2's tests
   need this to be safe first"). The handoff's own recommended order treats WS-E as **its own item,
   run before** the substantive fixes ("A and B first... Then E... then C and D"), not merged into
   one of the substantive workstreams. The proposal's phase-0 bundling contradicts the pattern its
   own cited source document uses.
3. Cost of sequencing instead of bundling is ~zero: the workstream already exists at `status: ready`
   and can go through `/wai:spec` → `/wai:decompose` → `/wai:execute` in a day or two before M2's
   PRD, in a **parallel worktree** if desired (per ADR-0004, cited in the same caveat).

**Recommendation:** run `local-deployment-test-orphan-cleanup-safety` (with F-2026-08-19-02 folded
in as a same-file, same-concern addendum, since -02 has no workstream of its own yet) as its own
micro-workstream, sequenced immediately before `/wai:prd-new local-execution-lifecycle-correctness`,
not as "M2 phase 0" inside one spec. This directly answers Q2: "should they precede M2 as their own
micro-workstream" — yes.

F-2026-09-02-01 — the proposal's own hedge ("ride-along only if... touches main.rs; otherwise
leave") is appropriately cautious and not a coherence violation either way. [INFO], no change
needed.

## 4. Anchor attack

**[SHOULD-FIX] `container.rs:322-469` citation is imprecise and excludes the load-bearing
evidence.** Verified via `grep -n "fn cleanup_orphan_executions\|resume_execution(\|Successfully
resumed execution"`:
```
326:    async fn cleanup_orphan_executions(&self) -> Result<(), ContainerError> {
501:                    .resume_execution(
515:                            "Successfully resumed execution"
553:    }
```
The function is `326-553`. The cited range `322-469` (322 is the doc-comment, fine) stops **32
lines before** the actual `.resume_execution(...)` call (501) and **46 lines before** the
success-path log line (515) that prove the auto-resume claim the proposal is making. The correct
citation is `322-553` (or, tighter, `468-517` for the specific resume branch). This is the single
most important anchor in the whole proposal (it's what motivates the D8 decision-fork) and it's the
one that's cut short.

**[SHOULD-FIX] Stale sub-citation inherited from BACKLOG.md, not independently re-verified.**
`BACKLOG.md`'s own row for F-2026-08-19-01 cites `crates/local-deployment/src/lib.rs:514-531`.
Verified: `lib.rs:495-535` on `main`@`492e8e94` is deployment-struct field wiring and hive-connection
startup logging — unrelated to orphan-cleanup safety. The actually-relevant code is
`LocalDeployment::for_test()` at `lib.rs:576-589`, which — per the workstream's own README —
**already has the fix** ("Task 022 fixed the new exposure... now invoke one shared process-wide
`DISABLE_WORKTREE_ORPHAN_CLEANUP` guard"; confirmed live at `lib.rs:589`:
`Self::disable_orphan_cleanup_for_tests();`). The genuinely-still-open gap is
`LocalContainerService::new_for_drain_test()` at `container.rs:168-190`, which calls the real
unguarded constructor (comment at line 169-170 admits this, claiming it's "harmless" only because
the worktree base dir happens to be absent in test environments — exactly the assumption the
workstream's README says is unsafe). The proposal's §3 table doesn't repeat line numbers for this
row so it isn't itself "wrong" on the page, but it inherited BACKLOG's imprecise framing without
re-verifying it, which matters for §1's overload analysis: the remaining phase-0 scope is **one
constructor**, not "sweep across `container.rs` and `lib.rs`" as the row implies. This is good news
for effort (smaller than framed) but the proposal should have surfaced it explicitly instead of
citing the file pair generically.

**All other cross-checked anchors hold exactly:**
- `msg_store.rs:55-71`, `:125-135` — exact.
- `local-deployment/container.rs:657`, `:836-837`, `:844-858` — exact (confirmed via full read of
  L600-880).
- `execution_process/mod.rs:56-61` — exact.
- `node_runner.rs:62` (`from_env`) — exact (`grep -n "pub fn from_env"` → line 62).
- `main.rs:273-275` (`axum::serve(...).with_graceful_shutdown(...).await?`) — exact.
- `db/models/execution_process/queries.rs:1369` (control test for
  `sqlite-busy-snapshot-calibration-stability`) — exact; confirmed the README's only unchecked box
  is literally "The branch containing this remediation is merged; mark this workstream `shipped`
  then" (line 54), matching §5 item 3 verbatim.
- `node-task-delete-dangling-shared-id` PR #468 + 2026-08-28 deploy verification — substantively
  correct (confirmed PR #468 merge commit `bd80dfa0` in `reviews/code-review-round-1.md:3`, and a
  full live deploy-verification transcript with SC1-SC3 evidence in
  `decisions-ledger.md:75-118`). Minor: the proposal's "VERDICT: PASS" quote actually belongs to the
  adjacent "Reachability gate" subsection (line 75) rather than literally inside the "Deploy
  verification" subsection header (line 77) — a paragraph-boundary misattribution, not a
  substantive error. [INFO]

**[INFO]** `vk-swarm-refactor/README.md` is still `status: draft` today (`grep "^status:"` →
`draft`), which the proposal's §5 item 6 implicitly covers ("refresh... P3/P4 rows") but doesn't
call out as a status-label question in its own right — worth an explicit line in the tracker-hygiene
PR since "draft" reads oddly for a north-star umbrella with 4 of 8 phases shipped.

## 5. Hidden-dependency attack

- `start_attempt`, `stop_execution`, `cleanup_orphan_executions`, `history_plus_stream` fan-out:
  documented in §1 above — real, but strengthens rather than undermines "single workstream," and
  doesn't point at M3.
- **No evidence of M2 → M3 reverse dependency.** None of the 6 files M2 touches
  (`core.rs`, `container.rs` ×2 crates, `msg_store.rs`, `execution_process/*`, `main.rs`) intersect
  `crates/executors/src/executors/opencode.rs` (797 lines, checked via `wc -l`) or any
  OpenCode-specific type. `grep -n "SpawnContext" crates/services/src/services/container.rs`
  returns nothing — the `SpawnContext` type M2 AC references ("initial runs, retries, and
  follow-ups receive the real `SpawnContext`") lives in the executors crate and is consumed, not
  produced, by M2's changes. Roadmap's stated direction ("M3 is decomposed against M2's settled...
  execution-lifecycle contract", `§5`) is consistent with what's on disk. Confirmed, no hidden
  finding.
- `cleanup_orphan_executions` is called once, at boot, from `main.rs:131-137` — already covered
  in §1's fan-out list; this is the concrete file that turns §7.6 into a `main.rs` startup-ordering
  change, which the roadmap's Design section (file list) omits. Worth naming explicitly in the PRD
  file set.

## Summary of taggable findings

| # | Tag | Finding |
|---|---|---|
| 1 | SHOULD-FIX | Caller fan-out (start_attempt ×3, stop_execution ×6, history_plus_stream ×12, cleanup_orphan_executions@main.rs boot) not enumerated; affects both size and correctness of the estimate. |
| 2 | BLOCKING | F-2026-08-19-01/-02 bundled as "M2 phase 0" violates the one-spec/one-reachability-gate rule; it's an existing independent `status: ready` workstream and the cited precedent (WS-E) argues for sequencing before M2, not merging into it. |
| 3 | SHOULD-FIX | `container.rs:322-469` citation for the ADR-0001 auto-resume claim excludes the actual `.resume_execution()` call (line 501) and success log (line 515); real range is 322-553. |
| 4 | SHOULD-FIX | F-2026-08-19-01's inherited `lib.rs:514-531` sub-citation is stale; the real remaining gap is `container.rs:168-190` (`new_for_drain_test`), narrower scope than implied — should be surfaced, not silently inherited. |
| 5 | INFO | Local-deployment already has a correct reference implementation of the flush-before-migrate invariant in `stop_execution` (~L2020-2145); useful, uncited evidence that shrinks AC5/7/9 effort vs. green-field framing. |
| 6 | INFO | Backlog tallies (48/8-high/15-open) and "nothing outranks M2" both independently reverified and correct; no wrong-target finding. |
| 7 | INFO | Lost-log race + tracker-closure bundling into M2 is correct (in-spec per roadmap Design text and AC6), not a caveat violation. |
| 8 | INFO | No hidden M2→M3 reverse dependency found. |

## Answers to the panel's questions

- Q1: M2 is correct; nothing more release-gating exists (§2, reverified).
- Q2: F-2026-08-19-01/-02 should **precede** M2 as their own micro-workstream, not be folded in as
  "phase 0" (§3, BLOCKING).
- Q3: Agree, keep `claude-executor-hooks-hang` out — roadmap itself assigns it to M5, after the
  first-release gate (§2, reverified).
- Q4: No clean split line exists without risking two lifecycle-identity models; keep M2 as one
  workstream, but tighten precheck to explicitly enumerate the caller fan-out in §1 above.
- Q5: Yes — `container.rs:322-469` (§4, SHOULD-FIX) and the inherited `lib.rs:514-531` sub-citation
  (§4, SHOULD-FIX) are both imprecise/stale on `main`@`492e8e94`.

VERDICT: REVISE
