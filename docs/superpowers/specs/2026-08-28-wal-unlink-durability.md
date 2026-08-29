---
doc_type: spec
status: active
workstream: wal-unlink-durability
change_kind: bugfix
verify_cmd: "bash scripts/live/wal-unlink-durability-repro.sh"
---

# wal-unlink-durability — external sqlite3 sessions silently destroy node write durability

> **Amendment 2026-08-30 (operator-approved, re-frozen): incident vector corrected.** The
> original text below describes the trigger as an external read-only `sqlite3` CLI query.
> Phase-1 empirical evidence (decisions-ledger, `## T1 mechanism evidence (2026-08-30)`)
> established on the current binary: the read-only flow does NOT reproduce the unlink under
> any probed condition (`VK_SQLITE_MAX_CONNECTIONS=1`, quiescent delays, CLI
> `PRAGMA wal_checkpoint(TRUNCATE)`, overlap attempts), while an external **write session**
> reliably produces `db.sqlite-wal (deleted)` / `db.sqlite-shm (deleted)` in the node's
> `/proc/<pid>/fd` table. Everywhere this spec says "CLI read mid-flow" (US1, SC1, SC4,
> Approach, Design §5), the contracted stimulus is now **an external sqlite3 session
> mid-flow, write-vector confirmed**. The hazard class, layered design (D1-D6), success
> criteria, and test strategy are vector-agnostic and UNCHANGED; the live 2026-08-28
> observation (task-delete resurrection, deleted-WAL fd) stands as the incident record.

## Intent
When any external process (admin shell, monitor, backup script) opens the node's SQLite DB and closes cleanly while the node server is running, SQLite's close-time recovery unlinks the `db.sqlite-wal` / `db.sqlite-shm` files. The running node keeps writing through its open file descriptors into the now-unlinked inodes. Those writes are silently lost when the node exits. Observed live: a task deleted via the node API returned 200/202 and the API listed it gone, yet after a graceful node stop the row RESURRECTED in the main DB; `/proc/<node-pid>/fd` showed `db.sqlite-wal (deleted)` held by the node mid-flow.

Any deployment where an operator or tool opens the node's DB with an external `sqlite3` session while the node is up is affected (2026-08-30 amendment: write sessions confirmed as the trigger; read sessions not reproducible on the current binary). The current workaround (evidence protocol: API reads mid-flow, CLI reads only post-shutdown) is an operational bandage — this workstream makes the node safe (or safely self-healing) by design, with **no silent data loss in any path**.

The design was settled at /wai:spec on 2026-08-28 after a mechanism investigation: a single-process model did NOT reproduce the unlink (a live WAL-mode connection holding the wal-index blocks the external close), which shows the real-node mechanism is a lock-state window the first task of this workstream must pin empirically on the real binary. The fix route is layered (D1): prevention by a dedicated wal-index-holding guard connection (D4, gated on that investigation), plus a detection + named-event + salvage-checkpoint + refuse-writes safety net (D2/D3/D6) so no path loses data silently. If the salvage checkpoint proves unable to recover writes already in an unlinked WAL (A6), the contracted minimum is exactly US2: detect the split-brain state, log loudly, and refuse to keep writing into the void.

**Promotion record.** This workstream is promoted from backlog finding **F-2026-08-28-01** (`dev-docs/BACKLOG.md`). At ship, the finding row is flipped to `fixed`. Prior evidence (linked, not duplicated):

- `dev-docs/BACKLOG.md` — finding F-2026-08-28-01 (medium, open at promotion time).
- The node-task-delete-dangling-shared-id workstream's decisions-ledger — `## Post-review known issues` item **K6** (hazard + evidence protocol) and `## Deploy verification` (the live verification transcript on the :9012 scratch pattern). That workstream is unmerged at spec time, so its ledger file is intentionally not path-linked here; the durable on-main record is BACKLOG F-2026-08-28-01.
- Reproduction (established 2026-08-28, vector corrected 2026-08-30): run node on a scratch DB, perform an external `sqlite3` session mid-flow, then write via the node API, stop the node gracefully, re-open with sqlite3 — the API-committed write is gone. Without the mid-flow external session, everything is durable. Phase-1 evidence (2026-08-30): the deleted-WAL state is confirmed reproducible via an external write session; the read-only flow is not reproducible on the current binary.
- Mechanism probes run at /wai:spec (2026-08-28, this host): (1) python WAL-mode experiment — a live connection blocks external close-unlink; (2) `linkat` via `/proc/self/fd` on a deleted file fails with ENOENT — in-place WAL relink recovery is NOT viable; (3) repo-wide grep — `WalMonitor` (crates/db/src/wal_monitor.rs) has NO spawn site: it is dead code today.


