# Decisions Ledger

## Submission
Plan accepted from submit envelope.

## Historical — Task 001 Attempt 2 STOP: TS4 Red Proof Not Reproducible

The attempt-1 red claim was false. This historical attempt-2 STOP was superseded by the
2026-08-29 re-sequence below, which moved red proof ownership to task 002. The release binary was
built with `cargo build --release -p server --bin vks-node-server` and the script was run on
2026-08-29 against `target/release/vks-node-server`.

### Verification Evidence

The historical script revision performed the required two-boot authenticated flow. It seeded frames
with API writes, then ran concurrent repeated external `sqlite3` reads in plain, `mode=ro`, and
`mode=rw` forms while polling `/proc/$PID/fd` every 0.5s for 60 iterations. Leg B was retried on a
fresh scratch database with the full sequence. Attempt 5 replaces the iteration bound with a
30-second wall-clock deadline.

```
========== SUMMARY ==========
Total PASS: 32
Total FAIL: 12

Leg results:
  LEG A: PASS
  LEG B: FAIL

[22:49:55] ERROR: One or more assertions failed
```

Observed detector evidence lines:

```
[22:48:51] Trip detector timeout after 30185ms
[22:49:23] Trip detector timeout after 30179ms
[22:49:55] Trip detector timeout after 30173ms
```

No `/proc/<pid>/fd` line containing `db.sqlite-wal (deleted)` was observed. Leg A therefore did
not prove the required current-code loss (`marker-A-post` persisted, count=1), and the required
red proof was not established. Per the authorized STOP trigger, execution stops for human
re-triage rather than recording leg A as a pass signal.

### Task 001 Decisions

- [Task 001] Keep `LEG_RESULTS` associative — each selected leg needs an independent result while the global assertion counters remain shared — `scripts/live/wal-unlink-durability-repro.sh`
- [Task 001] Use a script-scope `NODE_PIDS` array and have `run_node` set `RUN_NODE_PID` — command substitution would lose PID ownership and disable cleanup — `scripts/live/wal-unlink-durability-repro.sh`
- [Task 001] Run repeated concurrent CLI reads in plain, `mode=ro`, and `mode=rw` forms — the permitted empirical levers were needed to test the unlink window without external tracing dependencies — `scripts/live/wal-unlink-durability-repro.sh`
- [Task 001] Use `timings-B.txt` for baseline leg B — the contract forbids contaminating leg A's timings file — `scripts/live/wal-unlink-durability-repro.sh`
- [Task 001] Use integer nanosecond-to-millisecond arithmetic — avoids an undeclared `bc` dependency — `scripts/live/wal-unlink-durability-repro.sh`
- [Task 001] Keep the sibling's `check_status` name but pass command arguments after the label — assertions involving per-leg paths and PIDs require arguments while preserving centralized PASS/FAIL accounting — `scripts/live/wal-unlink-durability-repro.sh`

## 2026-08-29 — Task 040 amendment: drop CARGO_TARGET_DIR (operator-directed)
Operator decision on resuming execution in the `origin-clever-pangolin` worktree: the
main-baseline build no longer uses an isolated `CARGO_TARGET_DIR`; it builds in the
baseline worktree's own default `target/` dir. Rationale: the isolated dir existed only
to keep the build off quota-tight `/tmp`; the baseline worktree lives at
`/data/.cache/wal-main-baseline` on the `/data` volume (3.1T free at decision time), so
the default in-worktree target dir already satisfies the constraint with one less moving
part. Task 040 step 2 and its Allowed-moves path reference updated in place. Also fixed a
stale `/tmp/wal-main-baseline` reference in Allowed moves (the Change section had already
moved the worktree to `/data/.cache`). Historical tournament records left untouched.

## 2026-08-29 — Phase-1 re-sequence: red proof moves 001 → 002 (operator-approved)
- [Task 001/002 orchestrator] Empirical result from two execution attempts: on a FRESH
  scratch node the node's pool (min_connections=2, crates/db/src/lib.rs) holds the
  wal-index, so the external sqlite3 close does NOT unlink the WAL — the unlink needs the
  lock-state window the spec (line 16) said must be pinned empirically. 001's original
  red_proof (red on current code) was therefore unachievable BEFORE the window is known,
  and two implementer attempts confirmed it (attempt 1 recorded a false leg-A PASS —
  rejected by the adversarial panel; attempt 2 STOPped honestly with evidence).
  Operator approved re-sequencing: 001 now gates harness MECHANICS only (MODE=baseline
  green both legs + full-mode legs execute all assertions); 002 (which already owned the
  VERDICT-1 mechanism hunt + script-edit rights) gains the red_proof, SC4/TS4 coverage,
  and must encode the window in the SHARED CLI-read step so leg A's red arm also fires.
  — why: spec premise intact (incident was on a long-running node; window is real per the
  2026-08-28 live repro) but the plan's 001-before-mechanism sequencing bet failed.
  — files: phase-1/001-*.md, phase-1/002-*.md, plan.md, phase-1-evidence-and-repro.md

