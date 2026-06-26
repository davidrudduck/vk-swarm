//! Mock implementation of ProcessInspector for testing.
//!
//! This implementation maintains an in-memory list of processes that can be
//! manipulated for testing purposes.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

use async_trait::async_trait;

use super::{ProcessInspector, ProcessInspectorError, RawProcessInfo};

/// Mock implementation of ProcessInspector for testing.
///
/// Stores processes in memory and allows adding/removing them programmatically.
/// - Default: process is removed immediately on any kill call.
/// - Resilient (`set_resilient`): survives graceful (SIGTERM) kill, dies on force (SIGKILL).
/// - Unkillable (`set_unkillable`): survives both graceful and force kills (D-state simulation).
#[derive(Clone)]
pub struct MockProcessInspector {
    processes: Arc<RwLock<HashMap<u32, RawProcessInfo>>>,
    /// PIDs that survive graceful (force=false) kill but die on force (force=true) kill.
    resilient_pids: Arc<RwLock<HashSet<u32>>>,
    /// PIDs that survive both graceful and force kills (true D-state simulation).
    unkillable_pids: Arc<RwLock<HashSet<u32>>>,
}

impl Default for MockProcessInspector {
    fn default() -> Self {
        Self::new()
    }
}

impl MockProcessInspector {
    /// Create a new empty MockProcessInspector.
    pub fn new() -> Self {
        Self {
            processes: Arc::new(RwLock::new(HashMap::new())),
            resilient_pids: Arc::new(RwLock::new(HashSet::new())),
            unkillable_pids: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    /// Mark a PID as resilient: it survives SIGTERM but is removed on SIGKILL.
    /// Enables testing the SIGTERM→poll→SIGKILL escalation path in `fence`.
    pub fn set_resilient(&self, pid: u32) {
        let mut resilient = self.resilient_pids.write().unwrap();
        resilient.insert(pid);
    }

    /// Mark a PID as unkillable even with SIGKILL, simulating a true D-state process.
    /// `fence` will return `CouldNotKill` for processes in this state.
    pub fn set_unkillable(&self, pid: u32) {
        let mut unkillable = self.unkillable_pids.write().unwrap();
        unkillable.insert(pid);
    }

    /// Add a process to the mock.
    pub fn add_process(&self, process: RawProcessInfo) {
        let mut processes = self.processes.write().unwrap();
        processes.insert(process.pid, process);
    }

    /// Remove a process from the mock by PID.
    pub fn remove_process(&self, pid: u32) -> Option<RawProcessInfo> {
        let mut processes = self.processes.write().unwrap();
        processes.remove(&pid)
    }

    /// Clear all processes.
    pub fn clear(&self) {
        let mut processes = self.processes.write().unwrap();
        processes.clear();
    }

    /// Get the current process count.
    pub fn process_count(&self) -> usize {
        let processes = self.processes.read().unwrap();
        processes.len()
    }

    /// Build a tree of descendant PIDs for a given root PID.
    fn find_descendants(&self, root_pid: u32) -> Vec<u32> {
        let processes = self.processes.read().unwrap();

        // Find all children recursively
        let mut descendants = Vec::new();
        let mut to_visit = vec![root_pid];

        while let Some(current_pid) = to_visit.pop() {
            for process in processes.values() {
                if process.parent_pid == Some(current_pid)
                    && process.pid != root_pid
                    && !descendants.contains(&process.pid)
                {
                    descendants.push(process.pid);
                    to_visit.push(process.pid);
                }
            }
        }

        descendants
    }
}

#[async_trait]
impl ProcessInspector for MockProcessInspector {
    async fn list_processes(&self) -> Result<Vec<RawProcessInfo>, ProcessInspectorError> {
        let processes = self.processes.read().unwrap();
        Ok(processes.values().cloned().collect())
    }

    async fn get_process_tree(
        &self,
        root_pid: u32,
    ) -> Result<Vec<RawProcessInfo>, ProcessInspectorError> {
        let descendant_pids = self.find_descendants(root_pid);
        let processes = self.processes.read().unwrap();

        let descendants = descendant_pids
            .iter()
            .filter_map(|pid| processes.get(pid).cloned())
            .collect();

        Ok(descendants)
    }

    async fn find_processes_by_cwd_prefix(
        &self,
        cwd_prefix: &str,
    ) -> Result<Vec<RawProcessInfo>, ProcessInspectorError> {
        let processes = self.processes.read().unwrap();

        let matches = processes
            .values()
            .filter(|p| {
                p.working_directory
                    .as_ref()
                    .map(|cwd| cwd.starts_with(cwd_prefix))
                    .unwrap_or(false)
            })
            .cloned()
            .collect();

        Ok(matches)
    }

    async fn kill_process(&self, pid: u32, force: bool) -> Result<(), ProcessInspectorError> {
        // Check unkillable first (survives both graceful and force kills)
        {
            let unkillable = self.unkillable_pids.read().unwrap();
            if unkillable.contains(&pid) {
                let processes = self.processes.read().unwrap();
                return if processes.contains_key(&pid) {
                    Ok(()) // Signal sent; process didn't die
                } else {
                    Err(ProcessInspectorError::ProcessNotFound(pid))
                };
            }
        }

        // Check resilient (survives graceful but dies on force)
        if !force {
            let resilient = self.resilient_pids.read().unwrap();
            if resilient.contains(&pid) {
                let processes = self.processes.read().unwrap();
                return if processes.contains_key(&pid) {
                    Ok(()) // SIGTERM sent; process didn't die
                } else {
                    Err(ProcessInspectorError::ProcessNotFound(pid))
                };
            }
        }

        let mut processes = self.processes.write().unwrap();
        if processes.remove(&pid).is_some() {
            Ok(())
        } else {
            Err(ProcessInspectorError::ProcessNotFound(pid))
        }
    }

    async fn process_exists(&self, pid: u32) -> bool {
        let processes = self.processes.read().unwrap();
        processes.contains_key(&pid)
    }
}
