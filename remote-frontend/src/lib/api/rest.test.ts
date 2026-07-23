// NOTE (deviation from task 305 literal spec): the task file's assertion
// `(init.headers as Record<string,string>).Authorization` reads a property
// directly off the `Headers` object built by `makeRequest`. `Headers` is a
// class with only method access (`.get()`), so bracket/dot property access
// always returns `undefined` at runtime — this is a strict-TS-vs-actual-
// runtime-shape defect in the literal test text. Fixed to `.get('Authorization')`,
// matching the pattern already used in `organizations.test.ts`. Intent
// (assert the Bearer header is sent) is unchanged.
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { nodesApi } from './nodes';
import { tasksApi } from './tasks';
import { swarmLabelsApi } from './swarmLabels';

beforeEach(() => {
  localStorage.setItem('access_token', 'test-token');
  vi.spyOn(globalThis, 'fetch').mockResolvedValue({ ok: true, json: async () => ({}) } as Response);
});

describe('nodesApi (SC8)', () => {
  it('list(orgId) GETs /v1/nodes?organization_id= with Bearer header', async () => {
    await nodesApi.list('org-1');
    const [url, init] = (globalThis.fetch as any).mock.calls[0];
    expect(url).toContain('/v1/nodes?organization_id=org-1');
    expect((init.headers as Headers).get('Authorization')).toBe('Bearer test-token');
  });
});

describe('tasksApi (SC8)', () => {
  it('bulk(projectId) GETs /v1/tasks/bulk?project_id= with Bearer header', async () => {
    await tasksApi.bulk('proj-1');
    const [url, init] = (globalThis.fetch as any).mock.calls[0];
    expect(url).toContain('/v1/tasks/bulk?project_id=proj-1');
    expect((init.headers as Headers).get('Authorization')).toBe('Bearer test-token');
  });
  it('setExecutingNode(id, nodeId) PATCHes /v1/tasks/{id}/executing-node with {node_id}', async () => {
    await tasksApi.setExecutingNode('t1', 'n1');
    const [url, init] = (globalThis.fetch as any).mock.calls[0];
    expect(url).toContain('/v1/tasks/t1/executing-node');
    expect(init.method).toBe('PATCH');
    expect(JSON.parse(init.body)).toEqual({ node_id: 'n1' });
  });
});

describe('swarmLabelsApi (SC8)', () => {
  it('list(orgId) GETs /v1/swarm/labels?organization_id= with Bearer header', async () => {
    await swarmLabelsApi.list('org-1');
    const [url, init] = (globalThis.fetch as any).mock.calls[0];
    expect(url).toContain('/v1/swarm/labels?organization_id=org-1');
    expect((init.headers as Headers).get('Authorization')).toBe('Bearer test-token');
  });
});
