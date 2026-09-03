# Panel review — `proposal.md` (next workstream for the vk-swarm refactor)

Seat: read-only review panel. Repo: `main` @ `492e8e94`, worktree clean
(`git rev-parse --short HEAD` → `492e8e94`; `git status --porcelain` → empty).
Every claim below is checkable from this commit.

---

## Lens (a) — Fidelity

### Anchors that hold

I checked all 16 file:line anchors the proposal cites. **Fourteen are exactly right.**

| Proposal claim | Verified |
|---|---|
| `core.rs:486-511` — `start_attempt(...).is_ok()` folded into `has_in_progress_attempt`, handler returns `Ok(...)` regardless | ✅ `crates/server/src/routes/tasks/handlers/core.rs:486-495` (`.inspect_err(...).is_ok()`), `:502-511` (`ApiResponse::success`, `has_in_progress_attempt: is_attempt_running`) |
| `container.rs:714-748` — `try_stop` logs at `debug!` via `unwrap_or_else`, timeout "proceeding anyway" | ✅ `crates/services/src/services/container.rs:727-734` (`unwrap_or_else` → `tracing::debug!`), `:741-747` (`"try_stop timed out - proceeding anyway"`) |
| `msg_store.rs:56` send before `:67` history push | ✅ `crates/utils/src/msg_store.rs:56` `self.sender.send(msg.clone())`, `:59` lock acquired, `:67` `history.push_back` |
| `msg_store.rs:125-135` — `get_history()` then `get_receiver()` | ✅ `:128` `let (history, rx) = (self.get_history(), self.get_receiver());` — tuple evaluation is left-to-right, so the gap is real |
| local-deployment migrates before normalizer/batcher complete | ✅ `crates/local-deployment/src/container.rs:657` `migrate_execution_logs`, `:836-837` `log_batcher.finish`, `:850` `push_finished`, `:854-864` await handle. Ordering is exactly as claimed |
| No `starting/stopping/stop_failed/interrupted` states | ✅ `crates/db/src/models/execution_process/mod.rs:56-61` — `Running, Completed, Failed, Killed` |
| `server_instance_id` (20260118), no generation column | ✅ `crates/db/migrations/20260118184126_add_server_instance_id_to_execution_processes.sql`; `grep -rl "generation" crates/db/migrations/` → no matches |
| `claude.rs:172-185` `get_hooks_merged` | ✅ doc `:172-174`, `pub async fn get_hooks_merged` `:175`, `read_global_claude_hooks` `:185`; call site `:391`, `initialize(hooks)` `:422` |
| `main.rs:273-275` serve `Err` skips `perform_cleanup_actions` | ✅ `axum::serve(...).await?` at `:273-275`, `perform_cleanup_actions(&deployment)` at `:280` — `?` bypasses it |
| `queries.rs:1369` control test on main via #478 | ✅ `crates/db/src/models/execution_process/queries.rs:1369` `control_read_then_write_shape_reproduces_busy_snapshot` |
| `node_runner.rs:62` `from_env` reads ambient hive env | ✅ `crates/services/src/services/node_runner.rs:62-64` reads `VK_HIVE_URL`/`VK_NODE_API_KEY` |
| local-deployment `container.rs:320` orphan sweep | ✅ `async fn cleanup_orphaned_worktrees(db: &DBService)` at `:320`, invoked from `spawn_worktree_cleanup()` `:519`, which the real constructor calls at `:163` |
| M2/M3/M4-M6 children do not exist | ✅ `ls dev-docs/workstreams/` — no `local-execution-lifecycle-correctness`, no `opencode-attached-server` |
| M2 slug already named in roadmap §5 item 2 | ✅ roadmap `:217` |

Process/tracker facts also verified:

- `bash $WAI/scripts/wai-next.sh` → **exactly one** ready task, `hive-oauth-sw-bypass 301`. ✅ Task file frontmatter `status: ready` (`.../phase-3/301-full-repo-gates-...md:5`).
- `wai-followups.sh --open` → empty ✅. `wai-status.sh --check` → `OK: MASTER.md generated region matches frontmatter (byte-for-byte)` ✅.
- Backlog: `grep -c "^| F-" dev-docs/BACKLOG.md` → **48**; severity split 8 high / 20 medium / 20 low ✅; status split 27 fixed / 15 open / 2 promoted / 2 triaged / **2 wontfix**; the 8 high are 4 `fixed`, 2 `triaged`, 1 `promoted`, **1 `open` = F-2026-08-09-01** ✅ exactly as claimed.
- Ledger evidence: `dev-docs/workstreams/hive-oauth-sw-bypass/plans/hive-oauth-sw-bypass/decisions-ledger.md:287-296` — "Deploy verification round 2 — operator observations (2026-08-06)", verbatim `"sign in now works..."` ✅. `dev-docs/workstreams/node-task-delete-dangling-shared-id/plans/.../decisions-ledger.md:77` "Deploy verification (2026-08-28)", `:75` `VERDICT: PASS` ✅.
- `sqlite-busy-snapshot-calibration-stability/README.md:53` — sole unchecked box is "The branch containing this remediation is merged" ✅, and the control test **is** on main (above) ✅.
- `frontend-test-debt-2026-08/README.md:55` "## Outcome — COMPLETE (2026-08-01)", frontmatter `status: active` ✅.
- MASTER open questions `dev-docs/MASTER.md:150-153` — P3–P8 mapping + doctest/PTY downgrade, exactly as §5 item 8 says ✅.
- PR attributions: `#477` = event bus (`1f2caaea`), `#475` = task breakdown (`ac403b11`), `#467`/`#466`/`#468`/`#469` all as claimed ✅. `#478` = `e61d5f3d Require Hive OAuth before a LAN browser can use a local node` ✅.
- Task-count comparators: `find .../phase-*/ -name "[0-9]*.md" | wc -l` → node-ui-localize **19** ✅, design-system **24** ✅, hive-redesign **36** on disk but `dev-docs/MASTER.md:148` records **37** — the proposal's number matches the tracker, so **not** a finding.
- OpenCode: `opencode --version` → **1.18.26**; roadmap §8.2 certifies **1.18.19** ✅. `frontend/package.json` has no `test:e2e` and no Playwright — consistent with roadmap §9.5 "M3 adds the currently missing local-frontend Playwright setup".

### Fidelity findings

---

**[BLOCKING] F1 — §5 item 9 is factually wrong, and it recommends deleting a document that is still live.**

The proposal says: *"`dev-docs/2026-08-05-next-session-handoff.md` is fully consumed (all five WS-A..E shipped); mark superseded or delete."*

WS-E is not shipped:

```
dev-docs/2026-08-05-next-session-handoff.md:80  ### WS-E — test infrastructure (unblocks confidence in everything above)
dev-docs/2026-08-05-next-session-handoff.md:81  - `F-2026-08-04-02` — `test_fast_execution_no_lost_logs` flakes; ...
dev-docs/2026-08-05-next-session-handoff.md:83  - `F-2026-07-30-01` — `cargo test -p db` fails to compile ...
```

`F-2026-07-30-01` is `fixed` (`dev-docs/BACKLOG.md:24`), but **`F-2026-08-04-02` is `promoted` and unresolved** (`dev-docs/BACKLOG.md:42`) — it is the *same* lost-log race the proposal's own §3 nominates as M2's headline bundle item. The proposal contradicts itself: §3 says the bug is live and belongs in M2; §5 says the document tracking it is fully consumed and can be deleted.

Required revision: strike "all five WS-A..E shipped". State that WS-A–D are shipped and WS-E is half-consumed (F-2026-07-30-01 fixed, F-2026-08-04-02 carried into M2), and supersede the handoff *by pointing at M2*, not by asserting completion. Do not delete.

---

**[SHOULD-FIX] F2 — the backlog is stale in a way that invalidates §1's "15 findings are open" and is not in the §5 hygiene list.**