## User stories
- **US1:** As a node operator, when an external `sqlite3` session runs against my live node's DB mid-flow (write-vector confirmed 2026-08-30), I expect every write the node API has already committed to remain durable across a graceful node stop.
- **US2:** As a node operator, when an external process invalidates the node's WAL, I expect the node to surface a named, actionable log event — and to refuse to keep writing into the void if recovery is impossible — rather than silently continuing.
- **US3:** As a node operator, when no external process touches the DB, I expect normal node operation to be unchanged: WAL mode retained and no performance regression.

## Success criteria
SC1: With the fix deployed, the scripted reproduction (start scratch node → external `sqlite3` session mid-flow [write-vector stimulus per the 2026-08-30 amendment] → API-committed write → graceful stop → offline `sqlite3` inspect) leaves the API-committed write durable in the DB (row present offline). → US1
→ US1
SC2: When an external close unlinks the node's WAL (repro leg run with the prevention guard disabled), the node log carries a named, actionable event for the condition (level WARN or above, fixed event name plus the DB path plus remediation text), and subsequent write attempts fail with a distinct integrity error — no silent continuation. → US2
→ US2
SC3: With no external CLI access, normal node operation is unchanged: an offline `PRAGMA journal_mode;` against the node DB reports `wal` after the run, and the node write path shows no perf cliff against a baseline measurement recorded in the decisions-ledger (WAL mode retained unless the design argues otherwise with measurements). → US3
→ US3
SC4: A scripted live reproduction exists (`scripts/live/wal-unlink-durability-repro.sh`) that runs the full flow — scratch node on the :9012 pattern → external session mid-flow (write-vector stimulus) → API write → graceful stop → offline inspect — in two legs (guard-on, guard-off) and is usable as this spec's `verify_cmd`: it exits non-zero on current code (red) and zero after the fix (green). → US1
→ US1

## Users
- **Node operators / admins** who run external `sqlite3` sessions against a live node DB for quick checks, monitoring, maintenance, or debugging.
- **Monitoring / backup automation** that opens the node DB while the node is running.
- **End users of the node API**, whose committed writes (creates, updates, deletes) can be silently rolled back by such an external session — data-integrity loss with no error surfaced anywhere.


## Constraints
- Node DB access in Rust tests goes through the established db test utilities (`db::test_utils::create_test_pool()` / `create_test_pool_with_migrations()`); no hand-rolled `CREATE TABLE` schemas.
- Never touch the user's production node on :9002; live verification uses a scratch node on the :9012 pattern (`HOST=0.0.0.0 BACKEND_PORT=9012` plus `VK_DATABASE_PATH`/`VK_ASSET_DIR`/`VK_LOG_DIR`/`VK_BACKUP_DIR`/`VK_WORKTREE_DIR` under a scratch dir).
- Never echo `VK_NODE_API_KEY` / `VK_CONNECTION_TOKEN_SECRET`; never dump `credentials.json`.
- Detection uses Linux facilities (inotify on the DB directory, `/proc`); the node deploys on Linux. The 60-second poll remains as a backstop, and the refuse-writes trip response is platform-independent, so non-Linux builds degrade to poll-only detection rather than losing the safety net.
- Promotion bookkeeping: this spec is promoted from `dev-docs/BACKLOG.md` F-2026-08-28-01; the ship flow flips that row to `fixed`.


## Out of scope
- Fixing SQLite itself.
- Multi-host / NAS filesystem WAL semantics.
- Changing the backup subsystem (separate concern, unless the investigation proves the backup path shares the hazard — in which case DP1 authorises a halt for scope renegotiation and a linked finding is filed; no silent scope growth).
- In-place WAL relink recovery (hardlinking the unlinked inode back via `/proc/self/fd`): probe-refuted at /wai:spec (linkat fails with ENOENT on the target host). Do not re-propose it at decompose.


## Approach
Investigation-first, then a layered fix in four moves:

