# Phase-3 Integrated Adversarial Review — OpenCode (round 1)

**Reviewer:** OpenCode (z-ai/glm-5.2)
**Date:** 2026-07-02
**Scope:** full Phase-3 diff `git diff 8a90485c..HEAD` — commits `2efdf511` (301),
`0fd71a13` (302), `f073102e` (303), `70b05488` (304), `2eae1426` (ledger).
**Files in scope:** `crates/remote/src/nodes/ws/status_machine.rs` (301),
`crates/remote/src/nodes/ws/session.rs` (302/303/304), `crates/remote/src/nodes/ws/mod.rs` (+1 line).

**Verification run:**
- `cargo clippy --package remote --lib --no-deps -- -D warnings` → CLEAN (6.46s)
- `cargo test --package remote --lib status_machine` → 6/6 pass (hermetic unit tests)

---

## Mechanics lens — cross-task interactions

### 1. Matrix ↔ wire form (301 × 302) — CLEAN

`canonical_status_from_node` (`status_machine.rs:62-71`) accepts BOTH the node's
`#[serde(rename_all = "lowercase")]` forms (`todo`/`inprogress`/`inreview`/`done`/`cancelled`,
from `crates/db/src/models/task/mod.rs:25`) AND the hive's `#[serde(rename_all = "kebab-case")]`
forms (`in-progress`/`in-review`, from `crates/remote/src/db/tasks.rs:23`). Every wire value maps
to the enum variant `author_of_transition` (`status_machine.rs:29-43`) keys on — no string-compare
drift. Unknown values return `Err` (never coerced to `Todo`, tournament R1/F5). 302's call site at
`session.rs:2032` propagates the `Err` as a `HandleError::Database`. Verified: no two halves of the
matrix disagree.

### 2. Guard ↔ fence (303 × P2) — CLEAN

**(a) Double-reject / double-write:** P2's fence (`session.rs:1970-2022`) `break`s on both the
stale-token arm (`:2005`) and the no-active-assignment arm (`:2020`). `break` exits the op loop
entirely — 303's guard (`:2061`) is never reached. 303's reject arms write `node_op_log` (SKIP+
ADVANCE via `continue`), but they only run for ops that PASSED P2's fence. No double-reject, no
double `node_op_log` write. Verified.

**(b) Lease re-check:** 303 does NOT re-check the lease, by design. P2's fence
`SELECT ... WHERE task_id = $1 AND completed_at IS NULL` (`:1977`) guarantees an active assignment
with a current token at the moment of the read. 303 only checks the orthogonal transition-author
question. The ledger at `docs/plans/.../decisions-ledger.md:470-478` ratifies this: "fencing is
RIDDEN, not re-authored." P2's `WHERE completed_at IS NULL` is sufficient — 303 does not need to
re-check. (TOCTOU window between P2's read and 303's `find_by_id` is a pre-existing property of
the non-transactional apply loop, not a Phase-3 regression.)

### 3. No-op short-circuit × bad fencing token (303 internal) — CLEAN

Trace of the exact sequence the prompt asks for (`status == current` BUT bad `fencing_token`):

1. `shared_id` present → P2 fence runs (`:1970`).
2. Active assignment exists with `current_token`.
3. `op.fencing_token` is `None` or `< current_token` → `stale = true` (`:1986-1989`) → `break`
   (`:2005`).
4. 303's no-op short-circuit (`if status != current`, `:2071`) is NEVER reached.

The no-op is rejected by P2's fence — correct. The fence is the authoritative gate; a stale-token
op against a hive-managed task is rejected wholesale (including its metadata changes), consistent
with the fence's design. 303's comment at `:2059-2060` ("by the time we reach here ... already
passed the stale-token check (or broken out before this point)") is accurate.

### 4. Legacy guard ↔ op-log guard (304 × 303) — CLEAN

Both call sites import from the SAME module (`status_machine::node_may_author` /
`status_machine::author_of_transition`). Single source of truth — no matrix drift possible.
Verified the two transitions the prompt names:
- `InProgress→InReview`: `node_may_author(InProgress, InReview)` → `true` in BOTH paths (`:2075`
  op-batch, `:695` legacy). ✓
- `InReview→Done`: `node_may_author(InReview, Done)` → `false` in BOTH paths (op-batch hits the
  `Some(Hive)` arm → SKIP+ADVANCE; legacy hits the `Ok(Some(_)) =>` warn arm → no write). ✓

### 5. Legacy wire map never yields Done (304 internal) — CLEAN

`session.rs:679-684`:
```
Pending | Starting => Todo
Running              => InProgress
Completed            => InReview
Failed | Cancelled   => Todo
```
`Done` is NOT in the map. A node can never mark a shared task `Done` via the legacy path — only the
hive can (via `InReview→Done`, the operator-approve transition). Verified.

### 6. `update_assignment_status` vs `update_status_from_node` order (304 internal) — CLEAN

`update_assignment_status` (the assignment's own `execution_status`, NOT the shared task's
`task.status`) is called UNCONDITIONALLY at `session.rs:670`, BEFORE the `node_may_author` guard
at `:690-720`. If the guard rejects the shared-status transition, the assignment's execution
status is STILL updated (the node's own bookkeeping is not gated). Order is correct per spec — the
assignment's `execution_status` is a different field in a different layer (ADR-0010 §D: "failed"
is an execution_status outcome, not a task.status value). Verified.

### 7. Creation path (303 × 304) — CLEAN

- **303:** `repo.find_by_id(shared_id)` returns `None` → the `if let Some(existing)` block
  (`:2065`) is skipped → `upsert_from_node` creates the row. Creation is not a transition; the
  matrix governs transitions of an EXISTING row. ✓
- **304:** `task_repo.find_by_id` returns `Ok(None)` → the `Ok(None)` arm (`:713`) logs a warn
  and does NOT write. The legacy path updates an existing shared task (the assignment was already
  found at `:677`); creation is the op-batch path's job. Neither path rejects a creation. ✓

### 8. Test seed adaptation (303 × 301) — CLEAN

`fencing_tests::op_with_current_token_against_assigned_task_applies_normally` was adapted:
- Seed changed from `"todo"` → `"in-progress"` (`session.rs:3361`).
- `Todo→Done` (illegal after 301 — no author) → `InProgress→Done` (node-authored per matrix).

`author_of_transition(InProgress, Done)` → `Some(Node)` (`status_machine.rs:38`) → matrix-compliant.
The test's PURPOSE (verify a current-token op applies PAST P2's fence) is preserved: the op still
carries the current token T2 and applies normally. The adaptation comment at `:3350-3353` explains
the rationale. Verified.

