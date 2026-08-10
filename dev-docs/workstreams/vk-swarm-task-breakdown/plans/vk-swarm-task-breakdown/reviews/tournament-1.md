# Breakdown tournament — round 1 (closed)

method: external-cli, 2 seats (degraded from 3)
date: 2026-08-07

## Seats

| seat | tool | outcome |
|---|---|---|
| codex | codex-cli 0.147.0 | competed (attempt 1 lost to read-only report write; re-run with print-to-stdout prompt → `find-codex2.md`, 13 findings) |
| claude | claude-cli panel | competed (`find-claude.md`, 10 findings) |
| agy (gemini seat) | agy CLI | FAILED — individual quota exhausted (resets ~5 days) |
| opencode (glm-5.2) | opencode CLI | FAILED — local opencode.json invalid (unrecognized keys `plugins`, `config`) |

Two external seats ≥ 2 → real non-self peer validation retained (no sub-agent fallback needed).

## Peer validation

- Codex judged Claude (`judge-codex-on-claude.md`): 10/10 issues REAL; fixes 1, 5, 7, 10 corrected.
- Claude judged Codex (`judge-claude-on-codex.md`): 11/13 issues REAL; findings 2 and 8 ruled misreads
  (2: task 102's txn-scoped SQL-shape instruction misread as calling the pool-bound helper;
  8: mandatory non-empty description contradicts the deliberately nullable schema — SC1 is a
  live-acceptance observable, not a parser contract). Orchestrator independently verified both
  rejections against queries.rs:326-367 and the 101 DDL; upheld.
- Conflict adjudicated by orchestrator: claude#7 vs codex#2 (both about accept-transaction outbox
  atomicity). Ruling: keep 102's approach, sharpen it to explicitly PRE-AUTHORIZE the txn-scoped
  outbox INSERT with propagated errors as a documented divergence from Task::create's best-effort
  enqueue; NO refactor of task/queries.rs (blast-radius containment of the proven sync path).

## Scoreboard

| seat | validated issues | validated fixes | total |
|---|---|---|---|
| codex | 11 | 11 | 22 |
| claude | 10 | 6 | 16 |

## Remediations applied (all via envelope resubmit; render re-linted)

1. NEW task 204 — services-layer `start_breakdown_attempt` (prompt/run-reason injection via the
   public `start_execution` seam) + `should_finalize` Breakdown exclusion. Fixes the CRITICAL
   (301 was unimplementable: start_attempt hardcodes CodingAgent + task.to_prompt()) and the
   InReview/hive-push finalize leak. 203/301 re-dependent on it.
2. 203 — hook re-anchored AFTER the log-batcher flush/normalization (~:797-810) to kill the
   buffered-final-output race; find_by_execution_process_id hedge removed.
3. 202 — two-stage parser (stream-JSON result extraction per ResultMessage precedent, then fence
   scan) + stream-JSON fixture test; description stays Option (rejected finding recorded).
4. 102 — find_by_execution_process_id added (+ exact-lookup test); outbox assertion changed to
   entity-id-filtered (parent enqueue excluded); txn-scoped enqueue pre-authorization spelled out.
5. 301 — two-stage trigger (awaited draft insert, detached spawn); spawn-failure → proposal
   Failed (never a stranded 409-blocking draft) + route test.
6. 602 — deterministic test via the awaited stage-1 draft insert (no polling/status race).
7. 601 — files widened to the full verified Project-materialization inventory (stats/github/sync
   + two server files).
8. 603 — retargeted to frontend/src/pages/settings/ProjectSettings.tsx + settings.json namespace
   (all four locales).
9. 401 — real mock-HTTP proxy tests (method/path/body + error-envelope propagation) replace
   serde/registration-only tests.
10. 502/503 — TS4 reassigned to 503 (locale-parity + card + BOTH dropdown branches incl. mobile,
    TaskCard.breakdown.test.tsx added to files); 502 gains reorder/dependency-remap payload tests.
11. 101 — irreversible: true (ADR-0016 contract lands here; human gate reviews/101.approved).

## Termination

All peer-validated findings remediated; focused re-check: `wai-plan-lint.sh vk-swarm-task-breakdown`
PASS on the re-rendered tree (16 tasks, SC/TS coverage complete). Round closed — no further full
round (termination rule). Ready for /wai:execute.