1. **Instrumented root cause + red repro (SC4).** On the :9012 scratch pattern, run the real node binary, execute the mid-flow external session (write-vector stimulus per the 2026-08-30 amendment), and capture who unlinks the WAL (fatrace/strace on the scratch dir) plus the node's lock state at that instant (`/proc/<pid>/fd`, `lslocks`). Deliverable: the mechanism paragraph in the decisions-ledger and the red repro script `scripts/live/wal-unlink-durability-repro.sh`. This leg also validates or refutes the guard premise (D4 → DP2 if refuted) and the salvage-checkpoint premise (A6).
2. **Prevention (SC1).** A dedicated guard connection whose only job is holding the WAL wal-index mapped, so an external close can never become the last locker able to checkpoint+unlink (D4). Adopted only on T1 evidence; kill-switch `VK_WAL_GUARD=off` exists so the SC2 repro leg can still exercise the net.
3. **Detection + named event (SC2).** Revive the currently-dead `WalMonitor` (probe A3: no spawn site exists), wire its spawn at node startup, and extend it with an integrity watch: inotify on the DB directory for delete/move events on the WAL basename plus the existing 60s poll comparing WAL path presence and inode identity. An external-unlink transition emits the named WARN `wal_unlinked_externally` with the DB path and remediation; the current NotFound→0 silent swallow in `check_wal_size` is fixed to distinguish external unlink from non-WAL mode (D2).
4. **Salvage + refuse-writes (US2, SC1 backstop).** On the trip: attempt `PRAGMA wal_checkpoint(TRUNCATE)` through the pool (surviving connections read the orphaned WAL via their open fds), emit a named success/failure event, then hold an exclusive lock on a monitor-owned connection so subsequent writes fail loudly with a distinct integrity error while reads continue; the node stays up and the ERROR event names the operator remediation (restart the node) (D3, D6 per operator Q1).

Finally the verify half: the repro script runs two legs — guard-on (durability, SC1) and guard-off (named event + refusal, SC2) — plus a `PRAGMA journal_mode` assertion and write-path timing capture against a baseline recorded in the decisions-ledger (SC3).


## Design
All components live in `crates/db` and are wired at node startup; the write path and all existing call sites are untouched when no external access occurs (SC3).

**1. WalGuard (new, prevention).** A dedicated long-lived `SqliteConnection` opened outside the pool at startup (all four connect sites in `crates/db/src/lib.rs` share the same DB path; the guard attaches once at DBService init). It performs a dummy read so SQLite maps the wal-index (shm) and holds the associated shared lock for the connection's lifetime. The incident occurred while `min_connections=2` pool connections were alive (lib.rs:46-52, 392-407), demonstrating pooled connections do not reliably hold that lock — the guard exists to hold exactly it. The monitor health-checks the guard each tick; a dead guard is re-established with a named WARN (`wal_guard_reconnected`) so prevention never silently lapses (O9). `VK_WAL_GUARD=off` disables it for the SC2 repro leg.

**2. WalMonitor revival + integrity watch (detection).** `WalMonitor` (crates/db/src/wal_monitor.rs) already owns WAL size checks and PASSIVE/TRUNCATE checkpoints but is never spawned (probe A3). The workstream wires `WalMonitor::spawn_default` into node startup and extends it with: (a) an inotify watch on the DB directory for `IN_DELETE`/`IN_MOVED_FROM`/`IN_MOVED_TO` on the `db.sqlite-wal` basename, waking the monitor immediately (tokio-wrapped inotify; the `notify` crate or a thin libc binding — settled at decompose); (b) inode-identity tracking on the 60s poll (`std::os::unix::fs::MetadataExt::ino`) so a vanished or replaced WAL is detected even if the watch fd is lost (the poll also re-creates a dead watch); (c) a fix to the `check_wal_size` NotFound→0 swallow (wal_monitor.rs:230-233) so 'WAL missing while pool is WAL-mode' is never read as 'different journal mode'.

