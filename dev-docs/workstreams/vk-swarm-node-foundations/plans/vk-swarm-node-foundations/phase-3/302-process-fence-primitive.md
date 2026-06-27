---
id: "302"
phase: 3
title: Process fence primitive built on the existing ProcessInspector
status: passed
depends_on: []
parallel: false
conflicts_with: []
files:
  - crates/services/src/services/process_fence.rs
  - crates/services/src/services/mod.rs
irreversible: false
scope_test: "crates/services/src/services/process_fence.rs"
allowed_change: mixed
covers_criteria: [SC1]
---
## Failing test (write first)
In `crates/services/src/services/process_fence.rs` `#[cfg(test)] mod tests`, drive the fence with the
**existing `MockProcessInspector`** (`crates/services/src/services/process_inspector/mock.rs`) — no real
processes needed:
```rust
#[tokio::test]
async fn test_fence_already_gone_when_pid_absent() {
    let insp = MockProcessInspector::new(); // configure: pid 4242 does NOT exist
    let r = fence(&insp, 4242, "/var/tmp/vibe-kanban/worktrees/wt-a").await;
    assert!(matches!(r, FenceOutcome::AlreadyGone));
}

#[tokio::test]
async fn test_fence_not_our_process_when_cwd_marker_mismatch() {
    // pid 4242 EXISTS but its cwd does NOT match the worktree marker -> reused PID, do NOT kill.
    let insp = MockProcessInspector::new(); // configure: 4242 alive, cwd "/somewhere/else"
    let r = fence(&insp, 4242, "/var/tmp/vibe-kanban/worktrees/wt-a").await;
    assert!(matches!(r, FenceOutcome::NotOurProcess));
}

#[tokio::test]
async fn test_fence_kills_and_confirms_dead_when_marker_matches() {
    // pid 4242 alive, cwd under the worktree marker -> fence kills it (+ tree) and confirms gone.
    let insp = MockProcessInspector::new(); // configure: 4242 alive, cwd = marker; exists()->false after kill
    let r = fence(&insp, 4242, "/var/tmp/vibe-kanban/worktrees/wt-a").await;
    assert!(matches!(r, FenceOutcome::Fenced));
}
```
(Match `MockProcessInspector`'s real constructor/configuration API — READ `mock.rs` first; the calls
above are illustrative.)

## Change
**Do NOT reimplement liveness/kill/process-tree/cmdline logic — it already exists.** The
`ProcessInspector` trait (`crates/services/src/services/process_inspector/mod.rs:81`) and
`SysinfoProcessInspector` (`sysinfo_impl.rs`) already provide: `process_exists(pid)` (liveness, `:170`),
`kill_process(pid, force)` (SIGTERM→SIGKILL, `:136`), `get_process_tree(...)` (descendants, `:81`),
`find_processes_by_cwd_prefix(prefix)` (worktree-cwd match — a STRONGER PID-reuse guard than a cmdline
heuristic, `:115`), and a `MockProcessInspector` for tests. The fence is a thin ORCHESTRATION over them.

- **File:** `crates/services/src/services/process_fence.rs` (NEW). Add:
  - `pub enum FenceOutcome { AlreadyGone, Fenced, NotOurProcess }`
  - `pub async fn fence<I: ProcessInspector + ?Sized>(inspector: &I, pid: i64, worktree_marker: &str)
    -> FenceOutcome` (accept the trait so the test can pass the mock; cast pid to the inspector's `u32`
    as its API requires):
    1. if `!inspector.process_exists(pid).await` → `AlreadyGone`.
    2. **PID-reuse guard (the fingerprint):** confirm the live pid is OUR agent by cwd — use
       `inspector.find_processes_by_cwd_prefix(worktree_marker)` and check the pid is in the result (or
       inspect the process's cwd/cmd). If it is NOT → `NotOurProcess` (do NOT kill; a reused PID).
    3. else terminate it and its descendants — `get_process_tree` + `kill_process(.., force=false)`
       then escalate `force=true` — and poll `process_exists` until false (bounded retries). Return
       `Fenced`. **Never return `Fenced` until `process_exists` is false** (ADR-0001 safety invariant).
  - Construct the concrete inspector for production callers via `SysinfoProcessInspector::new()` (304
    passes it in, or `fence` has a convenience wrapper that builds one).
- **File:** `crates/services/src/services/mod.rs` — add `pub mod process_fence;` (alphabetical; it sits
  right after the existing `pub mod process_inspector;` at `:46`).

## Allowed moves
Create the fence orchestration module + register it. REUSE `ProcessInspector` for every OS interaction —
do not call raw `sysinfo` and do not duplicate `process_exists`/`kill_process`/`get_process_tree`/
`find_processes_by_cwd_prefix`. Do NOT wire into `cleanup_orphan_executions` (that is 304). Do NOT add a
crate dependency (none needed — `process_inspector` already encapsulates `sysinfo`).

## Sibling alignment
The sibling is the existing `crates/services/src/services/process_inspector/` module — READ its `mod.rs`
(trait + `ProcessInspectorError`), `sysinfo_impl.rs` (real impl + signatures, esp. `u32` pid type and
`find_processes_by_cwd_prefix` return shape), and `mock.rs` (test configuration API). The fence MUST
consume this trait, not reimplement it (this is the extension-sdk-boundary-fix antipattern the review
caught — breakdown-review R2). Justify in the ledger any helper you add that the inspector doesn't
already provide.

## STOP triggers
- `find_processes_by_cwd_prefix` does not actually expose each process's pid/cwd in a way that lets you
  match a SPECIFIC pid to the worktree → fall back to the inspector's cmd/cwd accessor on
  `RawProcessInfo`; if neither is usable, STOP and record (the cwd fingerprint is the chosen PID-reuse
  guard — do not silently weaken it to bare liveness).
- The `ProcessInspector` pid type (`u32`) vs the stored `pid` (`i64` in `execution_processes`) needs a
  cast — do it at the boundary; if a value doesn't fit, treat as not-found and record.

## Done when
`WAI_TYPECHECK_CMD="cargo check -p services" WAI_TEST_CMD="cargo test -p services process_fence" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-node-foundations 302` exits 0