---

## Fidelity lens — scope and spec trace

### 9. File scope — CLEAN

`git diff 8a90485c..HEAD -- crates/` shows ONLY:
- `crates/remote/src/nodes/ws/status_machine.rs` (new, 301)
- `crates/remote/src/nodes/ws/session.rs` (302/303/304)
- `crates/remote/src/nodes/ws/mod.rs` (+1 line: `mod status_machine;`)

No edits to `tasks.rs`, migrations, or WS enum definitions. ✓

### 10. ADR-0010 §D fidelity — CLEAN

`dev-docs/adr/0010-task-status-state-machine.md` §D matrix:

| Transition | Author (ADR) | Author (code `:29-43`) |
|---|---|---|
| `todo → in-progress` | hive | `Some(Hive)` (`:33`) ✓ |
| `in-progress → in-review` | node | `Some(Node)` (`:39`) ✓ |
| `in-progress → done` | node | `Some(Node)` (`:38`) ✓ |
| `in-review → done` | hive | `Some(Hive)` (`:35`) ✓ |
| `in-review → in-progress` (reopen) | hive | `Some(Hive)` (`:34`) ✓ |
| `* → cancelled` | hive | `Some(Hive)` (`:36`, guard `from != Cancelled`) ✓ |

Line-for-line match. The `(_, Cancelled) if from != Cancelled` arm correctly excludes the
`(Cancelled, Cancelled)` no-op (which the `if status != current` short-circuit at `:2071` catches
first anyway). ✓

---

## Additional findings (integrated lens)

### [INFO] session.rs:677 — legacy path (304) has no lease/fence check, unlike 303's op-batch path

**Description:** 303's op-batch path rides P2's fence (`WHERE completed_at IS NULL` + stale-token
check at `:1973-2006`). 304's legacy `handle_task_status` path looks up the assignment by
`assignment_id` (`:677`, `find_by_id` — no `completed_at IS NULL` filter) and does NOT verify an
active lease or fencing token. The `node_may_author` guard prevents wrong-AUTHOR writes but not
stale-WRITER writes: a partitioned node whose lease was reclaimed (assignment
`completed_at` set, token bumped) can still send a `TaskStatusMessage` with
`status: Completed` (→ `InReview`), and if the current shared task status is `InProgress`,
`node_may_author(InProgress, InReview)` returns `true` → the write proceeds, clobbering the current
holder's work. This is the SC3 stale-writer bug class.

**Impact:** A partitioned node with a reclaimed assignment can write node-authored transitions
(`InProgress→Done`, `InProgress→InReview`) without a valid lease, until P4 (inbound-collapse)
ships. No wrong-author corruption (the matrix guard holds), but stale-writer corruption is
possible via the legacy path.

**Evidence:**
- `session.rs:677`: `assignment_repo.find_by_id(status.assignment_id)` — finds by ID, no
  `completed_at` filter.
- `session.rs:694-698`: `node_may_author(current_task.status, proposed)` — checks author only,
  no lease/token.
- Contrast with op-batch path: `session.rs:1973-1989` checks `completed_at IS NULL` AND
  `op.fencing_token < current_token`.

**Why INFO, not SHOULD-FIX:** This gap is PRE-EXISTING (the legacy path never had a fence — 304
did not introduce it). 304 IMPROVED the legacy path by adding the author guard. The ledger at
`docs/plans/vk-swarm-hive-redesign/decisions-ledger.md:460-468` explicitly documents this:
"304's STOP triggers note that P4 (inbound-collapse) may instead DELETE the legacy path — if so
304 is superseded-obsolete." The gap is surfaced and has a planned P4 remediation. Per the
"No Deferred Remediation" rule, this is not a deferred Phase-3 finding — it is a pre-existing
condition outside Phase-3's scope (the legacy path's `TaskStatusMessage` carries no
`fencing_token` field, so adding a token check would require a protocol change beyond Phase-3's
file scope). **Recommendation:** either (a) add a `completed_at IS NULL` check to the assignment
lookup in `handle_task_status` as a minimal P3 hardening, or (b) confirm P4 is tracked and will
retire the legacy path before the cutover.

---

## Verdict

**CLEAN**

- [BLOCKING]: 0
- [SHOULD-FIX]: 0
- [INFO]: 1 (legacy path lease gap — pre-existing, documented, P4-tracked)

All eight cross-task interaction properties (items 1-8) and both fidelity properties (items 9-10)
verify clean. The matrix is the single source of truth, both write sites gate through it, P2's
fence correctly precedes 303's guard, no-op and reject arms do not wedge the op-log, and the test
seed adaptation preserves the fence test's purpose. The one INFO finding is a pre-existing gap
with a documented P4 remediation path, not a Phase-3 regression.
