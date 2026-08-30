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

## Historical — T1 mechanism evidence (2026-08-30, superseded by re-plan v2 below)

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
- [Task 002 orchestrator] VERDICT 1's evidence hunt (ledger `## Historical — T1 mechanism
  evidence (2026-08-30, superseded by re-plan v2 below)`, commit 797665bbf) established: the
  read-only CLI vector is NOT reproducible
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

## 2026-08-30 — Re-plan v2: fault-injection trip, lock-semantics guard verdict (operator-approved)
- [Task 002 orchestrator] Evidence: three hunts + an orchestrator probe (60+ external
  read/write sessions, lslocks captures) show the pool PERSISTENTLY holds shared POSIX
  locks on db + shm on the current binary — no external session can become the last
  wal-index holder; the incident predates this behavior or hit a rare pool-replacement
  window. Operator approved v2: leg B trips via deterministic fault injection
  (rm WAL/SHM — identical inode state; TS5 technique); guard mode (D4) decided by
  lslocks persistence of MapOnly vs HoldRead probes; VERDICT 3 salvage on the injected
  trip; leg A becomes regression coverage (external sessions provably safe). Spec
  amended (v2 block, SC2/SC4 wording) and re-frozen; 002 rewritten; 010 TS3 gains the
  lock-persistence differential assertion; 001/040 wording aligned.
  — files: spec, 001/002/010 task files, plan ledger

## T1 mechanism evidence (2026-08-30)

Host/tool versions: `sqlite3 3.53.4 2026-07-24`, `Python 3.14.7`, Linux
`6.8.0-138-generic`; binary `target/release/vks-node-server` on `:9012`.

### VERDICT 1 - persistent pool locks

Clean capture root `/tmp/opencode/wal-t1-nVisow`, node PID `4066118`: before and after the
single external `sqlite3 db.sqlite "PRAGMA user_version=$RANDOM;"` session, `lslocks` showed
the node's POSIX READ locks on `db.sqlite` (`1073741826..1073742335`) and `db.sqlite-shm` (`128..128`).
`/proc/4066118/fd` showed live `db.sqlite-wal` and `db.sqlite-shm` descriptors (including fds
14 and 15) before and after, with no `(deleted)` path.

On the current binary the pool holds the wal-index shared lock continuously, so an external
close-unlink is impossible in steady state (the clean capture above and the two prior hunts at
ledger lines 331-338). The incident window is therefore a pool-connection-replacement gap
(idle reap or connection error) or an older binary; the guard exists to hold exactly that lock
through those gaps. Fault injection by removing WAL/SHM reproduces the incident post-state
identically: node fds show `db.sqlite-wal (deleted)` and `db.sqlite-shm (deleted)`.

### VERDICT 2 - MapOnly

MapOnly was the minimal persistent-lock mode. Against a scratch node serving one authenticated
seeded-cookie API task write per second, PID `4068306` held the same POSIX READ locks on
`db.sqlite` and `db.sqlite-shm` at every 5-second capture from 0 through 60 seconds (13
captures; transcript `/tmp/opencode/wal-t1-map.log`). HoldRead also held those locks for the
same 0--60 second window (PID `4157993`; transcript
`/tmp/opencode/wal-t1-hold-checkpoint.log`), plus shm byte 124. While HoldRead remained open,
a stand-in connection's `PRAGMA wal_checkpoint(TRUNCATE)` returned `[(1, 880, 19)]`, confirming
that it blocks truncation. Select MapOnly: it supplies the required persistent shared lock
without HoldRead's checkpoint-blocking transaction.

### VERDICT 3 - A6 salvage viable

Superseded by `### VERDICT 3 redo — A6 salvage attribution` (operator amendment): counts retained for review history only.

Fault-injected salvage root `/tmp/opencode/wal-t1-ULlk2F`, node PID `3361`, surviving pre-trip
Python connection PID `3763`: after an API `marker-salvage-pre` write, `rm -f db.sqlite-wal
db.sqlite-shm` produced node fd entries for both `db.sqlite-wal (deleted)` and
`db.sqlite-shm (deleted)`. `PRAGMA wal_checkpoint(TRUNCATE)` through the surviving connection
returned `[(0, 0, 0)]`; after graceful node stop, offline `marker-salvage-pre` count was `1`.
Checkpoint via a surviving open fd therefore does flush old-inode frames; A6 salvage is viable.

