---
doc_type: spec
status: active
workstream: wai-ship-commit
change_kind: behaviour
---

# wai-ship-commit — Safety commit before graduation

## Intent (what / why)

`/wai:ship` (via `wai-close.sh`) only commits graduation artifacts — `git mv` renames,
`dev-docs/MASTER.md` regeneration, and the workstream README's status flip. It does **not**
commit any other outstanding working-tree changes made during the workstream: source code,
tests, configs, Dockerfiles, lockfiles, or any other modified/untracked files.

After `/wai:ship` runs and the PR merges, those uncommitted changes are silently left behind
in the worktree — the branch is "clean" from git's perspective only because the graduation
commit moved the docs, not because the actual work was saved.

**Goal:** `/wai:ship` must commit and push **all** outstanding working-tree changes before the
graduation commit, so nothing is lost and the worktree is truly clean after ship.

## Users / who is affected

US1: Any agent or human running `/wai:ship` to close a workstream. Currently, if the agent made
code changes during `/wai:execute` that weren't committed by the executor's per-task commit
step (e.g., manual edits, lint fixes, config tweaks), those changes are stranded.

## Success criteria

SC1: **No outstanding changes after ship.** → US1: After `/wai:ship` completes, `git status` shows a
clean working tree (no modified, untracked, or staged-but-uncommitted files).

SC2: **Safety commit precedes graduation.** → US1: The safety commit lands on the branch *before* the
graduation commit, so the PR contains both the work and the doc moves.

SC3: **Push includes safety commit.** → US1: The safety commit is pushed to the remote branch before
the merge menu is offered.

SC4: **Idempotent.** → US1: If there's nothing to commit (working tree already clean), the step is a
no-op — no empty commits, no errors.

SC5: **Deterministic scope.** → US1: The safety commit stages the entire working tree (`git add -A`),
matching the existing executor-experiment pattern (`wai-executor-experiment.sh:162`).

## Constraints

- Must not break the existing `wai-close.sh` clean-index guard (line 185: refuses to start
  with a pre-existing dirty *index*). The safety commit resolves this by committing first.
- Must not interfere with `wai-close.sh`'s targeted staging of `dev-docs/MASTER.md` and the
  workstream README (lines 252–253). The safety commit runs *before* close, so close's index
  guard sees a clean index.
- The safety commit message must be clearly distinguishable from the graduation commit
  (e.g., `ship(<topic>): save outstanding work before graduation`).
- Must work in worktrees (the primary use case for vk-swarm).

## Out of scope

- Granular per-file commit splitting (one big safety commit is acceptable).
- Modifying `wai-close.sh` itself — the safety commit is a `/wai:ship` concern, not a
  general close concern. (Spec-only workstreams that skip ship don't need this.)
- Pre-commit hooks or linting on the safety commit — it's a save point, not a review gate.
- Changing the executor's per-task commit behavior (that's a separate workstream).

## Approach

**Single insertion point in `wai-ship.sh`.** The safety commit step is added after the
`fail()` helper definition (line 41) and the already-shipped skip check (lines 114–119),
but before the spec resolution (line 122). This ordering is mandatory for two reasons:

1. `fail()` must be defined before the safety block calls it (kimi/deepseek finding).
2. The already-shipped skip must run before the safety commit to avoid creating a stranded
   local commit when re-running ship on an already-shipped workstream (gpt-5.5 finding).

No other file is modified. `wai-close.sh` is untouched — it already handles its own
deterministic commit scope correctly and must not be burdened with a general "save everything"
concern.

**Why `wai-ship.sh` and not `wai-close.sh`:** Close is a general-purpose graduation tool
used by spec-only workstreams, manual close calls, and the orchestrator. Adding a blanket
`git add -A` there would change the semantics of every close invocation. Ship is the only
flow that needs "save everything before merging" — it's the terminal step, the merge gate,
and the only place where losing work is irreversible.

## Design / architecture

### Step: safety commit (inserted after the already-shipped check in `wai-ship.sh`)

**Insertion point:** After the already-shipped skip block (current line 119: `exit 0`),
before the spec resolution (current line 122). At this point `fail()` is already defined
(line 41) and the already-shipped check has confirmed ship will proceed.

```bash
# --- Safety commit: save all outstanding work before graduation ---
# The working tree may contain uncommitted changes from the workstream (code edits, config
# tweaks, lockfiles, test fixtures) that the executor's per-task commit didn't capture.
# Commit them NOW so nothing is lost when the branch merges. wai-close.sh's clean-index
# guard (line 185) will see a clean index because this commit runs first.
#
# Detection: `git status --porcelain` covers both tracked modifications AND untracked files
# (git diff --quiet only checks tracked files, missing new files). Empty output = clean.
if [ -n "$(git status --porcelain 2>/dev/null)" ]; then
  git add -A \
    || fail "safety commit: git add -A failed (see output above)."
  git commit -q -m "ship($TOPIC): save outstanding work before graduation" \
    || fail "safety commit: git commit failed (see output above). No ship frontmatter edits have been made; note that 'git add -A' already staged the working tree."
  echo "  safety commit: all outstanding working-tree changes committed" >&2
else
  echo "  safety commit: working tree clean, nothing to commit" >&2
fi
```

