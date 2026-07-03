# Round 5 — Plan Fidelity Tournament Scoreboard

**Date:** 2026-07-03
**Target:** `vk-swarm-hive-redesign-p47` branch, HEAD `67996ab8`
**Governing intent:** `docs/plans/vk-swarm-hive-redesign/plan.md` (7-phase plan) + frozen spec `2ac86436…`
**Review lens:** Plan fidelity (was the plan followed? where did it diverge and why?)

## Models ran

| Model | How | Duration | Verdict |
|-------|-----|----------|---------|
| Codex (codex-cli) | local checkout | ~4min | REJECT (1 BLOCKING, 1 SHOULD-FIX) |
| Gemini (gemini-cli) | local checkout | ~8min | APPROVE-WITH-NOTES (9 INFO) |
| Claude/Opus (claude-cli) | local checkout | ~17min | REJECT (1 BLOCKING, 1 SHOULD-FIX) |

All 3 models ran successfully (no resilience fallback needed).

## Scoreboard

| Model | Points | Real Bugs Found | Co-finds | False Positives |
|-------|--------|-----------------|----------|-----------------|
| **Claude/Opus** (winner) | **7** | 1 BLOCKING (CF1: head-of-line wedge) | 1 SHOULD-FIX (CF2: missing DROP task) | 0 |
| Codex | 5 | 1 SHOULD-FIX (CF3: stale docs) | 1 BLOCKING (CF2: missing DROP task — co-find with Claude) | 0 |
| Gemini | 2 | 0 | 0 | 0 (9 INFO, all confirmed sound) |

## Findings Consolidation

### CF1 [BLOCKING] — Cross-phase convergence break (Claude found, verified real)

**The bug:** Three individually-defensible decisions combine to break a guarantee in the exact partition scenario SC3/SC5/SC7 target:

1. P2's permanent fence-reject (`session.rs:2127-2142`) used bare `break` — no `node_op_log` record, no `applied_through_seq` advance. Rejected op stays `acked_at IS NULL` forever.
2. `peek_unacked` is head-of-line (seq-ordered, filters `acked_at IS NULL`), so ALL subsequent ops for that node are blocked.
3. P4's entity-level dirty guard (`has_unacked_for_entity`) blocks ALL inbound reconcile for that task while the op is stuck.
4. P5's digest self-heal routes through the same `upsert_remote_task`, so it's defeated by the dirty guard.
5. P5's existence-based digest sees the task as "in sync" (still an active `shared_tasks` row), so it triggers neither resend nor pull.

**Net:** A task reassigned away from a partitioned node diverges forever, and that node's op-log head-of-line wedges.

**Root cause:** The fence-reject was the ONLY permanent reject that didn't SKIP+ADVANCE. All other rejects (unknown-status, hive-authored-transition, illegal-transition) use SKIP+ADVANCE (record in `node_op_log` + advance `applied_through_seq`). The fence-reject and no-active-assignment reject both used bare `break`.

**Fix:** Changed both reject branches (stale-token at `session.rs:2127-2142`, no-active-assignment at `session.rs:2144-2181`) from bare `break` to SKIP+ADVANCE+`break` — insert into `node_op_log` + advance `applied_through_seq` + `break`. This matches the pattern already used by the status-transition rejects. 3 test assertion blocks updated (stale-token test, no-assignment test, status-guard test) to assert the op IS now recorded + cursor advances.

### CF2 [BLOCKING/SHOULD-FIX] — Missing P7 follow-up DROP task (Codex found BLOCKING, Claude co-found SHOULD-FIX)

**The issue:** Ledger line 819 says Gate 2 ratified "follow-up DROP after P4/P5 confirm no reads. Adds a P7 follow-up task to the plan." No task 704 exists. Migration uses TRUNCATE (keep-but-empty), not DROP.

**Resolution:** The follow-up DROP is CANCELLED. After P4/P5 completion, the DISCARDABLE tables (`activity`, `auth_sessions`, `oauth_handoffs`, `revoked_refresh_tokens`) still have extensive active query references throughout `crates/remote/src` (REST routes, WS session, broker, listener, maintenance/partitioning, auth module, OAuth module). P4 collapsed ElectricSQL's inbound sync path but did NOT remove the Hive's own usage of these tables — they serve the Hive's activity stream, auth, and OAuth flows, not just ElectricSQL. The cutover migration's own comment confirms: "The tables stay (kept auth/activity code references them — the code removal is out of this workstream's scope)." The follow-up DROP was predicated on P4/P5 removing all reads; since they didn't, dropping would break the application. Keep-but-empty is ratified as FINAL. Ledger updated with full evidence.

### CF3 [SHOULD-FIX] — Stale docs from P4/405 (Codex found)

**The issue:** `crates/services/src/services/share.rs:9` still said "Task sync from Hive to local is now handled by ElectricSQL"; `crates/services/src/services/mod.rs:10-14` still labelled `share` under "Legacy Sync Modules (Deprecated)" / "being replaced". Task 405's Done-When required updating these.

**Fix:** Updated `share.rs:9` to reference WS activity stream (ADR-0007 / SC7) instead of ElectricSQL. Updated `mod.rs:3-17` to remove the stale "Electric SQL Integration" section and "Legacy Sync Modules (Deprecated)" label, replacing with "Inbound Sync (WebSocket Activity Stream)" that correctly describes `share` as the single live inbound channel.

## Lessons learned

1. **Cross-phase convergence bugs are invisible to per-task reviews.** The fence-reject at P2 was correct in isolation (don't apply a stale op). The dirty guard at P4 was correct in isolation (don't reconcile a task with pending ops). The digest heal at P5 was correct in isolation (existence-based compare). But together they created a permanent wedge in exactly the scenario the redesign was meant to fix. Only an integrated cross-phase review caught this.

2. **The SKIP+ADVANCE pattern is an invariant, not a suggestion.** Every permanent reject in the apply loop MUST record in `node_op_log` and advance `applied_through_seq` — otherwise `peek_unacked`'s head-of-line ordering wedges the entire outbox. The fence-reject was the only branch that violated this invariant. The fix is not new logic; it's applying the existing pattern consistently.

3. **Ledger follow-up tasks must be either executed or explicitly cancelled.** Gate 2 ratified a follow-up DROP task that was never created. The ledger entry sat as an open commitment. When P4/P5 completed without removing the table reads (because the tables serve purposes beyond ElectricSQL), the follow-up should have been formally cancelled with evidence — not left as an open ledger item.

## Gate status (post-remediation)

- `cargo clippy --all --all-targets --all-features -- -D warnings`: clean
- `cargo test -p db --lib`: 196 pass / 0 fail
- `cargo test -p remote --lib`: 94 pass / 0 fail (3 test assertion blocks updated)
- `cargo test -p services --lib -- --skip terminal_session`: 205 pass / 0 fail
- `cd frontend && npm run lint`: clean
- `cd frontend && npx tsc --noEmit`: clean