### VERDICT 3 redo — A6 salvage attribution

The earlier full two-arm transcript used scratch root `/tmp/opencode/wal-a6-redo.cAmqk7`.
The retained re-run from `/tmp/opencode/wal-a6-redo.sh` emits
`/tmp/opencode/wal-a6-redo-transcript.log` (ARM A lines 7-14 and ARM B lines 22-28;
scratch root `/tmp/opencode/wal-a6-redo-20260830-101600`) and reproduces ARM A pre-stop `1` /
ARM B pre-stop `0`. Each arm booted a fresh `:9012` scratch node, seeded a
session, wrote its marker through the node API, then opened a Python survivor connection before
the trip and read `SELECT count(*) FROM tasks` (`[(1,)]`) to retain the pre-trip shm mapping.
Removing both named WAL files produced deleted node fd evidence in both arms.

ARM A (salvage): after the trip, the survivor ran `PRAGMA wal_checkpoint(TRUNCATE)` and returned
`[(0, 0, 0)]` (`busy=0`, `nLog=0`, `nCkpt=0`: `wal_checkpoint(TRUNCATE)`'s post-truncation
success signature; sqlite3.h specifies both `pnLog` and `pnCkpt` are zero after successful
TRUNCATE, not a zero-work result). The fresh
`file:<db>?immutable=1` pre-stop main-file-only read nonetheless counted `marker-a6-A` as `1`;
the post-graceful-stop count remained `1`.

ARM B (control): after the identical trip, the survivor did not run a checkpoint
(`[('not-run',)]`). The same fresh immutable pre-stop read counted `marker-a6-B` as `0`; after
the node's graceful stop it counted `0` (`/tmp/opencode/wal-a6-redo-transcript.log` line 30) —
NOTE: that post-stop `0` is a pre-fix probe artifact (read 4ms after stop, before the shutdown
checkpoint completed; see the ORCHESTRATOR CORRECTION below — reliable runs read `1`).

ORCHESTRATOR CORRECTION (2026-08-30, transcript-verified): the prior sentence's "REFUTED"
reading inverted the evidence. The arms differ ONLY in the salvage checkpoint: ARM A
(checkpoint run) reads the marker durable in the main file PRE-STOP (immutable=1, count=1);
ARM B (no checkpoint) reads 0 pre-stop. Per the task's verdict rule that is
exactly the A6-VIABLE signature: checkpoint-via-surviving-open-fd DOES flush old-inode frames
into the main DB. The reported `[(0, 0, 0)]` tuple does not overturn the differential — the
tuple reflects post-truncation state, and no alternative explanation survives ARM B reading 0
pre-stop. A6 is VIABLE solely because of that pre-stop differential; task 030 encodes the
`salvage_checkpoint_succeeded` assertion. ARM B's post-stop recovery is the node's own shutdown
checkpoint through its still-open fds (reproduced at `wal-a6-redo-r3-transcript.log:31` and
`wal-002-r4-full.log:61`); it is NOT the attribution basis for A6 — A6 rests solely on the
pre-stop differential (ARM A=1, ARM B=0, reproduced in both runs). The retained `post_stop=0`
reading in `wal-a6-redo-transcript.log:30` is a pre-fix probe artifact read 4ms after stop,
before the shutdown checkpoint completed. `/tmp/opencode/wal-a6-redo.sh` defaults `TRANSCRIPT`
to the older cited transcript, so a default re-run clobbers it; the retained record transcripts
are `wal-a6-redo-transcript.log` (first run, superseded wait) and
`wal-a6-redo-r3-transcript.log` (fixed probe). The deleted-fd, API response, survivor-read,
checkpoint, immutable pre-stop, and post-stop lines for both arms are retained verbatim there.

