# Code Review — Round 1

**Target:** branch clever-pangolin (post-merge squash on main)   **Range:** `e61d5f3d..72f20ea0`   **Effort:** high

Run retroactively as the `/wai:close` pre-graduation gate (graduation itself had already been
performed manually during ship; the merge was user-instructed). Four parallel finder subagents
covered: (A) wal_guard / wal_monitor / db lib, (B) main.rs / file_logging / local-deployment /
container, (C) incidental clippy/cleanup files, (D) scripts/live/wal-unlink-durability-repro.sh.
Every finding below was verified against a real `file:line` by the finder before being reported.

## Findings

| # | File:line | Severity | Category | Finding | Confidence | Actionable? |
|---|-----------|----------|----------|---------|------------|-------------|
| 1 | crates/db/src/wal_monitor.rs:365,:465 | medium | correctness | Monitor task never exits when all `WalMonitorHandle` senders drop without `shutdown()`: select arm `Some(cmd) = rx.recv()` never matches `None`; leaked task holds pool, guard, salvage conn, inotify watch. | high | yes — `cmd = rx.recv()` arm with Shutdown-equivalent teardown on `None` |
| 2 | crates/db/src/wal_monitor.rs:723,:733 | low | correctness | `pool.close().await` inside `handle_trip` blocks the monitor loop (pool close waits for checked-out conns); queued `Shutdown` then hits the 10s ack timeout and the monitor leaks. | medium | yes — spawn the close |
| 3 | crates/db/src/wal_monitor.rs:399-408,:493-502 | low | correctness | Periodic TRUNCATE arm ignores `tripped` state: post-trip it releases/reacquires the read-mark and checkpoints on a fenced/closed pool → permanent warn-log spam. | high | yes — skip when tripped |
| 4 | crates/db/src/wal_monitor.rs:433-453,:505-525 | low | correctness | Guard liveness/reconnect keeps running after a trip → indefinite `WAL guard reconnect failed` error spam; reconnect serves no purpose once the fence is armed. | high | yes — skip when tripped |
| 5 | crates/db/src/wal_monitor.rs:106-109,:120-130 | low | correctness | Windows: `wal_identity` returns `None` off-unix so `Replaced` can never classify — unlink+recreate between 60s polls is silently missed. Unix/macOS unaffected. | high | yes — document the platform gap |
| 6 | crates/db/src/wal_guard.rs:64-82 | low | correctness | `WalGuard::reconnect` leaves stale `holding_read_mark`: set `true` unconditionally; error path after `self.conn` replaced returns `Err` with flag true but no open tx → spurious `COMMIT` later. | high | yes — reset flag before connect; HoldRead reacquires anyway |
| 7 | crates/db/src/wal_monitor.rs:363-454 vs :463-526 | low | quality | ~90 lines duplicated verbatim between Linux / non-Linux cfg blocks (command match, truncate arm, liveness block) — fixes must be applied twice and can drift. | high | yes — extract `handle_command` / tick helpers; only inotify behind cfg |
| 8 | crates/db/src/wal_monitor.rs:554-659 | low | quality | Duplicated match arms in `check_wal_size` (Appeared ≡ Unchanged-Present; Replaced\|Vanished ≡ Unchanged-Absent-ever-present). | high | yes — restructure with early Present-path return + `log_unlinked_and_trip()` |
| 9 | crates/db/src/wal_monitor.rs:666-668 | low | quality | Unreachable `_` catch-all arm masks future exhaustiveness regressions. | high | yes — delete |
| 10 | crates/db/src/wal_monitor.rs:138-145 | low | quality | `UnlinkedEvent` struct is dead weight: `event` field never read (`#[allow(dead_code)]`); constructed identically twice; tracing macros hardcode the event name. | high | yes — remove struct |
| 11 | crates/db/src/wal_guard.rs:8-11 | low | quality | `Mode::HoldRead` + release/reacquire machinery is production-dead (only `MapOnly` is constructed in production) — tested-but-unused state machine. | high | yes — document retention decision (kept: truncate-window read-mark release is the designed use) |
| 12 | crates/local-deployment/src/lib.rs:458-461 | low | correctness | Failed **initial** `WalGuard::connect` forfeits prevention for the process lifetime — monitor reconnect only runs when `guard: Some(..)`. Fail-open is spec'd, but a lazy retry is compatible with it. | high | yes — `guard_pending` flag; monitor lazily retries connect on liveness ticks |
| 13 | crates/local-deployment/src/lib.rs:99-101 | low | quality | Unnecessary `#[allow(dead_code)]` on `wal_monitor_handle` — the field is read at shutdown (:897). | high | yes — remove attribute |
| 14 | crates/local-deployment/src/lib.rs:448-479 | low | quality | No wiring test pins the new from_parts WAL block (guard connect + monitor spawn + VK_WAL_GUARD=off degrade). Live repro covers it end-to-end; unit gap only. | high | yes — `wal_monitor_wired_and_shuts_down` test |
| 15 | scripts/live/wal-unlink-durability-repro.sh:331-332,48 | low | correctness | `setsid sqlite3` external-write session is never tracked; EXIT trap only iterates `NODE_PIDS` → detached sqlite3 leaks on interrupt, holding a WAL write lock. | high | yes — track PGID, kill `-$pgid` in trap |
| 16 | scripts/live/wal-unlink-durability-repro.sh:93 | low | correctness | Scratch node binds `HOST=0.0.0.0` while every client uses 127.0.0.1 and no API key is set — unnecessary LAN exposure. Also makes the loopback-only `port_is_free` probe (m0262 finding D6) moot. | high | yes — `HOST=127.0.0.1` |
| 17 | scripts/live/wal-unlink-durability-repro.sh:526,533,591 | low | correctness | Leg B retry rewinds `PASS_COUNT`, so the final summary undercounts vs printed PASS lines (audit drift; exit code still driven by FAIL_COUNT). | high | yes — annotate the rewind in the transcript |
| 18 | scripts/live/wal-unlink-durability-repro.sh:268,403-407 | low | correctness | `trip_detector` returns 1 for both clean timeout and node-death (`/proc/$pid/fd` gone); Leg A records a node crash during the detector window as PASS for "no external WAL unlink". | medium | yes — distinct return code 2 for node-death; callers FAIL on it |
| 19 | scripts/live/wal-unlink-durability-repro.sh:55 | low | correctness | `NODE_PIDS=("${kept[@]}")` aborts under `set -u` on bash < 4.4 when the array is empty. | medium | yes — `${kept[@]+"${kept[@]}"}` idiom |
| 20 | scripts/live/wal-unlink-durability-repro.sh:262-324 | low | quality | Three near-identical ~20-line bounded-poll loops (trip/unlink-event/refusal-latch detectors) with recurring magic constants. | high | yes — extract `poll_for_evidence` helper |
| 21 | scripts/live/wal-unlink-durability-repro.sh:609,8-11 | low | quality | No usage guard: `--help` or a typo'd flag silently launches a full multi-minute two-leg run. | high | yes — arg guard + `--help` |

