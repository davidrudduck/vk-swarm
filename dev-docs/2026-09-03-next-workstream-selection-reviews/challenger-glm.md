# Challenger report — GLM (adversarial review of proposal.md)

Challenger: GLM (opencode seat). Read-only; repo walked at `main @ 492e8e94` in
`cosmic-aardvark`. Every claim below was re-derived from source on 2026-09-03.

---

## A. Anchor attack — every file:line in the proposal, re-verified

| Proposal anchor | Result | Evidence |
|---|---|---|
| `core.rs:486-511` false success on start failure | **HOLDS** | `start_attempt(...).inspect_err(...).is_ok()` at `crates/server/src/routes/tasks/handlers/core.rs:488-495`, folded into `has_in_progress_attempt` at :504; unconditional `Ok(ApiResponse::success(...))` at :502 |
| `services/container.rs:714-748` try_stop suppresses failures | **HOLDS** | `try_stop` at `crates/services/src/services/container.rs:713-748`; `unwrap_or_else` → `tracing::debug!` at :725-732; timeout warn "proceeding anyway" at :742-748 |
| `msg_store.rs:55-71` / `:125-135` race | **HOLDS** | `sender.send` at `crates/utils/src/msg_store.rs:56`, history `push_back` at :67; `history_plus_stream` takes `(get_history(), get_receiver())` at :128 — a push between the two is in neither |
| `local-deployment/container.rs:657` migrates before batcher/normalizer finish | **HOLDS** | `migrate_execution_logs` at :657 precedes `log_batcher.finish` at :837, `push_finished` at :848, normalization await at :856 — same completion flow |
| `execution_process/mod.rs:56-61` only `Running/Completed/Failed/Killed` | **HOLDS** | enum at `crates/db/src/models/execution_process/mod.rs:55-61` |
| migrations: `server_instance_id` 20260118, no generation column | **HOLDS** | `crates/db/migrations/20260118184126_add_server_instance_id_to_execution_processes.sql`; `grep -r generation crates/db/migrations/` → empty |
| `services/container.rs:322-469` ADR-0001 auto-resume vs D8 | **HOLDS** | `cleanup_orphan_executions` at :326; "Step 2: Classify — resume if session_id exists" at :449-470; resume test at :2209; startup call at `crates/server/src/main.rs:131` |
| `local-deployment/container.rs:320` sweep from constructors | **HOLDS** | `cleanup_orphaned_worktrees` fn at :320; `spawn_worktree_cleanup` :514; called from construction path at :163 |
| `node_runner.rs:62` ambient hive env | **HOLDS** | `NodeRunnerConfig::from_env` at `crates/services/src/services/node_runner.rs:62`; reached from `LocalDeployment::from_parts` at `crates/local-deployment/src/lib.rs:273-274` |
| `main.rs:273-275` serve Err skips cleanup | **HOLDS** | `axum::serve(...).await?` at `crates/server/src/main.rs:273-275`; `perform_cleanup_actions` only at :281 |
| `claude.rs:172-185` get_hooks_merged | **HOLDS** | `crates/executors/src/executors/claude.rs:174-185` |
| `queries.rs:1369` busy-snapshot control test | **HOLDS** | `control_read_then_write_shape_reproduces_busy_snapshot` at `crates/db/src/models/execution_process/queries.rs:1368` |
| host `opencode --version` = 1.18.26 | **HOLDS** | ran it: `1.18.26` |
| `/wai:next` = exactly one stale-ready task 301 | **HOLDS** | ran `wai-next.sh`: `hive-oauth-sw-bypass 301 … 1 ready task(s)`; task file `…/phase-3/301-*.md:5 status: ready` |
| backlog 48 findings / 8 high / 15 open | **HOLDS** | `awk` over `dev-docs/BACKLOG.md` → `total=48 high=8 open=15` |
| roadmap §5 item 2 names the M2 slug | **HOLDS** | M0 scope list, `2026-08-21-local-node-first-use-roadmap-design.md:212` |
| M1 README "Acceptance evidence (TS7)" | **HOLDS** | `dev-docs/workstreams/local-node-browser-oauth/README.md:14` |
| lost-logs "0.49–0.52s, zero patches" | **HOLDS** | README captures `finished in 0.52s` and `0.49s` with `got 0` |