- [Task 002 orchestrator] SUPERSEDES the implementer decision below, which inverted the
two-arm evidence: the immutable pre-stop control comparison attributes ARM A's durability
to the salvage checkpoint (A6 viable). ARM B's post-stop recovery is the node's own shutdown
checkpoint through its still-open fds (reproduced at `wal-a6-redo-r3-transcript.log:31` and
`wal-002-r4-full.log:61`); it is NOT the attribution basis for A6 — A6 rests solely on the
pre-stop differential (ARM A=1, ARM B=0, reproduced in both runs). Task 030 keeps the
`salvage_checkpoint_succeeded` assertion; the refusal latch remains the post-salvage step per
the design.
  Superseded implementer line (retained for history): "Treat a survivor checkpoint returning
  `[(0, 0, 0)]` as zero-frame evidence ... implement named-failure plus refusal in place of
  A6 surviving-fd salvage" - `docs/plans/wal-unlink-durability/decisions-ledger.md`

### Harness delta and red proof

The authorized harness edit replaces the old repeated CLI-read stimulator with one external
`PRAGMA user_version=$RANDOM` write session in Leg A, where a detector timeout is an asserted
PASS, and deterministic `rm -f db.sqlite-wal db.sqlite-shm` in Leg B. It removes the
`INCONCLUSIVE` result/exit/message and renames the read-era success state and labels. It also
adds `python3` as an exit-2 preflight dependency (the `.success` parsing helpers) and makes the
fault-injection assertion a real predicate (both files existed, `rm` succeeded, both absent)
rather than unconditional. Existing
assertion accounting, sentinels, abort handling, auth STOP, PID lifecycle, and timing bounds are
otherwise unchanged.

Fresh remediation-round-4 evidence used the current script. `MODE=baseline bash
scripts/live/wal-unlink-durability-repro.sh` exited `0` with `Total PASS: 24` and `Total FAIL: 0`
(`/tmp/opencode/wal-002-r4-baseline.log` lines 48-49); both legs were `PASS=12 FAIL=0 TOTAL=12`
(lines 52-53). `LEGS=AB MODE=full bash scripts/live/wal-unlink-durability-repro.sh` exited `1`
with `Total PASS: 27` and `Total FAIL: 5` (`/tmp/opencode/wal-002-r4-full.log` lines 64-65).
Leg A was GREEN (`PASS=17 FAIL=0 TOTAL=17`, line 68); Leg B was RED (`PASS=10 FAIL=5 TOTAL=15`,
line 69). Its fault-injection detector fired after 2ms (lines 44-51), the named events
`wal_unlinked_externally` and `wal_write_refusal_active` were absent (lines 54-57), and the
current binary returned HTTP 200 with parsed `.success=true` for `marker-B-post` (line 58),
followed by an offline persisted-count failure (line 61).

- [Task 002] Select MapOnly because it persistently holds the required db/shm shared locks for
  the complete 60-second API-write probe and avoids HoldRead's observed checkpoint busy result -
  `docs/plans/wal-unlink-durability/decisions-ledger.md`
- [Task 002] Preserve the post-trip offline-count assertion despite its current-code mismatch;
  the deterministic trip and named-event failures are real, but changing the assertion would
  fabricate the required red-loss evidence - `scripts/live/wal-unlink-durability-repro.sh`

### Amended harness verification

The operator amendment classifies the prior `marker-B-post` offline count `1` as the expected
current-code RED failure: graceful shutdown checkpoints the write through the still-open fds.
The assertion remains pinned to the POST-FIX `count = 0` expectation. The evidence below is
from the post-remediation script; the older `wal-t1-full-amended.log` records a
pre-remediation script version.

`LEGS=AB MODE=baseline bash scripts/live/wal-unlink-durability-repro.sh` exited `0` with
`Total PASS: 24`, `Total FAIL: 0` (`/tmp/opencode/wal-002-r4-baseline.log` lines 48-49); both
legs were `PASS=12 FAIL=0 TOTAL=12` (lines 52-53).

