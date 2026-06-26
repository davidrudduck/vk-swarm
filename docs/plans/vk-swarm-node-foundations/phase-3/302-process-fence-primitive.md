---
id: "302"
phase: 3
title: Process liveness + fingerprint fence primitive
status: ready
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
In `crates/services/src/services/process_fence.rs` `#[cfg(test)] mod tests`:
```rust
#[test]
fn test_liveness_true_for_self_and_false_for_dead_pid() {
    // The current process is alive; its own pid must read alive.
    let me = std::process::id() as i64;
    assert!(is_alive(me));
    // A very high pid is almost certainly dead.
    assert!(!is_alive(999_999_999));
}

#[test]
fn test_fingerprint_mismatch_means_not_our_process() {
    // The current process's cmdline does NOT contain a bogus worktree marker,
    // so the PID-reuse guard must report "not our process" (do NOT fence it).
    let me = std::process::id() as i64;
    assert!(!is_our_process(me, "/var/tmp/vibe-kanban/worktrees/NONEXISTENT-MARKER"));
}

#[tokio::test]
async fn test_fence_returns_already_gone_for_dead_pid() {
    let r = fence(999_999_999, "anything").await;
    assert!(matches!(r, FenceOutcome::AlreadyGone));
}
```

## Change
- **File:** `crates/services/src/services/process_fence.rs` (NEW). A standalone, dependency-light module
  using `sysinfo` (already a `services` dependency — `crates/services/Cargo.toml:62 sysinfo = "0.37"`).
  Public surface:
  - `pub fn is_alive(pid: i64) -> bool` — a process with this pid currently exists.
  - `pub fn is_our_process(pid: i64, worktree_marker: &str) -> bool` — the live process's command line
    (`sysinfo` `Process::cmd()`/`exe()`) references `worktree_marker` (the expected
    `task_attempts.container_ref`). PID-reuse guard: a live PID whose cmdline does NOT contain the
    marker is someone else's reused PID → returns false (see ledger: cmdline-heuristic fingerprint).
  - `pub enum FenceOutcome { AlreadyGone, Fenced, NotOurProcess }`
  - `pub async fn fence(pid: i64, worktree_marker: &str) -> FenceOutcome` — if `!is_alive` →
    `AlreadyGone`; if alive but `!is_our_process` → `NotOurProcess` (do NOT kill); else terminate the
    process **and its descendants** (find children by ppid via sysinfo; `Process::kill_with(Signal::
    Term)` then escalate to `Kill`), poll until `!is_alive` (bounded retries/timeout), return `Fenced`.
    **Never returns `Fenced` until the worktree has no live writer** (the ADR-0001 safety invariant).
- **File:** `crates/services/src/services/mod.rs`
- **Anchor:** the `pub mod …;` / `mod …;` declaration block (read it; place alphabetically near other
  service modules).
- **Before:** (the existing module list)
- **After:** add `pub mod process_fence;` in the correct alphabetical position.

## Allowed moves
Create the fence module + register it. Pure liveness/fingerprint/terminate logic over sysinfo. Do NOT
wire it into `cleanup_orphan_executions` (that is task 304). Do NOT add a new crate dependency (sysinfo
is already present). Do NOT use `nix` (it is not a `services` dependency).

## Sibling alignment
This is a new leaf module, but it must match how other `services` modules expose helpers. Read one
existing simple `crates/services/src/services/*.rs` module for: module-doc header style, error handling
(return plain bool/enum vs `Result`), and test layout. The kill path should mirror any existing
process-kill helper if one exists (`grep -rn "kill_with\|Signal::" crates/services/src`); justify
divergence in the ledger.

## STOP triggers
- `sysinfo` 0.37 does not expose process command line / kill on the target OS as assumed → STOP and
  record; the cmdline-heuristic fingerprint may need the persisted-start-time fallback (a spec-scope
  escalation per the ledger decision), do not silently weaken the PID-reuse guard.
- Registering the module surfaces a name clash or the `services/mod.rs` block is structured differently
  than expected → reconcile against the real file before finalizing.

## Done when
`WAI_TYPECHECK_CMD="cargo check -p services" WAI_TEST_CMD="cargo test -p services process_fence" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-node-foundations 302` exits 0
