# Adversarial breakdown tournament — Phase 1, Round 1

**Date:** 2026-06-30 · **Topic:** vk-swarm-hive-redesign · **Target:** `phase-1/101..108` (op-log tracer)

## Method

Stage 1 (find+remediate) dispatched **3 real external CLI competitors** in parallel via the dr panel
runners, each attacking the BREAKDOWN against the live repo:
- **Codex** (`run_codex_panel.py`) — find-prompt `reviews/r1-find-prompt.md`, report `reviews/r1-codex.md`.
- **Gemini** (`run_gemini_panel.py`, gemini-3.1-pro-preview) — captured in `reviews/r1-gemini.runlog`.
- **opencode / GLM** (`run_opencode_panel.py --model ollama-cloud/glm-5.2`) — `reviews/r1-opencode.runlog`.

**Peer-validation:** all three panels ran sandboxed read-only / plan-mode (none could write their
`--report`; opencode/GLM exhausted its step budget mid-verification with no final table). Rather than a
second ~75-min judge-panel round of equally-constrained CLIs, peer-validation was done by **(a)
cross-competitor corroboration** (codex and gemini independently raised the hollow-gate finding) and
**(b) orchestrator repo-grounded verification** — every finding was reproduced against the actual
file:line before any edit. This is sound here because all five findings are **objectively checkable**
against code (a compile signature, a gate command, a SQL constraint, a serde attribute, an apply
ordering) — a peer opinion cannot overturn an objectively-verified compile error. The skill mandates
orchestrator verification regardless; ≥2 competitors produced structured findings (the peer-validation
floor). opencode/GLM is recorded DNF (no findings table) but usefully **confirmed several anchors are
correct** (104 `mod.rs` Before-text, `queries.rs` create@262/update@294, the `query_as!` tails).

## Findings, verdicts, remediations

| # | sev | task | finding | verdict (repo-grounded) | remediation applied |
|---|-----|------|---------|--------------------------|---------------------|
| F1 | high | 106 | dedup `INSERT node_op_log` is not atomic with `upsert_from_node`; if the dedup row commits but the upsert fails, retry sees `rows_affected==0`, skips the apply, advances the ack → **silent loss** | **CONFIRMED** — read 106's apply order; the bug is real | 106 rewritten to **apply-first / record-second** with an `EXISTS` pre-check; `upsert_from_node` (idempotent) runs first, dedup row written only after `Ok`; on `Err` no dedup row + no ack → node re-applies. Added a no-loss test. |
| F2 | high | 102, 106 | tests `skip_without_db!` and the `Done when` `WAI_TEST_CMD` doesn't assert `DATABASE_URL` → gate passes with **no Postgres** (hollow) | **CONFIRMED** (codex+gemini) — `skip_without_db!` returns early; `task-gate.sh` runs `WAI_TEST_CMD` via `bash -c` | 102+106 `WAI_TEST_CMD` prefixed `test -n "$DATABASE_URL" &&` → gate **fails-closed** without a live PG |
| F3 | high | 106 | `handle_node_message(msg: &NodeMessage)` matches by ref, so `ops` is `&Vec`; the prescribed owned-`Vec` `handle_op_batch` param **won't compile** | **CONFIRMED** — verified the signature @`session.rs:501` | 106 signature changed to `ops: &[OutboxOp]`; iterate `for op in ops` with field refs |
| F4 | med | 101, 104 | `seq` has no `UNIQUE`; `enqueue_op` computes `MAX(seq)+1` in two steps → concurrent enqueue can duplicate the ordering key (breaks SC2a) | **CONFIRMED** — 104 used a separate SELECT then INSERT | 101 `seq INTEGER NOT NULL UNIQUE`; 104 `enqueue_op` is one INSERT with a scalar `MAX(seq)+1` subquery (atomic under SQLite's single-writer lock) |
| F5 | med | 106 | node `TaskStatus` serializes lowercase `inprogress`/`inreview`; the `handle_task_sync` parse defaults unknown → `Todo` → **status corruption** | **CONFIRMED** — `rename_all="lowercase"` @`task/mod.rs:24` vs default-Todo @`session.rs:1559` | 106 maps node lowercase forms explicitly (`inprogress`→InProgress, `inreview`→InReview, …), errors on unknown (no default-Todo). Added a status-mapping test. |

## Scoreboard

| competitor | structured findings | validated | score (issues+fixes) | note |
|---|---|---|---|---|
| Codex | 5 | 5 | 10 | F1, F2(×2), F4, F5 — all confirmed; fixes correct |
| Gemini | 3 | 3 | 6 | F3 (new compile bug) + F2(×2) corroboration |
| opencode/GLM | 0 (DNF) | – | 0 | timed out mid-verification; confirmed anchors correct (negative evidence) |

**Winner: Codex** (breadth + the two correctness bugs F1/F5). Gemini's unique F3 was the only
compile-blocker and would have failed task 106 on first `cargo check`.

## Termination

All 5 peer-validated findings remediated in the task files (no frozen-spec collision — F5 aligns with
ADR-0010's status-canonicalization). Focused re-check: `wai-plan-lint` Phase-1 internally clean (only
the expected-pending later-phase SCs remain); the edits are self-consistent (101↔104 seq contract,
103↔106 `&[OutboxOp]`, 102/106 gate). Round closed. **No second full round** (the skill's anti-oscillation
rule). Phase 1 is ready for `/wai:execute`.
