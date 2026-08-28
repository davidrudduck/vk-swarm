/**
 * Electric SQL Collections
 *
 * This module provides TanStack DB collections backed by Electric SQL shapes.
 * Each collection syncs data from the backend PostgreSQL database in real-time.
 *
 * The hive Electric proxy exposes exactly one shape — `shared_tasks`
 * (crates/remote/src/routes/electric_proxy.rs). Collection factories for
 * other tables were removed at the 2026-08-28 close review because their
 * shape URLs 404 against the real hive; see the decisions-ledger.
 */

import { createCollection } from '@tanstack/react-db';
import { electricCollectionOptions } from '@tanstack/electric-db-collection';
import { createShapeUrl } from './config';

/**
 * Base row type that satisfies Row<unknown> constraint.
 * All Electric types must extend this to work with TanStack DB.
 */
type ElectricRow = Record<string, unknown>;

/**
 * Shared-task row streamed by the hive Electric proxy.
 *
 * The authoritative schema lives in the hive's PostgreSQL `shared_tasks`
 * table (not this repo's node migrations), so only the columns the proxy
 * contract guarantees are typed; everything else flows through the
 * ElectricRow index signature.
 */
export type ElectricSharedTask = ElectricRow & {
  id: string;
  organization_id: string;
};

/**
 * Configuration type for Electric collection options.
 * Used for type inference in tests.
 */
export interface ElectricCollectionConfig<T> {
  shapeOptions: {
    url: string;
  };
  getKey: (item: T) => string | number;
}

/**
 * Create a collection for shared tasks.
 * Syncs the organization-scoped shared-task rows from the hive.
 *
 * @returns TanStack DB collection for shared_tasks
 */
export function createSharedTasksCollection() {
  return createCollection(
    electricCollectionOptions<ElectricSharedTask>({
      shapeOptions: {
        url: createShapeUrl('shared_tasks'),
      },
      getKey: (item) => item.id,
    })
  );
}
