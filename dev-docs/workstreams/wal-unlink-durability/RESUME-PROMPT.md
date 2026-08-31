# Resume prompt — wal-unlink-durability in /wai:execute (cross-machine handoff, 2026-09-01)

Paste everything below the line into the new session.

---

Resume the WednesdayAI workstream **wal-unlink-durability** in the /wai:execute harness.

```
/wai:execute wal-unlink-durability
```

## Machine setup

- Repo: `davidrudduck/vk-swarm` (origin: https://github.com/davidrudduck/vk-swarm.git)
- Branch: `clever-pangolin` — `git fetch origin && git checkout clever-pangolin && git pull`
- Verify clean state: working tree clean, 59 commits ahead of `origin/main`, fully pushed.
- Repo root cwd; rust workspace + `frontend/`, `remote-frontend/` (gates per AGENTS.md).

## Where the work stands (verified 2026-09-01, HEAD 5b860a4b2)

Spec `docs/superpowers/specs/2026-08-28-wal-unlink-durability.md` — `change_kind: bugfix`,
`verify_cmd: bash scripts/live/wal-unlink-durability-repro.sh`. Two operator-approved
amendments (v2 fault-injection trip; external write-session vector) are frozen in the spec —
do not re-litigate them.

**All 9 decompose tasks are `status: passed`** (001, 002, 010, 020, 021, 022, 030, 031, 040) in
`docs/plans/wal-unlink-durability/phase-*/`. Task 040 (ship gate) is green: repro script exits 0
with 33 PASS / 0 FAIL (Leg A 17/0, Leg B 16/0); the last STOP (missing final-checkpoint log line)
was cleared by the `vks_node_server` filter-directive fix + `eprintln!` outcome (commits
`5f9fad167`, `5b860a4b2`) — see ledger `## 2026-08-31 — Task 040 final verification` (SUPERSEDED note).

Delivered: `WalGuard` dedicated wal-index guard connection (`crates/db/src/wal_guard.rs`, kill-switch
`VK_WAL_GUARD=off`), revived `WalMonitor` with inotify fast-wake + inode-transition classification
emitting `wal_unlinked_externally` WARN, write-refusal latch on trip (fail-fast `SQL_BUSY`-class
integrity error, reads continue), salvage checkpoint on trip with named events, wired into
`LocalDeployment::from_parts` with shutdown ordering, final-WAL-checkpoint outcome logged at shutdown.
Evidence logs in `docs/plans/wal-unlink-durability/evidence/` (040 reruns, SC3 perf baseline,
journal_mode artefact). Panel history in `docs/plans/wal-unlink-durability/reviews/`.

## Remaining work, in order

1. **Close gate ledger sections (blocking `/wai:ship`).** The decisions-ledger
   (`docs/plans/wal-unlink-durability/decisions-ledger.md`) has NO `## Reachability gate` and NO
   `## Deploy verification` section; `wai-evidence.sh` fails closed for a `bugfix` spec without
   them. Write both:
   - `## Reachability gate` — (a) call-path trace: production entry point (`vks-node-server`
     main → `LocalDeployment::from_parts` → WalGuard/WalMonitor spawn → pool acquire path),
     cited file:line from real code; (b) real-seam test: the live repro script drives the real
     binary end-to-end (leg A/leg B) — cite it plus any pool-level integration test; (c)
     incident-symptom assertion: API-committed write survives graceful stop (the 2026-08-28
     task-resurrection symptom) — repro Leg A asserts exactly this.
   - `## Deploy verification` — must contain a fenced code block quoting REAL command output
     captured from the scratch-node run (journal_mode line, PASS tallies, the
     `Final WAL checkpoint completed` log line). Re-run the repro fresh to capture it:
     `SCRATCH_ROOT=/tmp/wal-resume bash scripts/live/wal-unlink-durability-repro.sh`
     (build release binary first). Prose self-attestation is rejected by the gate.
2. **Re-verify the mandatory gate on this machine** (clippy/test workspace, frontend +
   remote-frontend lint/tsc/vitest per AGENTS.md) before shipping.
3. **`/wai:ship wal-unlink-durability`** — flips spec + workstream README to `shipped`, writes
   staging_pointers, flips BACKLOG row `F-2026-08-28-01` promoted→fixed, graduates docs into
   `dev-docs/` via /wai:close, commits + pushes. Do NOT merge anything manually before ship.
4. **Open the PR** (none exists as of 2026-09-01): `clever-pangolin` → `main` on
   `davidrudduck/vk-swarm` only. Then the post-merge live verify via the spec's `verify_cmd`
   (wai-verify.sh from the ship menu) once merged+deployed.

## Known open finding (stays in backlog — do not fix in this workstream)

`F-2026-08-31-01` (low, open): WAL write refusal surfaces as generic HTTP 500; wants a
refusal-specific ApiError variant + harness leg-B assertion. Recorded 2026-08-31 during the 040
panel; legitimately deferred as a tracked backlog row, not silent debt.

## Hard constraints

- NEVER touch the production node on :9002. Live runs use the scratch :9012 pattern
  (`HOST=0.0.0.0 BACKEND_PORT=9012` + `VK_DATABASE_PATH`/`VK_ASSET_DIR`/`VK_LOG_DIR`/
  `VK_BACKUP_DIR`/`VK_WORKTREE_DIR` under a scratch dir). Port 9012 must be free after runs.
- Never echo `VK_NODE_API_KEY` / `VK_CONNECTION_TOKEN_SECRET`; never dump `credentials.json`.
- Tests use `db::test_utils::create_test_pool[_with_migrations]()` only — no hand-rolled schemas.
- This is a vibe-kanban worktree: no `pkill`/`killall`/pattern kills — exact PIDs only.
- No PRs against `BloopAI/vibe-kanban`.
- Findings discovered along the way go into `dev-docs/BACKLOG.md` (via /wai:finding-new), not chat.