`F-2026-08-04-01` ("OAuthDialog polls /api/auth/status forever with no timeout or error branch") is recorded `open`, medium, at `dev-docs/BACKLOG.md:41`. It is **fixed on main**:

```
$ git log --oneline -1 -- frontend/src/components/dialogs/global/OAuthDialog.tsx
e3688d32 fix(hive): stop the service worker intercepting the OAuth redirect chain (F-2026-08-03-01/-02, F-2026-08-04-01) (#469)

frontend/src/components/dialogs/global/OAuthDialog.tsx:32   export const POLL_DEADLINE_MS = 120_000;
frontend/src/components/dialogs/global/OAuthDialog.tsx:94-104  deadline effect → setState({ type: 'error', message: tRef.current('oauth.timeoutError') })
```

The same commit names `F-2026-08-03-01`/`-02`, both still recorded `triaged` (`BACKLOG.md:39-40`) though their workstream is `shipped` and the ledger records a live-deploy PASS.

This matters because roadmap **M0 AC1** ("`/wai:next` lists only genuinely unfinished ready work") and **AC3** ("every first-release blocker has exactly one owner") are gate criteria, and because §9.1 makes release conditional on *"No actionable finding of any severity ... required to satisfy the supported journey"* — a finding that is fixed but recorded open cannot be reasoned about. §5's nine items are all workstream-README flips; none reconcile BACKLOG.md. Add a tenth item: reconcile finding statuses against main before `/wai:prd-new`.

---

**[SHOULD-FIX] F3 — the ADR-0001 anchor range under-covers the behaviour it is cited for.**

The proposal anchors auto-resume at `crates/services/src/services/container.rs:322-469`. That range covers the doc comment and the fence half only. The function is `:326-553`, and the actual automatic relaunch is at **`:468-530`**:

```
container.rs:468   if let Some(session_id) = session_id {
container.rs:498       let resume_prompt = "Your previous session was interrupted. Please continue the task from where you left off.".to_string();
container.rs:500-508   match self.resume_execution(&task_attempt, process, &stored_action, session_id, resume_prompt).await
container.rs:512       ExecutionProcess::set_resume_state(pool, process.id, "resumed")
```

The claim is correct; the citation would not let a reader find it. Re-anchor to `:468-530`.

---

**[INFO] F4 — §3's phase-0 row omits an existing partial mitigation, which the M2 spec will otherwise re-derive.**

`LocalDeployment::for_test` already disables the sweep, via an `unsafe` process-global env write that the code itself documents as unsound:

```
crates/local-deployment/src/lib.rs:555-565
  // SAFETY (partial, and stated honestly): ... It is NOT race-free against sibling tests:
  // `set_var` is unsound against any concurrent `getenv` ...
  unsafe { std::env::set_var("DISABLE_WORKTREE_ORPHAN_CLEANUP", "1"); }
crates/local-deployment/src/lib.rs:586-588
  // The reach is pre-existing (container.rs:169's `new_for_drain_test` already calls the same
  // constructor), but this test module makes it routine ...
```

This *strengthens* the phase-0 case (the existing mitigation is admitted-unsound and does not cover `new_for_drain_test`, `container.rs:169-193`), and the workstream README already names removing it as required outcome 3 (`local-deployment-test-orphan-cleanup-safety/README.md:50-51`). But the proposal's framing ("Today those constructors *can* sweep real clean worktrees") reads as if nothing exists, which will cost the spec author a rediscovery pass.

Related: the separate `worktree-orphan-sweep-guard` workstream is `shipped` and covered a **different** finding — F-2026-07-30-03, the missing *dirty-file* guard (`worktree-orphan-sweep-guard/README.md:22-29`). Say so, or the M2 spec will re-litigate a guard that already exists.

---

**[INFO] F5 — `staging_pointers` across the trackers point at `docs/plans/…`, which is empty.**

`ls docs/plans/` → only `.wai-reporoot` and `.wai-topic`. The ledgers actually live at `dev-docs/workstreams/<ws>/plans/<ws>/decisions-ledger.md`. The proposal's ledger claims are correct (verified above), but anyone following a README's `staging_pointers` — e.g. `normalize-fast-execution-lost-logs-flake/README.md:9` → `docs/plans/vk-swarm-event-bus/decisions-ledger.md` — finds nothing. Worth one line in §5.

