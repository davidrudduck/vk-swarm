# Breakdown tournament — round 1 (2026-08-28)

method: external-cli-tournament (3 competitors, non-self peer-validation, rotation judging)
target: the wal-unlink-durability decompose breakdown (9 tasks, 5 phases) + frozen spec
termination: CLOSED — every peer-validated finding remediated and the focused re-check
(`wai-submit-plan.sh` replaying `wai-plan-lint.sh`) PASSES on the resubmitted tree.

## Panels

| Seat | Model/CLI | Find report | Judge of |
|---|---|---|---|
| A | Codex CLI (`run_codex_panel.py --task plan-review`) | `round-1-codex.md` (10 findings) | judged BY Grok (C→A) |
| B | Claude CLI opus (`run_claude_panel.py --model opus`) | `round-1-opus.md` (13 findings) | judged BY Codex (A→B) |
| C | opencode `xai/grok-4.20-0309-non-reasoning` (first choice `openrouter/z-ai/glm-5.3` failed server-side: model not configured) | `round-1-grok.md` (7 findings) | judged BY Opus (B→C) |

Judge verdicts: `round-1-judge-{codex,opus,grok}.md`. Rotation: Codex judged Opus,
Opus judged Grok, Grok judged Codex. Orchestrator independently re-verified every
finding against the repo before applying (a judge can also be wrong).

## Scoreboard

| Competitor | Real findings (validated) | Rejected | Fixes accepted |
|---|---|---|---|
| Opus | 11/13 (6 as-is + 5 judge-corrected) | 2 | 11 |
| Codex | 8/10 | 2 (incl. #2 literal fix — superseded by the 031 redesign; #7 already-present) | 7 |
| Grok | 2/7 | 5 | 2 |

Notes: Grok-as-judge applied an invalid "the code doesn't exist yet" framing to Codex's
findings — the tournament attacks task TEXT, not code; the orchestrator re-verified each
against the repo per protocol. Grok's own refuted findings included fabricated
`session_data` / salted-hash claims (migration 20260821000000 L41-48 has exactly
(id, token_hash, hive_user_id, created_at, revoked_at); seams.rs L77-85 is plain
lowercase-hex SHA-256), a "dependency cycle" misread of `conflicts_with` (correct
file-overlap encoding), a claim the covers mapping was incomplete (1:1 complete), a
non-existent `PRAGMA wal_index`, and an allowed_change objection to single-file `edit`.

## Headline finding (spec amendment — ADR-0001 path)

Opus #8-class + Codex's mechanism probe converged on a REAL design defect in the frozen
spec itself: a **fresh post-unlink connection's `BEGIN EXCLUSIVE` fences nobody** —
WAL-mode writers coordinate through the shm segment; the node's pooled conns still use the
deleted shm inode, and same-process fcntl locks do not self-conflict. Remediation
(amended into spec §3 + re-frozen via `/wai:precheck`, new token committed separately):

- the monitor retains a DEDICATED connection opened AT SPAWN (pre-unlink, old shm domain);
- salvage checkpoint runs through THAT connection (a fresh pooled conn opens the NEW
  empty inode and checkpoints nothing);
- the refusal latch is a held `BEGIN IMMEDIATE` on that same dedicated connection
  (RESERVED lock in the old domain → old-domain writers fail loud, readers unaffected);
- ordering: salvage FIRST, then latch (latch-first would block salvage);
- arm-failure → fail CLOSED: close the pool (writes AND reads fail fast), log the D6
  deviation loudly.

## Remediations applied (all peer-validated, orchestrator-verified)

- 001: project created once per leg (200 + `.success==true`; NO 201 exists in
  crates/server/src; duplicate git_repo_path also returns 200); leg-B post-trip write
  must show a DB-failure signal AND offline absence; single script-scope EXIT trap over
  a NODE_PIDS array (`trap ... RETURN` unreliable under `set -e`); preflight unsets
  VK_WAL_GUARD; leg A exports VK_WAL_GUARD=on; LEGS=A|B|AB + MODE=baseline contract
  (no fixed-code assertions — runnable against an unfixed main binary for 040).
- 002: gains edit-rights to the repro script (allowed_change mixed) — encode VERDICT 1's
  observed trigger window into leg B and re-prove the red state (closes the
  "nobody may fix the script" dead-end).
- 010: `use sqlx::{ConnectOptions, Connection}` (E0599 otherwise); `options_for` is
  pub(crate) (030/031 reuse); reconnect_restores_read_mark unit test; TS3 test proves
  durability through a full close + fresh offline reopen.
- 020: `run(mut self)` / `&mut self` signatures; last_wal_state seeded synchronously in
  spawn; trip idempotence (early-return + trip_events counter) kills the 60s re-fire
  loop; cross-platform WalState{Absent,Present(Option<u64>)} with cfg-gated
  wal_identity (Windows targets in pre-release.yml L182-193); guard-unavailable
  escalation trips once; ACKED shutdown (oneshot) replacing fire-and-forget
  (WalMonitorHandle::shutdown was detached, L122-125); TS1 gains monitor-level
  trip/no-trip/idempotence tests via create_test_pool.
- 021: watch is LOOP-LOCAL in run() (a struct field + &mut self select arm is E0499);
  installed before the first metadata reconcile; re-created on the 60s tick after death.
- 030: salvage via the dedicated pre-unlink conn (not the pool); salvage→latch ordering
  with rationale; TS5 does a REAL fs::remove_file unlink, closes ALL original conns,
  and asserts offline durability from a fresh connection.
- 031: RefusalLatch redesigned (BEGIN IMMEDIATE on the dedicated conn; fail-closed
  pool-close fallback; DP-level stop trigger); TS2 really unlinks then proves
  pre-existing pooled write fails + read succeeds.
- 022: `db_path: PathBuf` threaded through from_parts/new/for_test (from_parts is a test
  seam — tempdir DBs at L1340/L1373, DBService literal at L533-549; database_path()
  inside from_parts would point tests at the production DB).
- 040: baseline via `git worktree add --detach ... origin/main` (main is checked out at
  /data/Code/vk-swarm) + isolated CARGO_TARGET_DIR on /data (/tmp quota); both binaries
  run MODE=baseline; new stop trigger for the checked-out-main case.

## Dropped (peer-rejected; no re-litigation)

Opus#4 (TS3 negative control — 002's control leg supplies the differential), Grok#1/#2/#3/#4/#6
(refuted above), Codex#2's literal fix (superseded by the 031 fail-closed redesign),
Codex#7 (content already present in 010).

## Focused re-check

`wai-submit-plan.sh` (full re-render + plan-lint replay) PASSED after remediation —
one symbol-grounding nit (`wal_unlinked_externally (020)` read as a call) fixed and
re-passed in the same session. Round CLOSED per the termination rule; no confirmation
round launched.
