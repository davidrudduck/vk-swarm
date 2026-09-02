# Code Review — Round 2

**Target:** branch clever-pangolin (round-1 remediation, working tree)   **Range:** `git diff HEAD` (uncommitted round-1 remediation)   **Effort:** high

Verification review of the round-1 remediation itself (21 fixes across wal_monitor.rs,
wal_guard.rs, local-deployment/src/lib.rs, and the repro script), per the `/wai:close` loop.
Two parallel finder subagents: one on the Rust delta, one on the script delta. The round-1
non-actionable list (ledger `## Post-review known issues`) was in force — no adjudicated item
resurfaced.

## Findings

| # | File:line | Severity | Category | Finding | Confidence | Actionable? |
|---|-----------|----------|----------|---------|------------|-------------|
| 1 | crates/db/src/wal_monitor.rs:758 | medium | correctness | Incomplete round-1 fix: `handle_trip`'s `Err` arm spawns `pool.close()` but the sibling `None` arm (salvage conn unavailable at startup) still closes inline — the exact 10s-ack-blocking hazard the round-1 comment describes survives in that arm. | high | yes — spawn the close in the `None` arm too |
| 2 | crates/db/src/wal_monitor.rs:389-415 | low | quality | Lazy guard-connect retry unthrottled: `guard_liveness_check` runs after every command/event; with `guard_pending` stuck true each inotify event runs a full connect (up to 5s busy-timeout) + warn log. | high (behavior) | yes — throttle to one attempt per check interval (`last_guard_attempt`) |
| 3 | crates/db/src/wal_monitor.rs:401-404 | low | quality | Lazy reconnect hardcodes `Mode::MapOnly`; a future `HoldRead` wiring would silently come back weaker after a failed initial connect. Latent today (MapOnly-only production). | high | yes — carry the mode on `WalMonitor` (`guard_mode`, inferred from the initial guard, defaulting MapOnly) |
| 4 | scripts/live/wal-unlink-durability-repro.sh:40 | low | quality | New `--help` text advertises `MODE full \| quick` but preflight accepts only `full\|baseline` — advertised value exits 2. | high | yes — fix the heredoc |
| 5 | scripts/live/wal-unlink-durability-repro.sh:45 | low | quality | Help text omits exit code 2 (usage/preflight) added by the round-1 arg guard. | high | yes — document it |
| 6 | scripts/live/wal-unlink-durability-repro.sh:507 | low | quality | Leg B rc=2 (node died) path: after the distinct FAIL, attempt-1 flow still logs "detector timeout is provisional; retrying…" — misleading (no retry happens when FAIL_COUNT advanced). Verdict math correct; log noise. | high | yes — gate the provisional branch on `node_died` |

## Non-actionable

| # | File:line | Severity | Category | Finding | Confidence | Why non-actionable |
|---|-----------|----------|----------|---------|------------|---------------------|
| N1 | wal_monitor.rs:483-495,:554-563 | — | correctness | None-arm teardown equivalence vs Shutdown arm | high | Verified step-for-step: read-mark release (no-op unless held) → pool fence when `refusal.is_some()` → log → break; only the ack send omitted (no ack exists); break before `guard_liveness_check` |
| N2 | wal_monitor.rs handle_command | — | correctness | `true`/`false` → break/continue mapping at both cfg call sites | high | `true` only from Shutdown arm; both loops `if should_exit { break; }` |
| N3 | wal_monitor.rs check_wal_size | — | correctness | Arm coverage vs HEAD | high | Equivalent; `Replaced` tripped before the Present-path early return; `last_wal_state` reorder inert (run_checkpoint doesn't read it) |
| N4 | wal_monitor.rs debug_assert identity | — | correctness | Could `debug_assert!(wal_identity_eq)` fire legitimately? | high | Impossible: differing Some identities classify `Replaced`; Some/None mixing impossible per-process (cfg-gated) |
| N5 | wal_monitor.rs spawned pool.close() (Err arm) | — | correctness | Dropping the JoinHandle / ack-before-close | high | `Pool::close` sets the closed flag synchronously (new acquires fail fast); returns `()`; downstream only touches the pool in the already-broken trip path |
| N6 | wal_guard.rs reconnect flag reset | — | correctness | `holding_read_mark=false` before connect | high | Correct: reconnect only reached via `!is_alive()`; HoldRead reacquires post-connect; covered by `reconnect_restores_read_mark` |
| N7 | wal_monitor.rs tripped-skip in ticks | — | correctness | Any post-trip work wrongly skipped? | high | No: salvage uses `salvage_conn`; explicit commands route through `handle_command` unchanged |
| N8 | local-deployment test wal_monitor_wired_and_shuts_down | — | correctness | Determinism / task leak | high | 10s internal ack inside 15s wrapper; dropped handle triggers None-arm teardown on failure; no env mutation |
| N9 | wal_monitor.rs cfg loops | — | quality | Linux vs non-Linux drift after extraction | high | Arms identical apart from inotify branch; select! is unbiased |
| N10 | repro script PGID kill (76-78,362,371,377) | — | correctness | Stale PGID / wrong-pgroup kill | high | Init empty, set after `$!`, `-n`-guarded, cleared on both exit paths; PGID==session_pid (setsid semantics) |
| N11 | repro script HOST=127.0.0.1 | — | correctness | Any client assuming 0.0.0.0 | high | All clients loopback; server binds `format!("{host}:{port}")`, default already 127.0.0.1; behavior strictly narrowed |
| N12 | repro script arg guard | — | correctness | --help/unknown-arg behavior | high | Verified live: `--help` rc 0, bogus arg rc 2 with usage; `${1:-}` set-u safe; env interface untouched |
| N13 | repro script trip_detector rc=2 under `set -e` | — | correctness | Non-zero return aborting before inspection | high | Both call sites are `if` conditions (set -e suppressed); `elif [ $? -eq 2 ]` reads the detector rc correctly |
| N14 | repro script poll_for_evidence | — | quality | Extraction semantics drift | high | Identical budget/sleep/logging; probes carried no loop-local state |
| N15 | repro script PASS_COUNT annotation | — | correctness | Placement / summary math | high | Printed immediately before the rewind with correct values; LEG_PASS_COUNTS consistent |

## Remediation (same-session)

All 6 actionable findings fixed on `clever-pangolin` (uncommitted, folded into the remediation
commit): `None`-arm pool close spawned (#1); `last_guard_attempt` throttle per check interval
(#2); `guard_mode` field inferred from the initial guard via a new `WalGuard::mode()` getter,
used in the lazy connect (#3); help text `quick`→`baseline` (#4); exit-code-2 documented (#5);
`node_died` flag gates the Leg B provisional-retry branch with a distinct FAIL label (#6).
`bash -n` clean; `--help` rc 0 and unknown-arg rc 2 verified live.

## Verdict: With fixes

Actionable: [1,2,3,4,5,6]