---

## Lens (b) — Mechanics and judgment

### The ADR-0001 vs D8 tension: **CONFIRMED**, and understated

The proposal flags this as "the first `decisions[]` fork for `/wai:spec`". It is confirmed from the code and it is larger than a decisions entry.

Roadmap side — five independent statements of the same rule:
- §7.6.3 "launch no automatic replacement"; §7.6.5 "mark the execution interrupted"
- M2 AC12 "Accidental restart cannot create a second writer"
- D8 (`:774`) "Safe interruption without automatic continuation"
- §9.4.4 "No replacement starts automatically"; §9.4.5 "Exactly one interrupted execution remains"

Code side — `cleanup_orphan_executions` does the opposite today:
- `container.rs:468` — if a `session_id` exists, take the resume branch
- `container.rs:500-508` — call `resume_execution(...)` with a continuation prompt
- `container.rs:512` — `set_resume_state(..., "resumed")`
- fall-through to `mark_process_failed_with_task_update` only on resume *failure* (`:518-529`) or absent session (`:531-535`)

Governing document side — this is not incidental behaviour, it is an **accepted ADR**:
```
dev-docs/adr/0001-crash-recovery-fence-then-resume.md
  - **Status:** accepted   - **Date:** 2026-06-26
  Decision, step 2: "**Resume.** If the executor supports session resume and a `session_id` is
  persisted, reconstruct the `ExecutorAction` from the DB and re-enter `start_execution` ..."
```

So M2 cannot resolve this with a `decisions[]` entry alone — it must **supersede or scope-limit an accepted ADR**, which is a heavier artifact (new ADR, or a `Superseded-by` header on 0001). Two further points the proposal should carry into the spec:

1. The correct resolution is almost certainly *defer, not repeal*: roadmap §12 (M6) **is** ADR-0001's intent, and §12.1 explicitly says M6 "adds atomic recovery claims and automatic continuation" on M2's schema. So the M2 decision is "gate ADR-0001's resume branch behind an M6 capability flag, leave the fence half live", and ADR-0001 gets a status amendment reading *deferred to M6*, not *rejected*.
2. There is a live coupling to Q3 that the proposal does not see: when `executor_action` JSON is unparseable, the resume path **falls back to ClaudeCode** —
   ```
   container.rs:481-488  ExecutorAction::new(ExecutorActionType::CodingAgentInitialRequest(
                             CodingAgentInitialRequest { prompt: String::new(),
                               executor_profile_id: ExecutorProfileId::new(BaseCodingAgent::ClaudeCode) }))
   ```
   — so on a node with `~/.claude/settings.json` hooks (F-2026-08-09-01), an *attached-OpenCode* execution can be silently resumed into a Claude process that stalls pre-init. Gating the resume branch for M2 removes this too, which is a second reason to gate rather than re-scope D8.

**[SHOULD-FIX] F6** — reframe §2's last paragraph: this is an ADR supersession/deferral, not just a spec fork, and it should be recorded as a new ADR in the M2 workstream. Cite `container.rs:468-530` and `dev-docs/adr/0001-*.md` Decision step 2.

---

### Q1 — Is M2 the right next unit? **Yes, with one correction.**

The dependency graph (roadmap `:171-196`) is `M0 → (M1 ∥ M2) → M3 → FIRST RELEASE GATE`. M1 is shipped and live-verified (`local-node-browser-oauth/README.md:14-27`, TS7 dated 2026-08-26). M3 is defined as consuming M2's contract (roadmap `:222-223`, D4). Nothing else on `main` is on the release path: `/wai:next` is effectively empty, `wai-followups.sh --open` is empty, 0 open PRs. M2 is correct.

The one correction: **M0 is itself a release-gate item**, not "hygiene owed". Roadmap §9.1 first bullet: *"M0, M1, M2, and M3 are complete."* The proposal treats §5 as a preparatory docs PR and sequences it first (§6.1) — right action, wrong framing — and its nine items do not map onto M0's five acceptance criteria:

- M0 AC1 (`/wai:next` truth) — covered by §5 items 1–5 ✅
- M0 AC2 (umbrella contains dependency order and gate) — partly, §5 item 7
- M0 AC3 (every blocker has exactly one owner) — partly, §5 item 5; **not** for F-2026-08-19-01/-02 (see Q2) or the stale findings (F2)
- M0 AC4 (no M1–M3 workstream can close with live/E2E verification outstanding) — **not covered at all**; this is a stated rule that has to land somewhere durable (umbrella README or the child PRD template), and M1 already closed under it
- M0 AC5 (each child tracker points to exactly one canonical spec; no umbrella-wide decomposition) — **not covered**

**[SHOULD-FIX] F7** — restructure §5 as "M0 closure, mapped to AC1–AC5", and add the two uncovered criteria. Otherwise M0 will be declared done by a docs PR that never satisfied two of its five gates, and §9.1's first bullet becomes unverifiable at release.

Nothing smaller is release-gating. I checked the other candidates against §9.1's carve-outs and agree with every "leave" row in §4.

---

### Q2 — Test seams as M2 phase 0, or their own micro-workstream? **Phase 0 is right; the ownership transfer is missing.**

Phase 0 is the correct shape. Both fixes are constructor-level opt-outs in crates M2 already edits (`crates/local-deployment/src/{container.rs,lib.rs}`, `crates/services/src/services/node_runner.rs`), and M2's TS2 suite is exactly what makes the exposure routine — the code says so itself (`lib.rs:586-588`, quoted in F4). A separate micro-workstream would mean two PRs racing the same constructors, which roadmap §4 (`:198-200`) tells you to serialize anyway.

But the proposal is inconsistent about ownership. It correctly says `services-normalize-flaky-test` must be **closed as superseded** and F-2026-08-04-02 re-pointed at M2 (§5 item 5) — and then folds F-2026-08-19-01 into M2 phase 0 **without** doing the same for `local-deployment-test-orphan-cleanup-safety`, which is a live standalone workstream (`status: ready`, `dev-docs/MASTER.md` table) with four required outcomes of its own (`README.md:44-53`). That leaves the same finding with two owners, violating M0 AC3.

Note also that F-2026-08-19-02 has **no** workstream at all — `dev-docs/BACKLOG.md:51` workstream column is `—`, with the note "consider an explicit override/disable seam on `from_parts`". So phase 0 is simultaneously absorbing one workstream and adopting one orphan finding, and only the second is currently stated.

**[SHOULD-FIX] F8** — add a §5 item: close `local-deployment-test-orphan-cleanup-safety` as superseded-into-M2, carry its four required outcomes verbatim into M2's phase-0 tasks, and re-point both F-2026-08-19-01 and F-2026-08-19-02 at M2.

---

### Q3 — Should `claude-executor-hooks-hang` stay out of M2? **Yes. Agreed, and the rationale should be stronger.**

Agreed on the mechanics the proposal gives (different files — `crates/executors/src/executors/claude.rs`, anchor `:172-185` verified; self-contained; A/B repro recorded at `claude-executor-hooks-hang/README.md:20-27`). The decisive argument, which the proposal does not make, is the roadmap's own carve-out:

```
roadmap :458  "Direct OpenCode, Claude, and automatic continuation are identified as uncertified
               later milestones."
roadmap :619-627  §11.1 assigns hooks policy to M5.
```

So F-2026-08-09-01 — despite being the only open high — is **not release-gating**, because Claude is not in the first-release surface. That is the sentence §4 needs; "blocks every server-spawned Claude run" argues *urgency* and reads as if it might be gating.

One caveat that pulls the other way and should be recorded: the ADR-0001 fallback at `container.rs:481-488` can route a *non-Claude* execution into Claude on crash recovery. If M2 gates the resume branch (per F6), that path closes and the carve-out is clean. If M2 does not, F-2026-08-09-01 becomes reachable from an attached-OpenCode run and the §9.1 analysis changes. State the dependency.

