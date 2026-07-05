import { describe, it, expect } from 'vitest';
import { ELECTRIC_PROXY_BASE, ELECTRIC_SHAPE_TABLES, createShapeUrl } from './config';
import { createNodesCollection, createProjectsCollection, createNodeProjectsCollection } from './collections';

describe('electric bridge', () => {
  it('config is importable from remote-frontend', () => {
    expect(typeof ELECTRIC_PROXY_BASE).toBe('string');
    expect(typeof ELECTRIC_SHAPE_TABLES).toBe('object');
    expect(ELECTRIC_SHAPE_TABLES.nodes).toBeDefined();
    expect(typeof createShapeUrl).toBe('function');
  });
  it('existing collections are importable', () => {
    expect(typeof createNodesCollection).toBe('function');
    expect(typeof createProjectsCollection).toBe('function');
    expect(typeof createNodeProjectsCollection).toBe('function');
  });
});