(Numbering is continuous across finders; #21 rounds out the actionable list.)

## Non-actionable

| # | File:line | Severity | Category | Finding | Confidence | Why non-actionable |
|---|-----------|----------|----------|---------|------------|---------------------|
| N1 | crates/db/src/lib.rs:82,85 | low | quality | Swallowed pragma errors in `with_refusal_after_release` (`let _ =`) | high | sqlx `after_release` already warns on Err (sqlx-core-0.8.6 pool/connection.rs:298 `tracing::warn!(%error, "error from after_release")` then close_hard). A per-site warn was implemented, caused a test flake (after_release error→close_hard churn + warn burst against armed latch on max_connections(5) test pool), and was reverted; baseline 8/8 green |
| N2 | crates/local-deployment/src/lib.rs:453 vs :689-697 | low | correctness | Guard not taken before pool creation/migrations; orphaned-image cleanup task races the guard connect | high | Structural: `options_for` uses `create_if_missing(false)` — the DB file must exist before the guard can connect. Startup window is inherent to the design; documented |
| N3 | crates/server/src/main.rs:273-275 | low | correctness | `serve` Err skips `perform_cleanup_actions` (no monitor shutdown / final checkpoint / pool close) | high | Pre-existing (predates this workstream); logged as BACKLOG F-2026-09-02-01 |
| N4 | crates/db/src/lib.rs:702-713 | low | correctness | Migration pool is the one production pool not wrapped in `with_refusal_after_release` | medium | Unreachable: `WAL_WRITE_REFUSAL_ACTIVE` can only be set by a WalMonitor spawned after that pool is dropped |
| N5 | crates/server/src/main.rs:380-388 | low | quality | tracing + eprintln emit every checkpoint outcome twice | high | Deliberate per ledger: "durability outcome must be visible even if the log config regresses again" |
| N6 | crates/db/src/wal_monitor.rs:215-217 | low | correctness | `zero_pooled_busy_timeout` drains only idle conns; a concurrently-acquired conn keeps 30s busy-timeout | medium | Inherent tradeoff, documented at wal_monitor.rs:208-211 and lib.rs:76-77; no better sqlx hook without the documented deadlock |
| N7 | crates/db/src/wal_monitor.rs:380-383 + lib.rs:71-73 | low | correctness | Shutdown window: checked-out old-domain conns are not `query_only` between latch drop and pool return | medium | Window is during process shutdown; held conns cannot be fenced by construction; return path IS fenced |
| N8 | crates/db/src/wal_monitor.rs:875 | low | correctness | inotify events funneled through unbounded channel can accumulate during a long TRUNCATE | medium | Event volume on one DB dir is tiny; a bounded channel risks dropping the very removal events the feature detects |
| N9 | crates/db/src/wal_monitor.rs:533,:283 | low | quality | `std::fs::metadata` (blocking) called in async context | high | Microsecond local-FS syscall on 60s/event cadence; `spawn_blocking` adds complexity for no measurable benefit |
| N10 | crates/db/src/wal_monitor.rs:334-339 | low | correctness | `interval(Duration::from_secs(u64::MAX))` constructed when truncate disabled | high | Safe: guarded by `if truncate_enabled`, ticker never polled; construction cannot panic |
| N11 | crates/db/src/wal_guard.rs:21 | low | quality | `format!("sqlite://{}", path)` breaks on paths containing `?`/`%` | high | Pre-existing pattern identical to `database_url` construction in lib.rs:366,:433; out of scope |
| N12 | crates/local-deployment/src/container.rs:2007-2023 | — | correctness | stop_execution terminates the OS process despite `update_completion` failure | high | Explicit task-040 remediation per ledger ("leaving an executor running is not [acceptable]"); callers surface/log the error |
| N13 | crates/local-deployment/src/lib.rs:894-898 | — | correctness | Monitor stopped after compaction/bus, before final checkpoint | high | Correct and documented: latch stays armed through writers' in-flight window; final checkpoint runs after on fenced (query_only) conns |
| N14 | crates/server/src/file_logging.rs:82 | — | correctness | `vks_node_server=` filter directive | high | Verified correct: `[[bin]] name = "vks-node-server"` → tracing target `vks_node_server`; root-causes the silently-dropped shutdown logs |
| N15 | crates/remote/src/routes/relay.rs:192-218 | low | quality | Three near-identical boxed token-error arms could share a helper | high | Pre-existing duplication merely reformatted by the result_large_err boxing fix; out of scope |
| N16 | crates/server/src/routes/projects/handlers/with_stats.rs:52, templates.rs:119 | low | quality | Lowercase-name sorts could use `sort_by_cached_key` | high | Pre-existing characteristic, lists small, impact nil |
| N17 | crates/executors/src/logs/plain_text_processor.rs:117 | low | quality | `mem::take` drops buffer capacity that `drain(..)` retained | high | Exactly what clippy `drain_collect` prescribes; capacity churn negligible for a log-line buffer |
| N18 | scripts/live/wal-unlink-durability-repro.sh:81 | low | quality | SCRATCH_ROOT never cleaned on any exit path | high | Intentional for post-failure forensics; documented in `--help` text |
| N19 | scripts/live/wal-unlink-durability-repro.sh (various) | low | correctness | Verified-safe items: no secret echo path; EXIT trap runs on INT/TERM and `$!` is the exec'd server PID; 17/16 check counts correct; `check_status` STOP cannot produce a false green (exit 1 whenever STOP_REASON set); `[ -e ] && var=1` exempt from `set -e`; cannot touch production :9002 (port hard-coded 9012 + preflight free-check) | high | Disproved by reading the script; nothing to fix |

## Remediation (same-session, per No Deferred Remediation)

All 21 actionable findings were remediated on branch `clever-pangolin` in this session
(uncommitted at the time of this record; committed as the round-2 remediation commit):

- wal_monitor.rs: `handle_command() -> bool`, `truncate_checkpoint_tick()`, `guard_liveness_check()`
  extracted (dedup; #7); both cfg loops use `cmd = rx.recv()` with a `None` arm running
  Shutdown-equivalent teardown (#1); truncate + liveness ticks skip when `tripped` (#3, #4);
  `guard_pending` field + lazy guard reconnect on liveness ticks (#12); `check_wal_size`
  restructured with early Present-path return, explicit `transition == Replaced` trip,
  `debug_assert` identity check, `log_unlinked_and_trip()` helper, `wal_identity_eq()` (#8);
  `_` arm and `UnlinkedEvent` deleted (#9, #10); `handle_trip` pool close spawned (#2);
  module doc Platform-support section (#5).
- wal_guard.rs: `reconnect` resets `holding_read_mark` before connect (#6); `Mode` doc comment
  records the MapOnly-only production reality and Windows gap (#11).
- local-deployment/src/lib.rs: `#[allow(dead_code)]` removed (#13); `wal_monitor_wired_and_shuts_down`
  wiring test (#14).
- repro script: `--help`/unknown-arg guard (#21); WRITE_SESSION_PGID tracked + killed in trap (#15);
  `HOST=127.0.0.1` (#16); PASS_COUNT-rewind annotation (#17); trip_detector return 2 on node-death
  with distinct FAILs (#18); bash<4.4-safe array idiom (#19); `poll_for_evidence` helper (#20).
- N1: per-site warn implemented then reverted after flake bisect (see table); N3: BACKLOG row
  F-2026-09-02-01.

## Verdict: With fixes

Actionable: [1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21]