### Execution order (full ship flow after change)

1. Validate slug
2. `repo_root` + `cd`
3. **Branch guard** (`wai-branch-guard.sh --ensure`) — move to `wai/<topic>`
4. `fail()` helper defined
5. Workstream README exists check
6. Already-shipped skip check (exits 0 if already shipped — no safety commit runs)
7. **Safety commit** (NEW) — `git add -A && git commit` if working tree is dirty
8. Resolve spec
9. Pre-flight evidence gate
10. Flip spec status → shipped
11. Flip README status → shipped + set staging_pointers
12. `wai-close.sh` (graduates, commits graduation, pushes)

### Why `git status --porcelain` instead of `git diff --quiet`

- `git diff --quiet` only checks tracked file modifications. Untracked files (new source
  files, test fixtures, generated configs) would be silently missed.
- `git status --porcelain` covers everything: modified tracked files, staged changes,
  untracked files, and deleted files. Empty output = truly clean.
- This matches the success criterion: "no modified, untracked, or staged-but-uncommitted files."

### Why `git add -A` (not selective paths)

- Matching the existing executor-experiment pattern (`wai-executor-experiment.sh:162`).
- The workstream may have touched arbitrary directories (crates/, frontend/, docs/, config
  files at root). Selective path-based staging would need to enumerate every possible
  directory — brittle and incomplete.
- One safety commit is acceptable per the out-of-scope section. Granularity is not a goal.

### Interaction with `wai-close.sh`'s clean-index guard

`wai-close.sh` line 185 checks `git diff --cached --quiet` and refuses to start with a
pre-existing dirty index. The safety commit runs before close, so by the time close enters,
the index is clean (the safety commit committed everything). Close then stages only its own
targeted artifacts (moved destinations, MASTER.md, README) — exactly as it does today.

### Push behavior

The safety commit is pushed by `wai-close.sh`'s existing push logic (lines 268–281), which
pushes the entire feature branch. The safety commit + the graduation commit both land on the
remote in one push. No new push logic is needed in `wai-ship.sh`.

## Decisions

1. **Location: `wai-ship.sh`, not `wai-close.sh`.** Close is general-purpose; ship is the
   terminal merge gate. Adding `git add -A` to close would change semantics for all close
   callers (spec-only workstreams, manual close, orchestrator). Ship is the only flow where
   "save everything" is the right default.
   — Reversible: removing the step from ship later has no side effects.

2. **Detection: `git status --porcelain`, not `git diff --quiet`.** Covers untracked files.
   `git diff --quiet` misses new files that were never staged — a common case when an agent
   creates a new source file during execute but the per-task commit didn't capture it.
   — Reversible: can switch to `git diff --quiet` if untracked-file coverage is unwanted.

3. **Commit message format: `ship(<topic>): save outstanding work before graduation`.**
   Distinct from close's `close(<topic>): graduate staged docs into dev-docs/` — clearly
   signals this is a save-point, not a graduation. Uses the same `<topic>` slug as close.
   — Reversible: message format has no functional impact.

4. **No WAI_NO_SAVE bypass.** Unlike close's `WAI_NO_PUSH` escape hatch, there is no
   env var to skip the safety commit. If the working tree is dirty, ship commits it. Period.
   The user's explicit requirement: "do not defer any item... everything gets done as planned."
   — Reversible: adding a bypass later is trivial if needed.

5. **Fail-hard on commit failure.** If `git add -A` or `git commit` fails, ship aborts
   with `SHIP FAIL`. No files have been modified by ship yet at this point (the safety
   commit runs before any frontmatter edits), so the workstream is in a clean retry state.
   — Reversible: can downgrade to warn-and-continue if needed.

6. **Already-shipped skip runs before safety commit.** The already-shipped idempotency
   check (lines 114–119) must run before the safety commit. If the workstream is already
   shipped, ship exits immediately without creating any new commits. This prevents a
   stranded local commit on re-runs where `wai-close.sh` (and its push logic) never fires.
   — Reversible: reordering would re-introduce the stranded-commit bug.

## Test strategy

1. **Unit test: dirty working tree → safety commit created.** Set up a workstream with an
   uncommitted file, run `wai-ship.sh`, verify a commit with message `ship(<topic>): save
   outstanding work before graduation` exists in the log before the graduation commit.

2. **Unit test: clean working tree → no safety commit.** Set up a workstream with no
   uncommitted files, run `wai-ship.sh`, verify no extra commit was created (only the
   graduation commit).

3. **Unit test: untracked files included.** Create a new untracked file in the working tree,
   run `wai-ship.sh`, verify the file is included in the safety commit.

4. **Integration test: full ship flow with dirty tree.** Dirty working tree + frontmatter
   edits + graduation → verify `git status` is clean after ship, and the branch has two
   commits (safety + graduation) before the merge menu.

5. **Integration test: wai-close.sh index guard still passes.** Dirty tree → ship → close
   → verify close's `git diff --cached --quiet` guard does not fire (index is clean because
   the safety commit ran first).

6. **Unit test: already-shipped re-run → no safety commit.** Set up a workstream that is
   already shipped (README status: shipped), with a dirty working tree, run `wai-ship.sh`,
   verify no commit was created and the script exited with `SHIP SKIPPED`.