## Task 001 — Attempt 7 Harness Verification

The 2026-08-29 amendment moves the red proof and external-read trigger-window STOP to task 002.
Task 001 gates harness mechanics only. The release binary at
`target/release/vks-node-server` was used without rebuilding.

### Verification Evidence

`MODE=baseline bash scripts/live/wal-unlink-durability-repro.sh` exited `0`:

```
========== SUMMARY ==========
Total PASS: 24
Total FAIL: 0

Leg results:
  LEG A: PASS PASS=12 FAIL=0 TOTAL=12
  LEG B: PASS PASS=12 FAIL=0 TOTAL=12
```

`LEGS=AB MODE=full bash scripts/live/wal-unlink-durability-repro.sh` exited `1`; both selected
legs completed:

```
========== SUMMARY ==========
Total PASS: 23
Total FAIL: 6

Leg results:
  LEG A: INCONCLUSIVE PASS=15 FAIL=0 TOTAL=15
  LEG B: FAIL PASS=8 FAIL=6 TOTAL=14
```

Superseded by Attempt 8: the CLI-read assertion adds one PASS per full-mode leg (A 15→16, B 8→9); counts below retained for review history only.

Leg A's detector timed out without a recorded assertion failure, so its full-mode result is
`INCONCLUSIVE`, not a pass; it forces the required nonzero exit. The full-mode run also proves the
retry accounting path: attempt 1 timed out, stopped cleanly, and completed as provisional without
running dependent post-trip assertions. Its PASS counter alone was discarded before the
fresh-database retry. A retry is permitted only when attempt one recorded no new assertion failure;
any other attempt-one failure on the detector-timeout path remains in `FAIL_COUNT`, prevents the retry, and — because the provisional stop already ended that attempt's node — leaves the attempt sentinel unwritten, so Leg B is reported ABORTED, not completed-fail.
Attempt 2 timed out, recorded its detector and dependent-contract failures, and produced Leg B's
final `8/6/14` counts.

`LEGS=A MODE=full bash scripts/live/wal-unlink-durability-repro.sh` exited `1`:

```
========== SUMMARY ==========
Total PASS: 15
Total FAIL: 0

Leg results:
  LEG A: INCONCLUSIVE PASS=15 FAIL=0 TOTAL=15
```

Superseded by Attempt 8: the CLI-read assertion adds one PASS per full-mode leg (A 15→16, B 8→9); counts below retained for review history only.

For the abort proof, delayed listener PID `2783702` waited until preflight had created
`timings.txt`, then claimed port 9012. `SCRATCH_ROOT=/tmp/task001-abort-proof-attempt7 LEGS=A
MODE=baseline bash scripts/live/wal-unlink-durability-repro.sh` exited `1`; its boot-1 health check
timed out and produced an `ABORTED` Leg A. The listener was then killed and reaped by exact PID.

```
========== SUMMARY ==========
Total PASS: 0
Total FAIL: 1

Leg results:
  LEG A: ABORTED PASS=0 FAIL=1 TOTAL=1
```

The harness reported node `2792392` already dead during bounded cleanup, and port `9012` was free
immediately after the listener reap (`port_9012=free`), confirming no leftover harness node. Port
`9012` was free after the baseline, full, A-only, and abort runs.

### Task 001 Decisions

