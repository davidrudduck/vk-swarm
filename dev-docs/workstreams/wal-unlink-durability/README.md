---
workstream: wal-unlink-durability
doc_type: readme
status: shipped
title: "wal-unlink-durability"
staging_pointers:
  - dev-docs/workstreams/wal-unlink-durability/plans/wal-unlink-durability
  - dev-docs/workstreams/wal-unlink-durability/spec/2026-08-28-wal-unlink-durability.md
---

# wal-unlink-durability

wal-unlink-durability

## Acceptance evidence

Date: 2026-08-31. Branch `clever-pangolin`, HEAD at gate time `c30e2c77`. Frozen
`verify_cmd` (`bash scripts/live/wal-unlink-durability-repro.sh`) run fresh on a
cross-machine resume against a locally rebuilt release binary: exit 0, 33 PASS /
0 FAIL (Leg A 17/0 guard-on durability, Leg B 16/0 guard-off detection+refusal);
offline `PRAGMA journal_mode;` = `wal` on both legs; `Final WAL checkpoint
completed` present in the Leg B node log after graceful stop; port 9012 free
afterwards. Real command output and the SC1–SC3 verdict table:
`plans/wal-unlink-durability/decisions-ledger.md` sections `## Reachability gate`
and `## Deploy verification`; fresh logs under
`plans/wal-unlink-durability/evidence/wal-040-resume*.log`.

Mandatory gate (this machine, rustup stable 1.98.0):
`cargo clippy --all --all-targets --all-features -- -D warnings` EXIT 0;
`cargo test --workspace` EXIT 0 (68 suites, no failures); `frontend` lint EXIT 0,
`tsc --noEmit` EXIT 0; `remote-frontend` lint EXIT 0, `tsc --noEmit` EXIT 0,
`vitest run` EXIT 0 (54 files / 413 tests; Node 26 needs
`NODE_OPTIONS=--no-experimental-webstorage` — backlog `F-2026-08-31-03`, CI pins
Node 22 and is unaffected).
