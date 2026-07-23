import { describe, it, expect } from 'vitest';
import {
  ELECTRIC_PROXY_BASE,
  ELECTRIC_SHAPE_TABLES,
  createShapeUrl,
} from './index';
import {
  createNodesCollection,
  createProjectsCollection,
  createNodeProjectsCollection,
  createTaskAssignmentsCollection,
  createTaskOutputLogsCollection,
  createTaskProgressEventsCollection,
  type ElectricTaskAssignment,
  type ElectricTaskOutputLog,
  type ElectricTaskProgressEvent,
} from './index';

describe('electric config (SC8)', () => {
  it('ELECTRIC_PROXY_BASE points at the hive proxy (/v1/shape), not the node proxy', () => {
    expect(ELECTRIC_PROXY_BASE).toBe('/v1/shape');
  });

  it('ELECTRIC_SHAPE_TABLES has 6 tables', () => {
    const keys = Object.keys(ELECTRIC_SHAPE_TABLES);
    expect(keys).toHaveLength(6);
    expect(keys).toContain('node_task_assignments');
    expect(keys).toContain('node_task_output_logs');
    expect(keys).toContain('node_task_progress_events');
  });

  it('createShapeUrl produces hive-proxy URLs', () => {
    expect(createShapeUrl('nodes')).toBe('/v1/shape/nodes');
    expect(createShapeUrl('node_task_assignments')).toBe(
      '/v1/shape/node_task_assignments'
    );
  });
});

describe('electric collections (SC8)', () => {
  it('all 6 collection factories are functions', () => {
    expect(typeof createNodesCollection).toBe('function');
    expect(typeof createProjectsCollection).toBe('function');
    expect(typeof createNodeProjectsCollection).toBe('function');
    expect(typeof createTaskAssignmentsCollection).toBe('function');
    expect(typeof createTaskOutputLogsCollection).toBe('function');
    expect(typeof createTaskProgressEventsCollection).toBe('function');
  });

  // NOTE: field shapes below match the schema-aligned types already shipped
  // in this repo (and consumed by src/pages/Tasks.tsx), NOT the simplified
  // literals in docs/plans/vk-swarm-design-system/phase-3/306-*.md. See the
  // decisions-ledger entry for task 306 for why the plan's literal shapes
  // were not adopted verbatim (they conflict with an existing consumer).
  it('new types extend ElectricRow', () => {
    const a: ElectricTaskAssignment = {
      id: 'a',
      task_id: 't',
      node_id: 'n',
      node_project_id: 'np',
      local_task_id: null,
      local_attempt_id: null,
      execution_status: 'pending',
      assigned_at: '2026-01-01T00:00:00Z',
      started_at: null,
      completed_at: null,
      created_at: '2026-01-01T00:00:00Z',
      lease_expires_at: null,
      fencing_token: 1,
    };
    const o: ElectricTaskOutputLog = {
      id: 'o',
      assignment_id: 'a',
      output_type: 'stdout',
      content: 'm',
      timestamp: '2026-01-01T00:00:00Z',
      created_at: '2026-01-01T00:00:00Z',
      execution_process_id: null,
    };
    const p: ElectricTaskProgressEvent = {
      id: 'p',
      assignment_id: 'a',
      event_type: 'e',
      message: 'm',
      metadata: null,
      timestamp: '2026-01-01T00:00:00Z',
      created_at: '2026-01-01T00:00:00Z',
    };
    expect(a.id).toBe('a');
    expect(o.id).toBe('o');
    expect(p.id).toBe('p');
  });
});