- [Task 001] Write Leg B's leg-level completion sentinel only after the final completed attempt has assigned `LEG_RESULTS[B]` and its per-leg counts — a stale attempt-one sentinel cannot conceal a retry abort — `scripts/live/wal-unlink-durability-repro.sh`
- [Task 001] Give each Leg B attempt its own completion sentinel; discard only provisional attempt-one PASS accounting before a retry — completed non-timeout failures remain visible in the leg result — `scripts/live/wal-unlink-durability-repro.sh`
- [Task 001] Treat only attempt one's detector timeout as provisional: stop that attempt before dependent assertions, then retry only when no other new failure was recorded; a retry timeout is an assertion failure — `scripts/live/wal-unlink-durability-repro.sh`
- [Task 001] Stop on API 401/403 by recording an auth-drift STOP reason, aborting the active leg, and reporting unstarted selected legs as `SKIPPED_DUE_TO_STOP` — seeded-cookie contract drift is distinct from an assertion failure — `scripts/live/wal-unlink-durability-repro.sh`
- [Task 001] Bound health polling with a 30-second wall-clock deadline derived from `SECONDS`; the loop breaks at the deadline and passes its positive remaining interval to curl — `scripts/live/wal-unlink-durability-repro.sh`
- [Task 001] Bound the trip detector and CLI stimulator by the same 30-second wall-clock window; launch the stimulator in a new session and kill its exact process group — detector coverage lasts through the full stimulus interval without orphaned sqlite children — `scripts/live/wal-unlink-durability-repro.sh`
- [Task 001] Reject Leg B refusal credit for `000` and every status outside `100..599`; curl transport failures therefore fail explicitly as no HTTP response — a dead node cannot earn refusal credit — `scripts/live/wal-unlink-durability-repro.sh`
- [Task 001] Truncate both `timings.txt` and `timings-B.txt` at run start — reused scratch roots cannot retain stale Leg B samples — `scripts/live/wal-unlink-durability-repro.sh`
- [Task 001] Add explicit `|| return 1` checks to every required seed-session command and required project initialization command — callers in `if` contexts cannot mask a failed session insert or repository setup under bash errexit suppression — `scripts/live/wal-unlink-durability-repro.sh`
- [Task 001] Poll a graceful node stop for 15 seconds, then send exact-PID SIGKILL, reap it, and fail the assertion — a SIGTERM-ignoring node cannot hang the harness — `scripts/live/wal-unlink-durability-repro.sh`
- [Task 001] Bound every API curl with a two-second connect timeout and ten-second overall timeout — transport failures are explicitly classified instead of hanging — `scripts/live/wal-unlink-durability-repro.sh`
- [Task 001] Use `${LEGS-AB}` and `${MODE-full}` defaults — only unset selectors default, while explicitly empty selectors fail preflight with exit 2 — `scripts/live/wal-unlink-durability-repro.sh`
- [Task 001] Allocate primary leg directories only after the validated selector chooses that leg; use `$BACKEND_PORT` for the port check; remove unused state — selected-leg lifecycle state is the only allocated state — `scripts/live/wal-unlink-durability-repro.sh`
- [Task 001] Keep repositories under each leg directory rather than a separate `/tmp` mktemp directory; use a header-only seeded cookie rather than a curl cookie jar; retry the full Leg B sequence on a fresh scratch database; and use bounded API curls — these standing divergences preserve per-leg cleanup and make transport behavior observable — `scripts/live/wal-unlink-durability-repro.sh`
- [Task 001] Treat seeded-session 401/403 as STOP without incrementing assertion counters, test STOP after every API write (including each timing write), and stop the active node before returning — auth-contract drift is distinct from an assertion failure and cannot produce later writes — `scripts/live/wal-unlink-durability-repro.sh`
- [Task 001] Remove each node PID from `NODE_PIDS` immediately after it is reaped; the EXIT trap retains only live harness-owned processes and cannot signal a recycled historical PID — `scripts/live/wal-unlink-durability-repro.sh`
- [Task 001] Preflight `git` and `setsid` as exit-2 dependencies — project setup needs `git init`, and the bounded CLI stimulator needs a dedicated session — `scripts/live/wal-unlink-durability-repro.sh`
- [Task 001] Use fixed-string path matching only on `wal_unlinked_externally` log lines — neither log-field ordering nor regex metacharacters in scratch paths affect the assertion — `scripts/live/wal-unlink-durability-repro.sh`
- [Task 001] Print successful API task IDs to stderr and require a non-empty extracted ID — assertion stdout stays PASS/FAIL-only while malformed success envelopes cannot pass — `scripts/live/wal-unlink-durability-repro.sh`
- [Task 001] Guard the CLI process-group kill with `kill -0` — a reaped stimulator PID cannot target a recycled process group — `scripts/live/wal-unlink-durability-repro.sh`
- [Task 001] Re-test and reap a node after a failed SIGTERM send; if it is still live, exact-PID SIGKILL and bounded reap it — a send race cannot leave a dead PID in `NODE_PIDS` or a live node for the next leg — `scripts/live/wal-unlink-durability-repro.sh`
- [Task 001] Classify a full-mode Leg A detector timeout with no failed assertion as `INCONCLUSIVE` — the pre-002 timeout is uninformative and must force exit 1 rather than produce a false green — `scripts/live/wal-unlink-durability-repro.sh`
- [Task 001] Write Leg B's provisional attempt sentinel only when the attempt recorded no assertion failure at all, including its stop; otherwise finish dependent checks without a sentinel — an incomplete detection/refusal contract is `ABORTED`, not completed-fail — `scripts/live/wal-unlink-durability-repro.sh`
- [Task 001] Treat missing project IDs after failed project creation as the same mid-setup abort in both legs, and do not add a redundant ID assertion when creation already failed — one root failure has one assertion and one classification — `scripts/live/wal-unlink-durability-repro.sh`