`LEGS=AB MODE=full bash scripts/live/wal-unlink-durability-repro.sh` exited `1` as intended
(`/tmp/opencode/wal-002-r4-full.log` lines 63-71). Leg A was GREEN
(`PASS=17 FAIL=0 TOTAL=17`, line 68): the single write session executed and the trip detector
timed out after 29690ms (line 15). Leg B was RED (`PASS=10 FAIL=5 TOTAL=15`, line 69): fault
injection produced deleted-WAL fd evidence after 2ms (lines 44-51); `wal_unlinked_externally`
and its db-path check were absent (lines 54-55); the bounded pre-write
`wal_write_refusal_active` latch poll timed out after 29622ms (lines 56-57); `marker-B-post`
was accepted with HTTP 200 and parsed `.success=true` (line 58), and its pinned offline
`count = 0` assertion failed because the observed count was `1` (line 61).

- [Task 002] Poll `wal_write_refusal_active` before the post-trip write using the trip detector's
  30-second bounded polling shape, so post-fix refusal is proven armed before the write can race
  the salvage checkpoint - `scripts/live/wal-unlink-durability-repro.sh`
- [Task 002] Parse `.success` with explicit true and false helpers after status classification —
  non-2xx responses and valid 2xx refusal envelopes earn refusal credit, while successful 200
  envelopes and malformed responses remain distinct red signals —
  `scripts/live/wal-unlink-durability-repro.sh`
- [Task 002] Poll exact node PIDs before reaping the A6 probe — PIDs captured through command
  substitution are non-children, so `wait` alone cannot reliably observe graceful shutdown —
  `/tmp/opencode/wal-a6-redo.sh`

## 2026-08-30 — Phase-1 integrated review remediation

Seven BLOCKING findings from the phase-1 integrated adversarial review, all fixed in this session
(one line each, with the file the fix landed in):

1. Salvage connection opened in a SYNC constructor. `spawn`/`spawn_default` (wal_monitor.rs
   L140-166) are synchronous and task 022 calls them without `.await`, so an `.await` inside them
   cannot compile. The CALLER now opens the dedicated pre-unlink connection in async context and
   passes it into spawn as a trailing parameter; task 010 exposes a public opener in wal_guard.rs
   because `options_for` is pub(crate) and the wiring caller lives in another crate —
   `docs/plans/wal-unlink-durability/phase-4/030-salvage-checkpoint-on-trip-run-salvage-checkpoint-succeeded-failed-events.md`,
   `docs/plans/wal-unlink-durability/phase-2/010-walguard-module-dedicated-wal-index-holding-guard-connection-vk-wal-guard-kill-switch.md`,
   `docs/plans/wal-unlink-durability/phase-4/022-wire-walguard-walmonitor-into-localdeployment-from-parts-with-shutdown-ordering.md`.
2. Connect-only does not map the wal-index. Every dedicated old-domain connection (guard, salvage,
   refusal) now performs a dummy read at open — the VERDICT-2 and VERDICT-3 probes each issued a
   SELECT before their observation, and an unmapped connection holds nothing —
   `docs/plans/wal-unlink-durability/phase-2/010-walguard-module-dedicated-wal-index-holding-guard-connection-vk-wal-guard-kill-switch.md`,
   `docs/plans/wal-unlink-durability/phase-4/030-salvage-checkpoint-on-trip-run-salvage-checkpoint-succeeded-failed-events.md`,
   `docs/plans/wal-unlink-durability/phase-4/031-write-refusal-latch-on-trip-begin-immediate-on-the-dedicated-pre-unlink-connection-wal-write-refusal-active-event.md`.
3. A checkpoint returning Ok with a non-zero first column is BLOCKED, not salvaged (VERDICT 2
   recorded `[(1, 880, 19)]` for exactly that case). `run_salvage_checkpoint` now maps that row to
   an error so the emitted event is the failed one —
   `docs/plans/wal-unlink-durability/phase-4/030-salvage-checkpoint-on-trip-run-salvage-checkpoint-succeeded-failed-events.md`.