**No wrong anchors found.** Two unverifiable-as-stated items (not wrong, just uncited):

- [INFO] A-1. Comparator task counts (`node-ui-localize` 19, `design-system` 24, `hive-redesign` 37) are not verifiable from the repo — graduated task files are pruned (`find dev-docs/workstreams/vk-swarm-node-ui-localize -name '*.md' -path '*task*'` → 0). Cite the source (executor journal?) or drop; the argument survives without them (see B).

## B. Overload attack — honest task count for M2-as-scoped

Walking §7.1–7.6 against code:

| Sub-area | Tasks (incl. tests) | Drivers found in code |
|---|---|---|
| §7.1 states + generation schema | 5–6 | new enum variants → migration + sqlx enum + `frontend/src/lib/types.ts:4` shared TS enum + `ProcessesTab.tsx:55-76` status switches; generation/ownership columns; no-running-before-durable-identity ordering; `SpawnContext` |
| §7.2 create/start | 3–4 | handler rework + structured `start_failed` + attempt terminal classification + UI failure/retry. Note: the retry routes already handle errors correctly (`task_attempts/handlers/core.rs:529+`, `:678+` log, delete broken attempt, return Err) — the bug is specific to `tasks/handlers/core.rs`; but §7.4 then *reverses* the delete-on-failure behavior (`:538+` `TaskAttempt::delete`), so the same files get touched twice with opposite semantics |
| §7.3 cancellation | 2–3 | `stop_execution` + 6 caller groups (`execution_processes.rs:210`, `git_ops.rs:114`, `task_attempts/core.rs:902`, `services/container.rs:725`, `assignment_handler.rs:222`); `stopping/stop_failed` transitions + UI |
| §7.4 retry | 2 | stop-previous + history/draft preservation vs current `TaskAttempt::delete` |
| §7.5 durability | 5–7 | atomic `history_plus_stream` (internal to MsgStore — all 6 callers unaffected, feasible); ack'd batcher shutdown; migration reorder (:657 vs :837/:848); resumable+idempotent migration (also used by CLI `server/src/bin/migrate_logs.rs:103`); final/trailing-before-announce (webhooks at `local-deployment/container.rs:667+` fire before batcher finish at :837) |
| §7.6 restart policy | 4 | reconciliation-before-mutations barrier; gate ADR-0001 resume (main.rs:131–137); interrupted marking; explicit retry |
| TS2 fault-injection harness | 2–3 | roadmap :785 enumerates 8 injected failure classes |
| UI states + i18n | 1–2 | new statuses are user-visible per §7.2; frontend is localized → label keys |
| phase 0 seam (if kept) | 2–3 | README "Required outcomes" 1–4 + node_runner guard |
| **Total** | **26–34** (+3 if seam kept in) | |

- [SHOULD-FIX] B-1. The proposal's "roughly 20–28" is ~20% under the honest floor. The omissions are (a) frontend status work (shared TS enum ripple) and (b) the TS2 harness breadth. Size drives `/wai:decompose` phasing; state 26–34 or "high-20s to mid-30s".

**Is it one workstream or three?** One — and the roadmap *mandates* it, which the proposal never cites:

- `roadmap:86` "Each milestone is a separate WAI child workstream" and M0 AC5 (`:238-239`) "no umbrella-wide implementation plan attempts to decompose M1–M6 as one unit" — M2 must be a single child.
- §9.1 (`:458`) "No M1–M3 acceptance criterion is deferred" — a CLAUDE.md-style scope split of M2's own ACs across two trackers would defer half of them, violating the first-release gate.
- Any split along §7.1 (schema) vs §7.5 (durability) would strand AC-13 (`:345`: M3–M6 claim the *same* schema M2 introduces) in the first half only, and the second half would consume a schema it doesn't own — precisely the two-identity-model drift D5/SC7 (`:52`, `:768`) forbid. The only safe seam is *phases inside one spec*, which is what the proposal's sequence already does.