**3. Trip response (salvage + refusal).** On a detected external unlink: emit `warn!(event = "wal_unlinked_externally", path = ..., last_inode = ..., remediation = "node will refuse writes; restart the node after investigating")` (SC2's named, actionable event); then attempt `PRAGMA wal_checkpoint(TRUNCATE)` through the monitor's dedicated connection opened at spawn (pre-unlink): sharing the orphaned writers' shm domain, it reads the orphaned WAL through its open fds and flushes committed frames to the main DB if A6 holds — a pool-acquired connection could be a post-unlink connection attached to a NEW WAL inode and would never touch the orphaned one — and emit `wal_salvage_checkpoint_succeeded` (INFO) or `wal_salvage_checkpoint_failed` (ERROR) accordingly; the same pre-unlink connection then holds an open write transaction (`BEGIN IMMEDIATE`), because WAL-mode write coordination lives in the shared-memory segment: only a connection inside the orphaned shm domain can fence the orphaned writers, whose subsequent writes fail loudly with a busy/locked integrity error while WAL-mode readers are unaffected (a fresh post-unlink connection's `BEGIN EXCLUSIVE` cannot reach that domain); emit `error!(event = "wal_write_refusal_active", ...)` once. If the monitor's pre-unlink connection has itself died, the latch cannot fence old-domain writers — fail closed by closing the pool (writes AND reads fail fast; read availability is sacrificed, nothing is silent) and log the D6 deviation. The node stays up (D6, operator Q1). Writes already committed before the trip are either salvaged or named as lost — nothing is silent. (Mechanism corrected at the decompose tournament, 2026-08-28: adversarial review verified that a post-unlink `BEGIN EXCLUSIVE` cannot fence writers on the orphaned shm inode.)

**4. T1 mechanism validation (real binary).** The first task reproduces on the :9012 scratch node with fatrace/strace and lock-state capture to pin the unlinking process and syscall, proves the guard blocks it (D4; DP2 halt if refuted), and proves or refutes salvage-checkpoint-via-fd (A6). Outcomes are recorded in the decisions-ledger before later tasks proceed.

**5. Repro script.** `scripts/live/wal-unlink-durability-repro.sh` (new): scratch-node lifecycle on the :9012 pattern, mid-flow external session (write-vector stimulus), API write, graceful stop, offline inspect; two legs (guard-on, guard-off) plus `PRAGMA journal_mode` and timing capture. This is the spec's `verify_cmd`.


## Decisions
The full option table lives in the sidecar (`2026-08-28-wal-unlink-durability.decisions.json`). Summary of settled forks:

- **D1 fix route** — layered: prevention guard + detection/refuse net, investigation-first (chosen) over prevention-only (leak ⇒ silent loss returns, SC2 unimplemented) and detection-only (every CLI read rides the loss window).
- **D2 detection home** — revive and extend the dead `WalMonitor` and wire its spawn at startup (chosen) over a second standalone watchdog (duplicated machinery) and a per-write hook (hot-path cost, no idle coverage).
- **D3 salvage strategy** — checkpoint via open fds then refuse writes while staying up (chosen) over `/proc/self/fd` relink (probe-refuted: linkat → ENOENT) and byte copy-back (new inode does not heal the split world).
- **D4 prevention mechanism** — dedicated wal-index-holding guard connection, gated on T1 evidence with DP2 as the authorised halt (chosen) over `SQLITE_FCNTL_PERSIST_WAL` (governs the node's own closes, not the external CLI's) and filesystem permission games (breaks the node's own WAL operation).
- **D5 verify packaging** — one script, two legs (only genuine option; SC4 contracts a single `verify_cmd`).
- **D6 post-trip posture** — refuse writes, stay up (chosen by operator Q1, 2026-08-28) over checkpoint-then-exit (needs a supervisor; manual nodes go DOWN) and in-place pool recycle (Arc-shared pool swap re-creates the split it ends).

No decision deletes code, changes a contract or wire format, or is otherwise hard to walk back — every component is additive and the guard is env-flag-gated — so no ADR is required by the irreversible-decision rule.


## Test strategy
TS1: WalMonitor integrity watch: unit tests over a tempdir DB (db::test_utils) covering wal-vanished and inode-changed transitions, asserting the named WARN event fields and that the previous NotFound→0 silent swallow now distinguishes external unlink from non-WAL mode.
TS2: Write refusal: with the trip latch held, node write attempts fail with the distinct integrity error while reads succeed; asserted at the db layer using create_test_pool().
TS3: Guard effectiveness (Linux, sqlite3-CLI-gated): spawn the real sqlite3 CLI against a test-pool DB mid-write; with the guard active the WAL path survives the CLI close and committed rows remain visible to an offline inspect.
TS4: Live repro script (SC1/SC2/SC4): scripts/live/wal-unlink-durability-repro.sh on the :9012 scratch pattern — red on current code, green with the fix (guard-on leg), guard-off leg shows the named event plus refusal; also captures PRAGMA journal_mode and write-path timings for the SC3 baseline.
TS5: Salvage behavior: simulated external unlink in an integration test asserts the salvage checkpoint attempt, the recovered/failed named event, and the subsequent write refusal.
