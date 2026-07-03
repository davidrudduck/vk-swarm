ADVERSARIAL TOURNAMENT — FIND + REMEDIATE (round 2: Phases 2–6). You are ONE competitor against 2 peers. Attack the BREAKDOWN (task files), NOT production code (none exists yet). For EACH finding propose a concrete, applicable fix. Scoring: +1 per REAL cited problem, +1 per correct fix — every finding is judged by a PEER; not-real → 0, hand-wavy fix → 0. Quality beats quantity; a padded nit LOSES points. Honest `FINDINGS: 0` beats a rejected nit.

CONTEXT: vk-swarm = Rust Cargo workspace + React. This is a hub-and-spoke hive redesign. Node = local SQLite; hive (`crates/remote`) = Postgres. WS protocol enums are hand-duplicated in `crates/services/src/services/hive_client.rs` AND `crates/remote/src/nodes/ws/message.rs`. Phases under review:
- **P2 (SC3) lease/atomic-checkout + fencing** — `docs/plans/vk-swarm-hive-redesign/phase-2/201-210-*.md`
- **P3 (SC4) status state machine** — `phase-3/301-304-*.md`
- **P4 (SC7) inbound collapse / one-delete / dirty-guard** — `phase-4/401-405-*.md`
- **P5 (SC5) anti-entropy digest** — `phase-5/501-504-*.md`
- **P6 (SC1) no-fanout guard** — `phase-6/601-602-*.md`

ALSO READ: `docs/plans/vk-swarm-hive-redesign/CONTRACT.md` (cross-phase interface — WS variants §A, fencing §C, status §D), `decisions-ledger.md` (TRAPS + ratified judgment calls), `dev-docs/adr/0007..0011`, `plan.md`. P1 (phase-1, already reviewed) is the dependency these build on.

**ANCHOR-VERIFICATION CAVEAT (important):** P2–P6 tasks build on P1 (op-log) and on each other, so many cited anchors (e.g. `handle_op_batch`, `node_outbox`, `OutboxOp.fencing_token`, the `OpBatch`/`OpAck` variants) do NOT exist on `main` yet — they are created by P1 tasks. Verify each anchor against **`main` OR the cited upstream task's Before/After text** (e.g. "106 creates handle_op_batch"). A "symbol absent on main" that an upstream task in the plan creates is NOT a finding. Only flag an anchor that neither `main` nor any plan task provides.

Attack axes (cite task id + repo file:line or the upstream task that should provide it):
1. Two concerns in one task / not bite-sized.
2. Wrong/non-existent anchor (per the caveat above).
3. Ambiguous instruction resolvable two ways.
4. `allowed_change` mismatch.
5. Dependency/conflict error or CYCLE — especially CROSS-PHASE: P2/P3/P5 all edit `session.rs` and the dual WS enums; do the depends_on/conflicts_with edges prevent two tasks racing the same enum tail or the same `handle_op_batch`/`session.rs` block? (205 edits 106's handle_op_batch; 303/304 also edit session.rs; 202/501 both add enum variants.)
6. Unmarked irreversible (deletes, destructive migrations).
7. Untestable or HOLLOW test — ESPECIALLY hive tasks (all P2 hive, P3 303/304, P5 503): do they enforce the live-Postgres precondition (the `test -n "$DATABASE_URL" &&` fail-closed gate), or skip-pass? Is P6/601 genuinely hermetic? Is the P2 SC3 "partition cannot double-execute" test (210) real or asserted-by-construction?
8. CONTROL-FLOW GROUNDING: open the real code. P2/205 fencing reject — is the lookup key (`shared_tasks.id` via `find_by_source_task_id`) correct? P4/402's unlink helper — does it actually fix the SQLite three-valued-logic no-op the ledger describes? P3's matrix — does it match the REAL `TaskStatus` enum {Todo,InProgress,InReview,Done,Cancelled} (no Assigned/Failed) per the ratified ADR-0010?
9. Fidelity: walk each SC to a task. SC3 fencing (at-most-once effect + bounded overlap) — delivered? SC5 self-heal without reset_* — delivered? SC7 one-delete-one-conflict — both legs identical? SC1 no-fanout — the 601 guard actually exhaustive?

Output one row per finding: `severity | task | file:line-or-upstream-task | issue | remediation`. Then `FINDINGS: <n>` and a one-line self-assessment.