4. The TS3 `/proc/locks` self-pid assertion was vacuous — SQLite's shm fcntl locks are per-process,
   so the test's own pool already produced a matching READ lock. Replaced by a real differential:
   close the pool first, then run the external write session and assert the WAL still exists, so
   only the guard can be preventing the last-closer unlink (this also removes the metadata-panic
   path on the shm file) —
   `docs/plans/wal-unlink-durability/phase-2/010-walguard-module-dedicated-wal-index-holding-guard-connection-vk-wal-guard-kill-switch.md`.
5. Read-mark release/reacquire around the TRUNCATE tick was unconditional, so the first tick would
   have converted the selected MapOnly guard into a HoldRead one and permanently blocked TRUNCATE
   including the node's shutdown checkpoint. Both calls are now gated on HoldRead mode, and the
   stale "HoldRead expected" note was corrected to the recorded MapOnly verdict —
   `docs/plans/wal-unlink-durability/phase-2/010-...md`,
   `docs/plans/wal-unlink-durability/phase-3/020-walmonitor-revival-wal-path-fix-inode-transition-classification-wal-unlinked-externally-event-guard-ownership.md`,
   `docs/plans/wal-unlink-durability/phase-4/022-...md` (hardcoded HoldRead literal corrected to
   MapOnly per VERDICT 2).
6. WalState seeding missed the leg-B race: spawn seeds Absent, the first API write creates the WAL,
   and the removal can land before the next tick, so the incident read as benign. The monitor now
   tracks whether the WAL was ever observed present; Absent→Present updates state without tripping,
   and an Absent observation is benign only when the WAL was never seen —
   `docs/plans/wal-unlink-durability/phase-3/020-...md`.
7. Shutdown ordering and event payloads. The Shutdown arm now releases the read-mark (HoldRead
   only), drops the refusal latch, and only then acks, so the node's final TRUNCATE cannot race a
   live latch; and both named events carry the DB path in `path` (WAL path moved to a separate
   `wal_path` field) as SC2 and the harness require —
   `docs/plans/wal-unlink-durability/phase-3/020-...md`,
   `docs/plans/wal-unlink-durability/phase-4/031-...md`.
   Test fault injection in 030 and 031 now removes BOTH the `-wal` and `-shm` files, matching the
   harness's injection step.

SC1 DIFFERENTIAL NOTE. With the vacuous `/proc/locks` assertion gone, SC1's live differential rests
on task 010's pool-closed TS3 test plus the VERDICT-2 lock-persistence evidence (MapOnly held the
db and shm POSIX READ locks across all 13 captures, `/tmp/opencode/wal-t1-map.log`, copied to the
evidence directory below). Leg A of the live harness is REGRESSION COVERAGE only: on this binary an
external write session provably cannot unlink the WAL, so leg A's detector timeout proves the
absence of a regression, not the presence of guard prevention.

HARNESS CHANGE AND TRUE OBSERVED COUNTS. Leg B's assertion count went 15 → 16: the
`wal_unlinked_externally` grep became a bounded 30s poll (unchanged count; the node emits it
asynchronously while the fd-state detector trips in ~2ms) and a new post-trip read assertion was
added (authenticated `GET /api/projects` expecting HTTP 200 with a parsed `.success == true`),
which is the reads-continue property of the D6 posture — it passes on the unfixed binary (nothing
is refused) and post-fix (only writes are refused), and fails only in the fail-closed pool-death
deviation. Leg B also boots with `VK_WAL_CHECK_INTERVAL_SECS=5`, a documented divergence from the
60s production default so the monitor's poll fallback fires inside the harness's 30s windows.

Observed on the current (unfixed) binary, `target/release/vks-node-server`:
`MODE=baseline bash scripts/live/wal-unlink-durability-repro.sh` exited `0` with `Total PASS: 24`
and `Total FAIL: 0` (`/tmp/opencode/wal-ir-baseline.log` lines 48-49); both legs `PASS=12 FAIL=0
TOTAL=12` (lines 52-53). `LEGS=AB MODE=full bash scripts/live/wal-unlink-durability-repro.sh`
exited `1` with `Total PASS: 28` and `Total FAIL: 5` (`/tmp/opencode/wal-ir-full.log` lines 65-66);
Leg A `PASS=17 FAIL=0 TOTAL=17` and Leg B `PASS=11 FAIL=5 TOTAL=16` (lines 69-70). The five leg-B
reds are the unlink-event poll timeout (line 53-54), its db-path check, the refusal-latch poll
timeout, the accepted `marker-B-post` write, and the pinned offline persisted count; the new
post-trip read PASSED (line 59) as predicted for an unfixed node.

