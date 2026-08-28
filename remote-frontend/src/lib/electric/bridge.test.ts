import { describe, it, expect } from 'vitest';
import {
  ELECTRIC_PROXY_BASE,
  ELECTRIC_SHAPE_TABLES,
  createShapeUrl,
  createSharedTasksCollection,
} from './index';

// Bridge smoke (SC8 lineage): the module's public barrel must stay importable
// as a unit and expose the single-table contract the hive proxy actually
// serves (/v1/shape/shared_tasks — crates/remote/src/routes/electric_proxy.rs).
// Deep contracts live in config.test.ts / collections.test.ts / electric.test.ts.
describe('electric bridge', () => {
  it('barrel exposes the single-table config + collection surface', () => {
    expect(ELECTRIC_PROXY_BASE).toBe('/v1/shape');
    expect(Object.keys(ELECTRIC_SHAPE_TABLES)).toEqual(['shared_tasks']);
    expect(createShapeUrl('shared_tasks')).toBe('/v1/shape/shared_tasks');
    expect(typeof createSharedTasksCollection).toBe('function');
  });
});
