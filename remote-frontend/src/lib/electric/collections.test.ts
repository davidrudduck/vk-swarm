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
  createNodesCollection,
  createProjectsCollection,
  createNodeProjectsCollection,
  createTaskAssignmentsCollection,
  createTaskOutputLogsCollection,
  createTaskProgressEventsCollection,
  type ElectricCollectionConfig,
} from './collections';
import { createCollection } from '@tanstack/react-db';
import { electricCollectionOptions } from '@tanstack/electric-db-collection';

describe('Electric Collections', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('createNodesCollection', () => {
    it('creates a collection with correct shape URL', () => {
      const collection = createNodesCollection();

      expect(electricCollectionOptions).toHaveBeenCalledWith(
        expect.objectContaining({
          shapeOptions: expect.objectContaining({
            url: '/v1/shape/nodes',
          }),
        })
      );
      expect(createCollection).toHaveBeenCalled();
      expect(collection).toBeDefined();
    });

    it('uses id as the key extractor', () => {
      createNodesCollection();

      const config = (electricCollectionOptions as ReturnType<typeof vi.fn>)
        .mock.calls[0][0] as ElectricCollectionConfig<{ id: string }>;
      expect(config.getKey({ id: 'node-uuid' })).toBe('node-uuid');
    });
  });

  describe('createProjectsCollection', () => {
    it('creates a collection with correct shape URL', () => {
      const collection = createProjectsCollection();

      expect(electricCollectionOptions).toHaveBeenCalledWith(
        expect.objectContaining({
          shapeOptions: expect.objectContaining({
            url: '/v1/shape/projects',
          }),
        })
      );
      expect(createCollection).toHaveBeenCalled();
      expect(collection).toBeDefined();
    });

    it('uses id as the key extractor', () => {
      createProjectsCollection();

      const config = (electricCollectionOptions as ReturnType<typeof vi.fn>)
        .mock.calls[0][0] as ElectricCollectionConfig<{ id: string }>;
      expect(config.getKey({ id: 'project-uuid' })).toBe('project-uuid');
    });
  });

  describe('createNodeProjectsCollection', () => {
    it('creates a collection with correct shape URL', () => {
      const collection = createNodeProjectsCollection();

      expect(electricCollectionOptions).toHaveBeenCalledWith(
        expect.objectContaining({
          shapeOptions: expect.objectContaining({
            url: '/v1/shape/node_projects',
          }),
        })
      );
      expect(createCollection).toHaveBeenCalled();
      expect(collection).toBeDefined();
    });

    it('uses id as the key extractor', () => {
      createNodeProjectsCollection();

      const config = (electricCollectionOptions as ReturnType<typeof vi.fn>)
        .mock.calls[0][0] as ElectricCollectionConfig<{ id: string }>;
      expect(config.getKey({ id: 'node-project-uuid' })).toBe(
        'node-project-uuid'
      );
    });
  });

  describe('createTaskAssignmentsCollection', () => {
    it('creates a collection with correct shape URL', () => {
      const collection = createTaskAssignmentsCollection();

      expect(electricCollectionOptions).toHaveBeenCalledWith(
        expect.objectContaining({
          shapeOptions: expect.objectContaining({
            url: '/v1/shape/node_task_assignments',
          }),
        })
      );
      expect(createCollection).toHaveBeenCalled();
      expect(collection).toBeDefined();
    });

    it('uses id as the key extractor', () => {
      createTaskAssignmentsCollection();

      const config = (electricCollectionOptions as ReturnType<typeof vi.fn>)
        .mock.calls[0][0] as ElectricCollectionConfig<{ id: string }>;
      expect(config.getKey({ id: 'assignment-uuid' })).toBe('assignment-uuid');
    });
  });

  describe('createTaskOutputLogsCollection', () => {
    it('creates a collection with correct shape URL', () => {
      const collection = createTaskOutputLogsCollection();

      expect(electricCollectionOptions).toHaveBeenCalledWith(
        expect.objectContaining({
          shapeOptions: expect.objectContaining({
            url: '/v1/shape/node_task_output_logs',
          }),
        })
      );
      expect(createCollection).toHaveBeenCalled();
      expect(collection).toBeDefined();
    });

    it('uses id as the key extractor', () => {
      createTaskOutputLogsCollection();

      const config = (electricCollectionOptions as ReturnType<typeof vi.fn>)
        .mock.calls[0][0] as ElectricCollectionConfig<{ id: string }>;
      expect(config.getKey({ id: 'log-1' })).toBe('log-1');
    });
  });

  describe('createTaskProgressEventsCollection', () => {
    it('creates a collection with correct shape URL', () => {
      const collection = createTaskProgressEventsCollection();

      expect(electricCollectionOptions).toHaveBeenCalledWith(
        expect.objectContaining({
          shapeOptions: expect.objectContaining({
            url: '/v1/shape/node_task_progress_events',
          }),
        })
      );
      expect(createCollection).toHaveBeenCalled();
      expect(collection).toBeDefined();
    });

    it('uses id as the key extractor', () => {
      createTaskProgressEventsCollection();

      const config = (electricCollectionOptions as ReturnType<typeof vi.fn>)
        .mock.calls[0][0] as ElectricCollectionConfig<{ id: string }>;
      expect(config.getKey({ id: 'event-1' })).toBe('event-1');
    });
  });
});