### Historical Attempt 4 Note

Attempt 4's `31/12` full-mode totals and shared Leg B completion behavior belonged to the prior
script revision. Attempt 5 replaces those mechanics and measurements; this note is retained only
to preserve the review history.

## Task 001 - Attempt 8 Harness Verification

`MODE=baseline bash scripts/live/wal-unlink-durability-repro.sh` exited `0`:

```
Total PASS: 24
Total FAIL: 0
  LEG A: PASS PASS=12 FAIL=0 TOTAL=12
  LEG B: PASS PASS=12 FAIL=0 TOTAL=12
```

`LEGS=AB MODE=full bash scripts/live/wal-unlink-durability-repro.sh` exited `1` with no
`ABORTED` legs on this host:

```
Total PASS: 25
Total FAIL: 6
  LEG A: INCONCLUSIVE PASS=16 FAIL=0 TOTAL=16
  LEG B: FAIL PASS=9 FAIL=6 TOTAL=15
```

`LEGS=A MODE=full bash scripts/live/wal-unlink-durability-repro.sh` exited `1`:

```
Total PASS: 16
Total FAIL: 0
  LEG A: INCONCLUSIVE PASS=16 FAIL=0 TOTAL=16
```

For the abort proof, a throwaway listener with exact PID `2286502` waited until preflight had
created `timings.txt`, then claimed port 9012. `SCRATCH_ROOT=/tmp/task001-attempt8-abort LEGS=A
MODE=baseline bash scripts/live/wal-unlink-durability-repro.sh` exited `1`:

```
Total PASS: 0
Total FAIL: 1
  LEG A: ABORTED PASS=0 FAIL=1 TOTAL=1
```

The listener was killed and reaped by exact PID after the harness finished. Port `9012` was free
after every run, including the abort proof (`port_9012=free`).

Attempt 9 verification: `bash -n` passed; the ABORTED/UNKNOWN summary check precedes INCONCLUSIVE;
one `MODE=baseline` run exited `0` with `24/0`. Port `9012` was free afterwards.

### Task 001 Decisions

- [Task 001] Persist successful CLI-read evidence in a per-leg sentinel and assert it after the
  detector returns - a stimulus that executed no successful sqlite3 read is a failed assertion,
  not an inconclusive timeout - `scripts/live/wal-unlink-durability-repro.sh`
- [Task 001] Check Leg B's CLI-read assertion before provisional retry classification - a
  no-stimulus attempt cannot receive timeout credit or write the provisional completion sentinel,
  so the leg is reported `ABORTED` - `scripts/live/wal-unlink-durability-repro.sh`
- [Task 001] Credit refusal only for a real non-2xx response or a 2xx JSON envelope whose
  `.success` is false; `000`, malformed status, and 2xx responses without false success fail -
  `scripts/live/wal-unlink-durability-repro.sh`
- [Task 001] Report `INCONCLUSIVE` as a completed-but-uninformative pre-002 timeout, while only
  `ABORTED` and `UNKNOWN` use the did-not-complete reason -
  `scripts/live/wal-unlink-durability-repro.sh`

## T1 mechanism evidence (2026-08-30)

Host/tool versions: `sqlite3 3.53.4 2026-07-24`, `Python 3.14.7`, Linux
`6.8.0-138-generic`.

### VERDICT 1 - STOP

The required current-code CLI-read trigger window was not reproducible on the real `:9012`
node. Baseline remained previously verified at `24/0`; the unmodified full harness remained
non-zero with detector timeouts (`LEGS=B MODE=full`: attempt 1 and retry both timed out, final
`PASS=9 FAIL=6`). `VK_SQLITE_MAX_CONNECTIONS=1` produced the same result. A five-second
quiescent delay and a CLI `PRAGMA wal_checkpoint(TRUNCATE); SELECT` also produced no deleted
WAL. The prescribed overlap experiment (CLI held open while an API write ran) blocked the API
write and the node exited at the bounded attempt, so it is not valid incident evidence.