I2 POINTER. The `[(0, 0, 0)]` tuple returned by the ARM-A salvage checkpoint is
`wal_checkpoint(TRUNCATE)`'s post-truncation SUCCESS signature — sqlite3.h specifies both `pnLog`
and `pnCkpt` read zero after a successful TRUNCATE, so it is not a zero-work result. The live
reading of the two-arm evidence is the ORCHESTRATOR CORRECTION in
`### VERDICT 3 redo — A6 salvage attribution` above; A6 rests on the pre-stop differential
(ARM A = 1, ARM B = 0), not on the tuple.

EVIDENCE DIRECTORY. The load-bearing transcripts are now copied into
`docs/plans/wal-unlink-durability/evidence/` (`wal-a6-redo-r3-transcript.log`, `wal-t1-clean.log`,
`wal-t1-map.log`, `wal-t1-hold-checkpoint.log`, `wal-002-r4-baseline.log`, `wal-002-r4-full.log`,
plus this session's `wal-ir-baseline.log` and `wal-ir-full.log`). The `/tmp/opencode/...` citations
throughout this ledger are retained for history; they are ephemeral, and the repo copies are the
durable record. `.gitignore:91` excludes `*.log`, so these eight files were staged with `git add
-f` deliberately — the filenames are kept identical to the citations rather than renamed.

- [Integrated review] Open the salvage connection in the async caller and pass it into the
  synchronous spawn, rather than making spawn async — spawn has no other await and task 022 calls
  it from a non-await position -
  `docs/plans/wal-unlink-durability/phase-4/030-salvage-checkpoint-on-trip-run-salvage-checkpoint-succeeded-failed-events.md`
- [Integrated review] Replace the vacuous per-process lock assertion with a pool-closed external
  write differential, because SQLite's shm fcntl locks cannot distinguish the guard from the test's
  own pool within one process -
  `docs/plans/wal-unlink-durability/phase-2/010-walguard-module-dedicated-wal-index-holding-guard-connection-vk-wal-guard-kill-switch.md`
- [Integrated review] Add the post-trip read assertion and shorten leg B's WAL check interval,
  making the reads-continue property of D6 an asserted contract rather than an assumption -
  `scripts/live/wal-unlink-durability-repro.sh`

## Task 010 orchestrator

- [Task 010 orchestrator] sqlx 0.8 `Connection::close` takes `self`; cannot close a struct field. First amendment (reconnect from live conn + assert is_alive/holding_read_mark) FAILED expedited Opus breakdown: vacuous — a no-op reconnect() would pass; unused `sqlx::Connection` import would fail clippy -D warnings. Second amendment (Opus exact fix): TEMP-table probe + second BEGIN DEFERRED to prove conn replacement and rematerialised read-mark; drop `Connection` and `PathBuf` imports. reconnect implemented as `self.conn = self.options.clone().connect().await?;` then pragmas + dummy read + HoldRead BEGIN/SELECT. — files: 010 task file

- [Task 010 orchestrator] Panel round 2 (HEAD `10d725a13`): implementer CONFORMS. Grok/Opus cited dummy-read as hollow vs connect-only mutant because sqlx 0.8.6 `SqliteConnectOptions` already emits `PRAGMA journal_mode=WAL` on connect (`sqlx-sqlite-0.8.6/src/options/mod.rs:181`, `connect.rs:66-71`). Probe: no-conn wal=false; journal_mode-or-pragmas-without-dummy wal=true; dummy-read wal=true. TS3 still fails with no guard (spec L130 outcome locked). Dummy-read line kept because the task and later salvage opener (030) spell it; deleting it is plan-level, not implementer drift. DISCARD as implementation DEVIATES. Sibling advisory: `wal_guard.rs` created beside unlisted `crates/db/src/backup.rs` — not touched.

## Task 020 orchestrator

- [Task 020 orchestrator] Pre-resolutions: skip `self.refusal = None` (031 owns the field); NotFound→0 swallow rework authorized in `check_wal_size`; spawn/spawn_default trailing `guard: Option<WalGuard>` (zero callers); do not add salvage/inotify/refusal fields.

- [Task 020 orchestrator] Attempt 1 (`4e2fa18e7` feat + `8d95c7707` clippy) panel unanimous DEVIATES: `Replaced` routed into size-threshold (`Unchanged | Replaced if Present(_)`); `vanished_trips` hollow vs Replaced≡Unchanged. Attempt 2 (`e224af884`) dedicated `Replaced | Vanished` trip arm (no Absent guard) + `replaced_trips` (seed `Present(Some(u64::MAX))`). Stage-1 CONFORMS.

- [Task 020 orchestrator] Panel round 2 (HEAD `e224af884`): Grok CONFORMS. Opus/GPT DEVIATES on spec TS1 L128 "asserting the named WARN event fields". Task Failing-test section authorizes only `tripped`/`trip_events` assertions; 040 live harness (`scripts/live/wal-unlink-durability-repro.sh`) asserts the log line names the DB path (SC2). `replaced_trips` fails a Replaced≡Unchanged mutant and a no-op `check_wal_size`. DISCARD WARN-fields as plan-level, not implementer drift. Remaining SHOULD-FIX not rejected: guard-health after every `select` arm (extra probes, not a classification hole; 022 wires the guard); `replaced_trips` ungated on non-unix (host is Linux; Manual verification is compile-on-non-Linux); Replaced arm omits redundant `wal_ever_present = true` (already true whenever last was Present); `#[allow(dead_code)]` on frozen `UnlinkedEvent.event` (warn! uses the task-specified literal).

## Task 021 orchestrator

- [Task 021 orchestrator] `files:` amended to include `Cargo.lock` — adding `notify = "8.2.0"` under `[target.'cfg(target_os = "linux")'.dependencies]` always rewrites the workspace lockfile; without it Stage-1 file-set fails. Mechanical, not a behaviour change.

- [Task 021 orchestrator] Pre-resolution: do NOT STOP if the linux watch select arm requires `#[cfg(target_os = "linux")]` around watch locals / a cfg-split of the `run` loop. That is the spec's documented non-Linux poll-only degradation. Do not restructure existing command/tick/truncate arms beyond adding the watch arm + recreating the watch inside the 60s tick arm. Record the cfg split in the ledger if used.

- [Task 021 orchestrator] Pre-resolution: sibling `filesystem_watcher.rs` uses notify 8.2.0 + notify-debouncer-full, gitignore walk, canonicalize. Expected divergence (task-authorized): RAW un-debounced events for a single known filename — no debouncer, no gitignore. List every sibling exclusion/guard in the ledger and justify each divergence. Do not add `notify-debouncer-full`.

## Task 021 implementer

Sibling divergences documented (authorized):
- **notify-debouncer-full**: NOT USED. Sibling debounces at 200ms for general file changes; we need raw un-debounced events to detect WAL removal immediately (no debounce delay).
- **Gitignore walk + path_allowed filter**: NOT USED. Sibling scans .gitignore files and filters events against ignore rules for a recursive watch of potentially thousands of files; we filter by simple filename match (wal_basename) for a single known directory (NonRecursive), no gitignore relevance.
- **Canonicalize + path resolution**: NOT USED. Sibling canonicalizes all paths for gitignore matching and to detect paths outside the watched root; we watch only db_path.parent() with NonRecursive, no path resolution needed.
- **RecursiveMode**: DIVERGED. Sibling uses Recursive; we use NonRecursive (watch parent dir only, don't recurse into subdirs).
- **Channel capacity**: DIVERGED. Sibling uses bounded channel(64); we use unbounded tokio::sync::mpsc::UnboundedReceiver per spec (Event callback posts to unbounded channel).
- **Access event filter**: NOT NEEDED. Sibling explicitly filters `event.kind.is_access()` because recursive watch generates spurious access events; unlink detection on a single dir does not emit access events, no filter needed.
- **Permission error handling**: SIMPLIFIED. Sibling skips individual permission errors during gitignore walk; we fail the entire watch setup on error (tracing::warn + return None, degrade to poll fallback).
- **Watch scope**: NARROWED. Sibling watches root (recursive); we watch only db_path.parent() (non-recursive, single known filename, immediate detection).

Implementation notes: Red test written first (`is_wal_removal_matches_delete_and_rename_from`), confirm it fails on function absence. Added cfg-gated helper + watch setup + select arm. Non-Linux: 60s poll only, debug log at startup.

- Select arm now uses `pending()` when watch is `None` and sets `watch = None` on `recv()` returning `None` so the 60s tick can re-arm.

- [Task 021 orchestrator] Panel round 2 (HEAD `e47930ed7`): Opus CONFORMS, Grok CONFORMS. GPT DEVIATES on re-arm-after-`check_wal_size` race vs L52 "create watch before first metadata reconcile". L52 is the INITIAL create (already before the loop at `wal_monitor.rs:280`); L68 does not order re-arm vs `check_wal_size`. DISCARD as plan-level, not implementer drift. Round-1 closed-channel `None` fix confirmed. Marked passed.

## Task 030 implementer

- A6-true branch used (pre-stop immutable before==0 / after==1) — ledger ### VERDICT 3 redo L411-421 — crates/db/src/wal_monitor.rs
- `cargo test -p db wal_monitor` after SC13 amendment: test result: ok. 13 passed; 0 failed (Opus scratch mutant stub Ok((0,0,0)) fails after==1)

## Task 030 orchestrator

- Panel round 1 (HEAD `0180d5025`): Opus+GPT DEVIATES SC13 — `trip_runs_salvage_checkpoint` post-close offline n==1 stays green if `run_salvage_checkpoint` is stub `Ok((0,0,0))` (SQLite close-time checkpoint through still-open fds). Grok CONFORMS (task-text match). Adjudication: hollow is real; the spelled test contradicted this ledger's own L411-421 ("post-stop is NOT the attribution basis for A6"). First amendment (post-only immutable n==1) REJECTED by expedited Opus breakdown: real salvage also returns `(0,0,0)`, so post-only still fails to discriminate; also `no such table` panics and a reused immutable conn caches pages. Replacement: BOTH-sides `main_file_probe` (fresh `options_for(...).immutable(true)` each call) — `before==0` then `after==1` while pool+mon remain open. Also fix stray 9-space indent at `wal_monitor.rs:979`.

## Task 031 orchestrator

- Pre-resolutions (do not STOP for these):
  - `refusal: Option<RefusalLatch>` on `WalMonitor`; `spawn`/`spawn_default` init `refusal: None`. Do NOT add a spawn parameter.
  - All five existing `WalMonitor { ... }` test literals must add `refusal: None`.
  - `handle_trip`: keep 030 salvage first (`run_salvage_checkpoint` still uses `salvage_conn.as_mut()`). AFTER that match, `match self.salvage_conn.take()` and arm. Latch even if salvage returned Err.
  - `salvage_conn` is None: fail-closed with `error = "salvage connection unavailable"` (no `e`) then `self.pool.close().await`. Arm `Err(e)`: fail-closed with `error = ?e` then `self.pool.close().await`. Two log sites; do not invent a dummy sqlx::Error.
  - Shutdown BOTH linux (`wal_monitor.rs:305`) and non-linux (`:393`): after `release_read_mark`, `self.refusal = None;`, then the existing info log, then ack. Do not restructure other arms.
  - Test lives in `wal_monitor.rs` `tests` module (in-module visibility). Write it FIRST, verbatim.
  - STOP only if a task STOP trigger actually fires at test time (arm Err / write succeeds / read blocked).