Verdict on Q4: not overloaded at the workstream level; overloaded only if the estimate stays at 20–28 and phase 0 keeps both seams (see D).

## C. Wrong-target attack — anything more release-gating than M2?

Checked: all 15 open backlog rows (`dev-docs/BACKLOG.md`), every README with status draft/ready/active (16 of them — `grep -n "^status:" dev-docs/workstreams/*/README.md`), the 2026-08-05 handoff, roadmap §9 (`:450-572`).

- **Hooks hang F-2026-08-09-01 (high, open) does NOT gate first release.** Roadmap `:78`: "The first release does not certify … Claude CLI"; gate = M0–M3 complete (`:456-457`, SC8 `:54`); Claude is M5, downstream of the gate (`:193-197`). Proposal's separate-parallel-bugfix handling is correct.
- **Nothing else open is gating.** Remaining opens are low/medium (WAL error code, write gate, tailwind tokens, Node 26 pin, dead code) or test debt. §9.3 execution items 1–9 are M2's own ACs; items 10–11 are M3's. §9.5's missing frontend Playwright is explicitly M3's to add (`:520` "M3 adds the currently missing local-frontend Playwright setup").
- **M0 itself is unfinished** (tracker hygiene) — the proposal sequences it first (§6.1). Correct.

Two target-adjacent misses, neither reordering the queue:

- [SHOULD-FIX] C-1. **F-2026-08-04-01 (OAuthDialog endless spinner, medium, open) is stale-fixed and orphaned.** Backlog row (`BACKLOG.md:41`) still points at workstream `hive-oauth-sw-bypass` — which is `status: shipped`. The code now has a poll deadline + timeout error + tryAgain (`frontend/src/components/dialogs/global/OAuthDialog.tsx:101` `oauth.timeoutError`, error branch :287-293). Roadmap M0 AC3 (`:235`) demands every first-release blocker have exactly one owner; a shipped workstream owning an open (apparently fixed) auth finding fails that. Add verify-and-close to the §5 hygiene PR.
- [SHOULD-FIX] C-2. **§5 item 9 is factually wrong: "all five WS-A..E shipped."** WS-E in the handoff (`2026-08-05-next-session-handoff.md:80-83`) is the lost-logs flake F-2026-08-04-02 **plus** F-2026-07-30-01. F-2026-07-30-01 is `fixed` (`BACKLOG.md:24`), but F-2026-08-04-02 is the lost-log race — open, and the proposal itself routes it into M2 (§3 row 1). The handoff is *not* fully consumed; mark it superseded **with an explicit re-point of F-2026-08-04-02 to M2**, not a bare delete, or the provenance chain (who owns the race) breaks exactly the way M0 AC3 forbids.

## D. Bundle-coherence attack — F-2026-08-19-01/-02 as M2 phase 0

The rule (handoff `:26-30`): "One WAI workstream = one spec = one reachability gate, and that gate is per-incident… Three unrelated bugs under one spec makes that gate meaningless."

**For bundling:** Both findings sit on the single seam M2's TS2 suite must hammer: `LocalDeployment::from_parts` → `LocalContainerService` constructor spawns the sweep (`container.rs:163`→`:514`→`:320`) and `from_parts` calls `NodeRunnerConfig::from_env()` (`lib.rs:273-274`). They are not ride-alongs; without them M2 cannot write its deterministic suite on a developer machine without either sweeping real worktrees or dialing the live hive. The roadmap forces M2 to be one child (B above), so enabling tasks for its own test strategy arguably belong inside it. The lost-logs race is different in kind: it *is* §7.5 bullet 1 verbatim (`:312` "Make MsgStore history-plus-live subscription atomic") — one spec, one gate, no strain. Same for closing `services-normalize-flaky-test`: M0 explicitly orders "consolidate contradictory normalizer trackers under the confirmed product defect" (`:228`), and its H1 was disproved 2026-08-14 by the 0.52s datum (README).