The permitted syscall trace did show unlink ownership for other phases: the node process
(`2436441`) unlinked `db.sqlite-shm`/`db.sqlite-wal` during graceful shutdown, and SQLite client
processes (`2446304`, `2449714`, `2458678`) unlinked the same files during shutdown/offline
inspection. In a separate live-node exploratory run, an external SQLite client write caused
the node's `/proc/<pid>/fd` to retain `db.sqlite-wal (deleted)` and `db.sqlite-shm (deleted)`;
that was not the required read-only CLI flow and therefore does not establish VERDICT 1.

Decision: STOP with halt code `human_gate_required` under the task's no-reproducible-trigger
condition. VERDICT 2 and VERDICT 3 were not run because their prerequisite trip was not
established. No script maintenance delta was retained.

- [Task 002] Stop at VERDICT 1 rather than infer a read-only trigger from the exploratory
  external-write result - the required real-node CLI-read unlink window was not observed, and
  the task requires human re-triage when it cannot be reproduced -
  `docs/plans/wal-unlink-durability/decisions-ledger.md`

## 2026-08-30 — Re-plan: trip stimulus = external write session; 002 rescoped to design-decision experiments (operator-approved)
- [Task 002 orchestrator] VERDICT 1's evidence hunt (ledger `## T1 mechanism evidence
  (2026-08-30)`, commit 797665bbf) established: the read-only CLI vector is NOT reproducible
  on the current binary under any probed condition; an external WRITE session reliably
  produces the deleted-WAL/deleted-SHM state. Operator approved the 10,000-foot re-plan:
  the workstream's outcome is "no silent data loss under any external vector", and the
  guard/monitor/salvage/refusal design is vector-agnostic — so the vector hunt stops here.
  Spec amended narrowly (incident vector: external session, write-confirmed) and re-frozen;
  002 rescoped to the three design-decision experiments (unlinker mechanism under the
  write-vector, guard mode MapOnly-vs-HoldRead, A6 salvage) + harness write-session stimulus;
  phases 2-4 unchanged. 001 remains passed (its mechanics contract was vector-free).
  — files: docs/superpowers/specs/2026-08-28-wal-unlink-durability.md,
  docs/plans/wal-unlink-durability/phase-1/002-*.md, docs/plans/wal-unlink-durability/plan.md
- [Task 002 orchestrator] Spec re-frozen after the 2026-08-30 vector amendment: new
  .precheck.passed token (operator-approved amendment; substantive precheck checks —
  anchors vs main, SQL literals, SC coverage — unaffected; plan-lint re-passed;
  freshness gate verified green). ADR-0001 deliberate re-freeze path.

### Task 002 current-code write-stimulus re-confirmation — STOP

Host/tool versions: `sqlite3 3.53.4 2026-07-24`, `Python 3.14.7`, Linux
`6.8.0-138-generic`; binary `target/release/vks-node-server`, health version
`0.0.125`, git commit `c31474131`.

The exact mandated changing-value write shape was re-confirmed without re-running the
closed read vector: after a successful boot-2 health check on a fresh `:9012` scratch
database, `sqlite3 <db> "PRAGMA user_version=$RANDOM;"` ran and exited 0. Before the
write, `/proc/3571550/fd` showed live `db.sqlite-wal` and `db.sqlite-shm` descriptors
(fds 14, 15, 17, 19, 21, 25-27); after the write it showed the same non-deleted
paths. `lslocks` showed the node holding POSIX READ locks on `db.sqlite` and
`db.sqlite-shm` both before and after. `strace -f -e trace=unlinkat,renameat2` on
the sqlite3 write session emitted only `+++ exited with 0 +++`; no unlink or rename
occurred. The node was then terminated by its exact PID and reaped, and port `9012`
was free.

This exact write stimulus therefore stops tripping on the current binary. The
task-authorized harness edit was not made, because substituting another probe would
violate the task's STOP rule and the read vector is closed. VERDICT 1 mechanism
identity, VERDICT 2 MapOnly/HoldRead experiments, VERDICT 3 salvage, the script red
proof, and the guard-mode decision are consequently not established in this run.

STOP: `human_gate_required` — environment drift: the required exact external
changing-value write no longer produced `db.sqlite-wal (deleted)`.

- [Task 002] Stop before script maintenance or guard/salvage experiments when the exact
  mandated changing-value write stimulus no longer trips; no alternate vector was used
  because the task explicitly closes the read-vector hunt and requires operator approval
  for substitution — `docs/plans/wal-unlink-durability/decisions-ledger.md`
