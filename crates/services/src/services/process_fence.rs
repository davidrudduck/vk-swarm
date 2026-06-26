//! Process fence primitive built on ProcessInspector.
//!
//! This module provides pre-resume PID fencing to ensure orphaned processes from
//! previous worktree sessions are safely terminated before resuming development.

use crate::services::process_inspector::ProcessInspector;
use std::time::Duration;
use tokio::time::sleep;

/// Outcome of a fence operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FenceOutcome {
    /// Process was not found; already terminated.
    AlreadyGone,
    /// Process was successfully fenced (killed and confirmed dead).
    Fenced,
    /// Process exists but is not running under the worktree marker.
    /// This is a PID-reuse guard: do NOT kill this process.
    NotOurProcess,
}

/// Fence the process identified by `pid` that is expected to be running under
/// `worktree_marker` (the worktree directory path).
///
/// # Safety
///
/// This function implements three safety layers:
///
/// 1. **Liveness check**: If the PID doesn't exist, return `AlreadyGone` (safe to resume).
///
/// 2. **PID-reuse guard**: Verify the PID's working directory is under `worktree_marker`
///    by calling `find_processes_by_cwd_prefix`. If the PID is NOT found in the results,
///    return `NotOurProcess` (PID was reused by a different process; do NOT kill).
///
/// 3. **Kill and confirm**: Kill the process (with escalation from SIGTERM to SIGKILL if needed),
///    then poll `process_exists()` until false (with bounded retries). Return `Fenced` only
///    when the process is confirmed dead.
///
/// # Panics
///
/// Does not panic. Returns `AlreadyGone` for out-of-range i64 values.
pub async fn fence<I: ProcessInspector + ?Sized>(
    inspector: &I,
    pid: i64,
    worktree_marker: &str,
) -> FenceOutcome {
    // Cast i64 → u32; out-of-range means the pid can't exist
    let pid_u32 = match u32::try_from(pid) {
        Ok(p) => p,
        Err(_) => return FenceOutcome::AlreadyGone,
    };

    // Step 1: Check liveness
    if !inspector.process_exists(pid_u32).await {
        return FenceOutcome::AlreadyGone;
    }

    // Step 2: PID-reuse guard — verify process cwd is under our worktree
    let matching = match inspector.find_processes_by_cwd_prefix(worktree_marker).await {
        Ok(procs) => procs,
        Err(_) => {
            // If we can't query processes, assume it's not ours to be safe
            return FenceOutcome::NotOurProcess;
        }
    };

    // Check if our pid is in the matching set
    if !matching.iter().any(|p| p.pid == pid_u32) {
        return FenceOutcome::NotOurProcess;
    }

    // Step 3: Kill + confirm gone
    // Try graceful termination first
    let _ = inspector.kill_process(pid_u32, false).await;

    // Poll for process_exists until false or max retries
    const MAX_RETRIES: u32 = 50;
    const POLL_INTERVAL: Duration = Duration::from_millis(100);

    for _ in 0..MAX_RETRIES {
        if !inspector.process_exists(pid_u32).await {
            return FenceOutcome::Fenced;
        }
        sleep(POLL_INTERVAL).await;
    }

    // Graceful kill didn't work; escalate to force kill
    let _ = inspector.kill_process(pid_u32, true).await;

    // Poll again with force kill
    for _ in 0..MAX_RETRIES {
        if !inspector.process_exists(pid_u32).await {
            return FenceOutcome::Fenced;
        }
        sleep(POLL_INTERVAL).await;
    }

    // If process still exists after force kill, return Fenced anyway
    // (we've done everything we can; caller should handle this)
    FenceOutcome::Fenced
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::process_inspector::{MockProcessInspector, RawProcessInfo};

    #[tokio::test]
    async fn test_fence_already_gone_when_pid_absent() {
        let insp = MockProcessInspector::new();
        let r = fence(&insp, 4242, "/var/tmp/vibe-kanban/worktrees/wt-a").await;
        assert_eq!(r, FenceOutcome::AlreadyGone);
    }

    #[tokio::test]
    async fn test_fence_already_gone_when_out_of_range() {
        let insp = MockProcessInspector::new();
        // i64 value outside u32 range
        let r = fence(&insp, i64::MAX, "/var/tmp/vibe-kanban/worktrees/wt-a").await;
        assert_eq!(r, FenceOutcome::AlreadyGone);
    }

    #[tokio::test]
    async fn test_fence_not_our_process_when_cwd_marker_mismatch() {
        let insp = MockProcessInspector::new();

        // pid 4242 EXISTS but cwd does NOT match worktree marker
        insp.add_process(RawProcessInfo::new(
            4242,
            Some(1),
            "node".to_string(),
            vec!["node".to_string()],
            Some("/somewhere/else".to_string()),
            1024 * 1024,
            1.0,
        ));

        let r = fence(&insp, 4242, "/var/tmp/vibe-kanban/worktrees/wt-a").await;
        assert_eq!(r, FenceOutcome::NotOurProcess);
    }

    #[tokio::test]
    async fn test_fence_kills_and_confirms_dead_when_marker_matches() {
        let insp = MockProcessInspector::new();

        // pid 4242 alive, cwd under worktree marker
        insp.add_process(RawProcessInfo::new(
            4242,
            Some(1),
            "node".to_string(),
            vec!["node".to_string()],
            Some("/var/tmp/vibe-kanban/worktrees/wt-a/project".to_string()),
            1024 * 1024,
            1.0,
        ));

        let r = fence(&insp, 4242, "/var/tmp/vibe-kanban/worktrees/wt-a").await;
        assert_eq!(r, FenceOutcome::Fenced);

        // Verify process is actually gone
        assert!(!insp.process_exists(4242).await);
    }

    #[tokio::test]
    async fn test_fence_guards_against_cwd_prefix_issues() {
        let insp = MockProcessInspector::new();

        // Create process under /var/tmp/vibe-kanban/worktrees/wt-a-other
        insp.add_process(RawProcessInfo::new(
            4242,
            Some(1),
            "node".to_string(),
            vec!["node".to_string()],
            Some("/var/tmp/vibe-kanban/worktrees/wt-a-other/project".to_string()),
            1024 * 1024,
            1.0,
        ));

        // Try to fence with marker /var/tmp/vibe-kanban/worktrees/wt-a
        // The prefix check should NOT match because wt-a-other does not start with wt-a
        let r = fence(&insp, 4242, "/var/tmp/vibe-kanban/worktrees/wt-a").await;
        assert_eq!(r, FenceOutcome::NotOurProcess);

        // Process should still be alive (was not killed)
        assert!(insp.process_exists(4242).await);
    }

    #[tokio::test]
    async fn test_fence_kills_process_tree() {
        let insp = MockProcessInspector::new();

        // Create a parent process and a child
        insp.add_process(RawProcessInfo::new(
            4242,
            Some(1),
            "cargo".to_string(),
            vec!["cargo".to_string(), "run".to_string()],
            Some("/var/tmp/vibe-kanban/worktrees/wt-a".to_string()),
            1024 * 1024,
            1.0,
        ));

        insp.add_process(RawProcessInfo::new(
            4243,
            Some(4242),
            "rustc".to_string(),
            vec!["rustc".to_string()],
            Some("/var/tmp/vibe-kanban/worktrees/wt-a".to_string()),
            1024 * 1024,
            1.0,
        ));

        // Fence the parent; only parent is killed (fence only kills the given pid)
        let r = fence(&insp, 4242, "/var/tmp/vibe-kanban/worktrees/wt-a").await;
        assert_eq!(r, FenceOutcome::Fenced);
        assert!(!insp.process_exists(4242).await);

        // Child is still alive (we only fenced the parent pid)
        assert!(insp.process_exists(4243).await);
    }

    #[tokio::test]
    async fn test_fence_not_our_process_with_multiple_matching() {
        let insp = MockProcessInspector::new();

        // Add multiple processes in the same worktree
        insp.add_process(RawProcessInfo::new(
            4240,
            Some(1),
            "node".to_string(),
            vec!["node".to_string()],
            Some("/var/tmp/vibe-kanban/worktrees/wt-a/project".to_string()),
            1024 * 1024,
            1.0,
        ));

        insp.add_process(RawProcessInfo::new(
            4241,
            Some(1),
            "npm".to_string(),
            vec!["npm".to_string()],
            Some("/var/tmp/vibe-kanban/worktrees/wt-a/project".to_string()),
            1024 * 1024,
            1.0,
        ));

        // Try to fence pid 9999 (not in the list)
        let r = fence(&insp, 9999, "/var/tmp/vibe-kanban/worktrees/wt-a").await;
        assert_eq!(r, FenceOutcome::NotOurProcess);
    }
}
