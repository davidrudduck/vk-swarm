import { describe, it, expect } from 'vitest';
import {
  ELECTRIC_PROXY_BASE,
  ELECTRIC_SHAPE_TABLES,
  createShapeUrl,
  createSharedTasksCollection,
  type ElectricSharedTask,
} from './index';

describe('electric config (SC8)', () => {
  it('ELECTRIC_PROXY_BASE points at the hive proxy (/v1/shape), not the node proxy', () => {
    expect(ELECTRIC_PROXY_BASE).toBe('/v1/shape');
  });

  it('ELECTRIC_SHAPE_TABLES contains exactly the proxied table', () => {
    // The hive proxy serves one shape route: /v1/shape/shared_tasks
    // (crates/remote/src/routes/electric_proxy.rs). Tables removed at the
    // 2026-08-28 close review (nodes, projects, node_projects,
    // node_task_*, …) had no proxy route — their URLs 404'd.
    const keys = Object.keys(ELECTRIC_SHAPE_TABLES);
    expect(keys).toEqual(['shared_tasks']);
  });

  it('createShapeUrl produces hive-proxy URLs', () => {
    expect(createShapeUrl('shared_tasks')).toBe('/v1/shape/shared_tasks');
  });
});

describe('electric collections (SC8)', () => {
  it('exposes the shared-tasks collection factory', () => {
    expect(typeof createSharedTasksCollection).toBe('function');
  });

  it('ElectricSharedTask extends ElectricRow (open schema)', () => {
    // The authoritative shared_tasks schema lives in the hive's PostgreSQL,
    // so the type only pins the proxy-guaranteed columns and stays open
    // for the rest via the ElectricRow index signature.
    const t: ElectricSharedTask = {
      id: 't1',
      organization_id: 'org1',
      title: 'flowing through the index signature',
    };
    expect(t.id).toBe('t1');
  });
});
