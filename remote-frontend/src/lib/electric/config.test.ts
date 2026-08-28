import { describe, it, expect } from 'vitest';
import {
  getElectricBaseUrl,
  createShapeUrl,
  ELECTRIC_SHAPE_TABLES,
  type ElectricShapeTable,
} from './config';

describe('Electric Config', () => {
  describe('getElectricBaseUrl', () => {
    it('returns the correct base URL for Electric proxy', () => {
      const baseUrl = getElectricBaseUrl();
      expect(baseUrl).toBe('/v1/shape');
    });
  });

  describe('ELECTRIC_SHAPE_TABLES', () => {
    it('contains ONLY the table the hive proxy actually serves', () => {
      // The hive proxy routes exactly one shape: GET /v1/shape/shared_tasks
      // (crates/remote/src/routes/electric_proxy.rs). Advertising any other
      // table here would produce 404 shape URLs against the real hive.
      expect(Object.keys(ELECTRIC_SHAPE_TABLES)).toEqual(['shared_tasks']);
      expect(ELECTRIC_SHAPE_TABLES.shared_tasks.table).toBe('shared_tasks');
    });
  });

  describe('createShapeUrl', () => {
    it('creates URL for the shared_tasks shape', () => {
      const url = createShapeUrl('shared_tasks');
      expect(url).toBe('/v1/shape/shared_tasks');
    });

    it('throws error for tables the proxy does not serve', () => {
      // 'nodes' was removed at the 2026-08-28 close review: the proxy has no
      // /v1/shape/nodes route, so this URL would 404 against the real hive.
      expect(() =>
        createShapeUrl('nodes' as ElectricShapeTable)
      ).toThrow('Unknown Electric shape table: nodes');
    });

    it('throws error for invalid table name', () => {
      expect(() =>
        createShapeUrl('invalid_table' as ElectricShapeTable)
      ).toThrow('Unknown Electric shape table: invalid_table');
    });
  });
});