**Against bundling:** The caveat is per-incident, not per-convenience. `local-deployment-test-orphan-cleanup-safety` already exists as `status: ready` with four enumerated required outcomes — a perfectly coherent standalone gate ("no test constructor can sweep a real worktree or reach the live hive"). F-2026-08-19-01 and -02 are one incident class (ambient production state reachable from test constructors of `from_parts`), and M2's PRD/spec cycle is docs work that proceeds in parallel — executing the micro-workstream first costs zero schedule. Folding it in also forces M2's reachability gate to span two unrelated paths (lifecycle correctness + test-constructor isolation), which is the exact dilution the caveat prohibits.

**Pick: split it out.** Amend the ready README's scope with one line absorbing F-2026-08-19-02 (same seam, same gate), execute it as the first implementation unit after the hygiene PR, and let M2's spec cite it as a satisfied prerequisite. This keeps every gate per-incident at zero cost. If the panel prefers bundling, it is survivable only because the seam findings are strict prerequisites of TS2 — but then the M2 spec must name them in its own success criteria (the proposal's §6.2 already does: "success criteria = … + test-seam safety"), which the proposal got right. Grade: [SHOULD-FIX] D-1, not blocking — both readings are defensible; the split is cleaner under the cited rule.

F-2026-09-02-01 ride-along (low, `main.rs:273-275`): correctly conditioned on file overlap; §7.5's "acknowledged batcher shutdown" work lives in `local-deployment/container.rs` + `log_batcher`, not `main.rs` — leaving it out is right. [INFO] D-2.

## E. Hidden-dependency attack

Greps run: `start_attempt` (20 hit groups), `stop_execution` (10), `cleanup_orphan_executions` (4), `migrate_execution_logs` (7), `history_plus_stream`/`stream_live_only` (7).

- **No M3 dependency.** M3 consumes M2's contract (`roadmap:202-203`); today's executors already persist `session_id` (resume path uses it, `services/container.rs:451-470`). M2's schema must be M3-*shaped* (AC-13) but nothing in §7 requires the attached server. Confirmed independent.
- **No P4 event-bus dependency in the completion path.** `grep -n "event_bus\|EventBus\|publish" crates/local-deployment/src/container.rs` → only `SharePublisher`; "completion is announced" = `push_finished` (:848) + webhooks (:667+) + publisher. The normalizer/tailer machinery the event-bus workstream shipped is in the pipeline M2 reorders — same crates, no blocker.
- [SHOULD-FIX] E-1. **Startup sequencing is a hidden §7.6 dependency.** `main.rs:131-140` sequences `cleanup_orphan_executions` → queued-message drain, with an explicit comment that the drain ordering exists to avoid racing "304's resume path". Gating ADR-0001's resume (required by D8/§7.6/`:421` "Do not resume automatically"/SC6 `:50`) changes that sequencing contract. The M2 spec's reconciliation design must own this ordering, and the shipped resume tests (`services/container.rs:2209`, `:2292`) will invert — the PRD must schedule their replacement, not discover them mid-execute.
- [INFO] E-2. `migrate_execution_logs` has three surfaces: completion path (`local-deployment/container.rs:657`), on-demand (`:2136`), and the `migrate_logs` CLI binary (`server/src/bin/migrate_logs.rs:103`). The resumable/idempotent requirement (AC-9) must hold across all three.
- [SHOULD-FIX] E-3. **Second loss mechanism the atomic-subscribe fix does not cover:** `history_plus_stream`'s live leg silently drops broadcast `Lagged` errors (`msg_store.rs:132` `.filter_map(|res| res.ok())`), capacity 10000 (`:44`). AC-6's "loses and duplicates zero messages" under stress will trip Lagged skips even after the snapshot/receiver gap is closed. The spec must decide Lagged handling (or explicitly bound the stress claim) — otherwise AC-6 is unfalsifiable-failing.
- [INFO] E-4. `start_attempt`'s honest caller surface for §7.2/§7.4 sizing: create (`tasks/handlers/core.rs:488`), retry ×2 (`task_attempts/handlers/core.rs:529`, `:678`), shared/remote (`tasks/handlers/remote.rs:66`, `:346`), node dispatch (`assignment_handler.rs:143`), publisher (`share/publisher.rs:103`). Retry routes already propagate errors; create does not — §7.2 is genuinely local to create, §7.4 genuinely rewrites retry. No surprise callers.

## F. D8 vs ADR-0001 — proposal's flagged tension, corroborated and upgraded

The proposal calls it "the first decisions[] fork." Correct, and stronger than stated: the auto-resume it must gate is *shipped, tested, and startup-sequenced* (`main.rs:131`; tests at `services/container.rs:2209`; drain dependency at `main.rs:137`). The roadmap nowhere acknowledges the existing fence-then-resume implementation (grep for resume/fence in the roadmap yields only future-tense M6/policy text; `:421`, `:644`, SC6 `:50`). Reversing it is an irreversible, live-verified-behavior change → per `/wai:precheck` it needs an **ADR in the M2 spec**, not just a `decisions[]` entry. [SHOULD-FIX] F-1: name it as ADR-required in §6.3, and note M6 will re-enable the machinery (`:680` M6 "Continue the durable launch generation") — gate, don't delete.

---

## Findings index

| ID | Tag | Summary |
|---|---|---|
| B-1 | SHOULD-FIX | Task estimate 20–28 understates honest 26–34; add frontend TS-enum ripple + TS2 harness breadth |
| C-1 | SHOULD-FIX | F-2026-08-04-01 stale-open, owned by a shipped workstream; add verify-and-close to §5 hygiene |
| C-2 | SHOULD-FIX | §5 item 9 "all five WS-A..E shipped" is false — WS-E's F-2026-08-04-02 is the open lost-log race; re-point, don't delete |
| D-1 | SHOULD-FIX | Split F-2026-08-19-01/-02 into the already-ready micro-workstream (one amended scope line) instead of M2 phase 0; per-incident gate rule |
| E-1 | SHOULD-FIX | §7.6 must own main.rs:131-140 startup sequencing + inversion of shipped resume tests |
| E-3 | SHOULD-FIX | broadcast Lagged silently dropped (msg_store.rs:132) — AC-6 unfalsifiable without a decision |
| F-1 | SHOULD-FIX | D8/ADR-0001 conflict requires an ADR (precheck rule), gate-not-delete for M6 reuse |
| A-1 | INFO | Comparator task counts unverifiable (graduated plans pruned); cite or drop |
| D-2 | INFO | F-2026-09-02-01 correctly left out (§7.5 work doesn't touch main.rs) |
| E-2 | INFO | migration idempotency spans 3 surfaces incl. CLI binary |
| E-4 | INFO | start_attempt caller map confirms §7.2/§7.4 scoping |

No [BLOCKING] findings: the thesis (M2 is the sole unstarted critical-path item; gate = M0–M3; nothing open outranks it), every anchor, the lost-logs bundling, the supersession of `services-normalize-flaky-test`, and the hooks-hang separation all survived attack.

## Required revisions

1. Fix §5 item 9 (C-2): handoff is superseded-with-one-repoint (F-2026-08-04-02 → M2), not "fully consumed".
2. Add F-2026-08-04-01 verify-and-close to the §5 hygiene list (C-1).
3. Re-estimate M2 at 26–34 tasks and enumerate the full caller surface (B-1, E-4) in the PRD seed.
4. Move F-2026-08-19-01/-02 out of "M2 phase 0" into the ready `local-deployment-test-orphan-cleanup-safety` micro-workstream executed immediately before M2 (D-1) — or, if the panel overrules, keep phase 0 but state in the PRD that M2's own success criteria own the seam outcomes (as §6.2 already gestures).
5. In §6.3, elevate the D8/ADR-0001 fork to an ADR-required decision covering startup sequencing and the shipped resume tests (E-1, F-1); add the Lagged-handling decision to the spec's open questions (E-3).

VERDICT: REVISE