---

### Q4 — Is M2 as scoped overloaded? **Yes at the top of the estimate, and there is a clean split line the proposal did not consider.**

The size estimate is defensible but its comparator set is wrong. The relevant sibling is **M1**, not `node-ui-localize`:

```
$ find dev-docs/workstreams/local-node-browser-oauth/plans/local-node-browser-oauth/phase-*/ -name "[0-9]*.md" | wc -l
22
```

M1 was 22 tasks across 4 phases for **13 acceptance criteria** confined to one boundary (auth). M2 has 13 acceptance criteria across **six sub-areas spanning five crates** (`db` migration, `services`, `local-deployment`, `server`, `utils`) plus a new permanent schema, plus phase 0, plus a frontend surface (below). "20–28" is the floor, not the range.

**The clean split line that does not create two identity models is §7.5.** Roadmap D5/SC7 forbid a second lifecycle identity — but §7.5 (Output durability: MsgStore atomicity, acknowledged batcher shutdown, await raw+normalized persistence, migration ordering, resumable/idempotent migration) **introduces no identity at all**. It is pure plumbing in `crates/utils` + `crates/local-deployment`, entirely disjoint from the generation schema that §7.1/§7.6 introduce. Splitting it out is therefore SC7-safe by construction.

There is a hard operational reason to land it *first*, which the proposal's own §3 evidence supplies without drawing the conclusion: the lost-log race makes `cargo test -p services` fail at roughly **1-in-6 to 1-in-8** (`normalize-fast-execution-lost-logs-flake/README.md:130-132, 143-148`), and it is confirmed to fire on an idle machine (`:118-125`, 0.49s). M2's own gate for every one of its ~25 tasks runs that crate. Bundling the fix into the middle of M2 means every M2 Stage-1 gate before it is a coin flip; M1's own acceptance evidence already had to explain this away (`local-node-browser-oauth/README.md:22`: *"`cargo test --workspace` — EXIT 101 once: only ... test_fast_execution_no_lost_logs"*).

**Recommended shape (not a workstream split — SC8 wants one spec per milestone):**

- **phase 0** — test seams (F-2026-08-19-01/-02). Makes the M2 suite constructible.
- **phase 1** — §7.5 output durability incl. the lost-log race, landing as its own PR. Turns the `services` gate deterministic before any identity work starts.
- **phases 2+** — §7.1 launch state + generation schema, §7.2 create/start, §7.3 cancellation, §7.4 retry, §7.6 restart policy + the ADR-0001 gating decision.

That keeps one spec, one identity model, and removes the flake from the critical path of the other ~20 tasks.

**[SHOULD-FIX] F9** — adopt the phase ordering above and replace the comparator set with M1 = 22 tasks; state the estimate as ~25–32 including phase 0 and the frontend surface.

---

### Anything release-gating missed?

**[SHOULD-FIX] F10 — M2 has an unacknowledged frontend + i18n surface.** §3's file lists are Rust-only, but roadmap §7.1/§7.3 add four states and §7.2 requires *"show the failure and retry action in the UI"* (M2 AC1: *"Failed Create & Start is visible"*). `ExecutionProcessStatus` is a generated exhaustive TS enum:

```
shared/types.ts:885  export enum ExecutionProcessStatus { running = "running", completed = "completed", failed = "failed", killed = "killed" }
```

consumed at 20 sites across 4 frontend files, including 12 in `frontend/src/hooks/useConversationHistory.ts` (`:308, 448, 450-451, 518, 523-524, 540, 581, 752, 788, 1011`) and the status icon/colour switches in `frontend/src/components/tasks/TaskDetails/ProcessesTab.tsx:53, 68`. Adding `starting/stopping/stop_failed/interrupted` forces `npm run generate-types`, four-locale i18n strings (CLAUDE.md §2: en, ja, ko, es), and a review of every one of those 20 sites — several of which key "is it live?" off `=== running` and will silently mis-handle `starting`. Add `frontend/` and `shared/types.ts` to §3's file column and budget tasks for it.

