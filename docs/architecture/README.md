# Architecture Documentation

This folder contains technical architecture documentation for Vibe Kanban's internal systems. These documents are intended for developers working on the codebase.

## Contents

| Document | Description |
|----------|-------------|
| [Database Documentation](./db/README.md) | Complete database architecture, schema, and function reference |
| [Frontend Task Sorting](./frontend-sorting.mdx) | Task sorting strategy, implementation, and state management |
| [Node API Key Component](./node-api-key-component.mdx) | Architecture and implementation of the NodeApiKeySection component |

## Quick Reference

**Local Node (SQLite)**: Stores projects, tasks, execution history, and logs for a single machine. Each node operates independently and can work offline.

**Central Hive (PostgreSQL)**: Manages organizations, users, shared tasks, and node coordination. Enables distributed task management across multiple machines.

**Synchronization**: Uses ElectricSQL for real-time data sync and WebSocket for task assignment and execution events.

## Related Documentation

- [Swarm/Hive Setup Guide](../swarm-hive-setup.mdx) - How to configure nodes to connect to a hive
- [AGENTS.md](/AGENTS.md) - Development conventions and API patterns
