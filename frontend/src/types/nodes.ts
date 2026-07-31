/**
 * Node types for the swarm/hive architecture.
 * These types correspond to the Rust types in crates/remote/src/nodes/domain.rs
 */

export type NodeStatus = 'pending' | 'online' | 'offline' | 'busy' | 'draining';

export interface NodeCapabilities {
  executors: string[];
  max_concurrent_tasks: number;
  os: string;
  arch: string;
  version: string;
  git_commit: string;
  git_branch: string;
}

export interface Node {
  id: string;
  organization_id: string;
  name: string;
  machine_id: string;
  status: NodeStatus;
  capabilities: NodeCapabilities;
  public_url: string | null;
  last_heartbeat_at: string | null;
  connected_at: string | null;
  disconnected_at: string | null;
  created_at: string;
  updated_at: string;
}

export interface NodeProject {
  id: string;
  node_id: string;
  local_project_id: string;
  name: string;
  git_repo_path: string;
  default_branch: string;
  swarm_project_id: string | null;
  swarm_project_name: string | null;
  last_seen_at: string;
  created_at: string;
}
