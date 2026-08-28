import { describe, it, expect, vi, beforeEach } from 'vitest';

// Mock the TanStack DB modules before importing collections
vi.mock('@tanstack/react-db', () => ({
  createCollection: vi.fn((config) => ({
    ...config,
    _isMockCollection: true,
  })),
}));

vi.mock('@tanstack/electric-db-collection', () => ({
  electricCollectionOptions: vi.fn((config) => ({
    ...config,
    _isElectricConfig: true,
  })),
}));

// Import after mocks are set up
import {
  createSharedTasksCollection,
  type ElectricCollectionConfig,
} from './collections';
import { createCollection } from '@tanstack/react-db';
import { electricCollectionOptions } from '@tanstack/electric-db-collection';

describe('Electric Collections', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('createSharedTasksCollection', () => {
    it('creates a collection with the shared_tasks shape URL', () => {
      const collection = createSharedTasksCollection();

      expect(electricCollectionOptions).toHaveBeenCalledWith(
        expect.objectContaining({
          shapeOptions: expect.objectContaining({
            url: '/v1/shape/shared_tasks',
          }),
        })
      );
      expect(createCollection).toHaveBeenCalled();
      expect(collection).toBeDefined();
    });

    it('uses id as the key extractor', () => {
      createSharedTasksCollection();

      const config = (electricCollectionOptions as ReturnType<typeof vi.fn>)
        .mock.calls[0][0] as ElectricCollectionConfig<{ id: string }>;
      expect(config.getKey({ id: 'task-uuid' })).toBe('task-uuid');
    });
  });
});