**[SHOULD-FIX] F11 — the M2 generation identity may collide with an already-shipped fencing model, and §3 does not mention it.** Roadmap D5/SC7 say M2 introduces "the permanent execution-generation and ownership identity" and that later milestones need no replacement schema. A monotonic per-assignment fencing token already exists in the shipped hive schema under an accepted ADR:

```
dev-docs/adr/0009-lease-checkout-fencing.md   Status: accepted, Discharges: ADR-0001 D7
crates/remote/migrations/20260128000001_add_lease_fencing.sql:7   ADD COLUMN fencing_token BIGINT NOT NULL DEFAULT 0;
crates/remote/migrations/20260128000001_add_lease_fencing.sql:12  CREATE SEQUENCE node_fencing_token_seq ...
crates/db/migrations/20260201000400_add_node_outbox.sql:19        fencing_token INTEGER,  -- NULL in the tracer; populated by phase-2 fencing
```

M2's execution-generation is node-local and per-execution; ADR-0009's token is hive-assigned and per-task. They are different things — but SC7 and D5 exist precisely to prevent two overlapping ownership models, and the node outbox already carries a `fencing_token` column awaiting population. The M2 spec must state the relationship (and pick distinct names) before the migration lands, or M6 inherits two "generation" concepts. This is a `decisions[]` fork the proposal has not identified.

**[INFO] F12** — M0 AC4 interacts with the release gate: §9.1 requires *no* deferred M1–M3 criterion, and M1 closed with `F-2026-08-04-02` explicitly excused in its acceptance evidence (`local-node-browser-oauth/README.md:22`). M2 inherits and discharges that; worth stating in M2's PRD so the release-time §9.7 review can trace it.

---

## Verdict

The proposal reads the repo accurately — 14/16 anchors exact, every tracker and script claim reproduced, the strategic call (M2 next) correct and well-evidenced. The ADR-0001 vs D8 tension is real and is the most valuable thing in the document. Three things must change before it drives `/wai:prd-new`: one factually wrong recommendation that would delete a live tracking document (F1), an ownership gap that leaves two findings double- or un-owned against M0 AC3 (F8), and two unscoped surfaces (frontend enum fan-out F10, fencing-token collision F11) that would otherwise surface mid-execution as "discovered in-flight" scope.

**Required revisions**

1. **[BLOCKING] F1** — strike "all five WS-A..E shipped" from §5 item 9; WS-E's `F-2026-08-04-02` is `promoted`/unresolved (`BACKLOG.md:42`) and is M2's own headline item. Supersede the handoff by pointing at M2; do not delete.
2. **[SHOULD-FIX] F2** — add BACKLOG reconciliation to §5 (F-2026-08-04-01 fixed by `e3688d32` but recorded `open`; F-2026-08-03-01/-02 `triaged` post-PASS).
3. **[SHOULD-FIX] F3** — re-anchor ADR-0001 auto-resume to `container.rs:468-530`.
4. **[SHOULD-FIX] F6** — reframe §2's tension as an ADR-0001 supersession/deferral-to-M6, with a new ADR as the artifact; note the ClaudeCode fallback at `container.rs:481-488`.
5. **[SHOULD-FIX] F7** — restructure §5 against M0 AC1–AC5; AC4 and AC5 are currently uncovered.
6. **[SHOULD-FIX] F8** — close `local-deployment-test-orphan-cleanup-safety` as superseded-into-M2 and re-point F-2026-08-19-01/-02.
7. **[SHOULD-FIX] F9** — adopt phase 0 → §7.5 → identity ordering; recomparator against M1 (22 tasks); revise estimate to ~25–32.
8. **[SHOULD-FIX] F10** — add `frontend/` + `shared/types.ts` + i18n to M2's scope.
9. **[SHOULD-FIX] F11** — add a `decisions[]` fork for execution-generation vs ADR-0009 `fencing_token`.
10. **[INFO] F4, F5, F12** — fold in as spec-authoring notes.

VERDICT: REVISE
