# /dr:document — hive-redesign Documentation Reconciliation

**Date:** 2026-07-03
**Target:** `vk-swarm-hive-redesign-p47` branch, repo `/data/Code/vk-swarm`
**Mode:** Maintain (docs exist, reconciling against hive-redesign code)

## Component classified

- **Type:** hybrid/multi-role (background service + library + user-facing sync)
- **Audiences:** user + admin + developer

## Docs convention detected

- **Topic tree** (`docs/architecture/`, `docs/core-features/`, `docs/features/`, `docs/development/`, `docs/configuration-customisation/`, `docs/api/`)
- Mintlify-style site config at `docs/docs.json`

## Tier-1 anchor reconciliation (mechanical)

No version/config/env anchors required correction — the hive-redesign did not change any config keys, env vars, or service names. All changes were prose-level (architectural description updates).

## Tier-2 prose staleness — sections rewritten

### 1. `docs/architecture/swarm-sync.mdx` (developer-facing, PRIMARY target)

| Section | Lines | Change |
|---------|-------|--------|
| Guarded Write Sites | 121-135 | Removed legacy `handle_task_status` and `handle_task_sync` paths; now documents only `handle_op_batch_apply` as the single write site |
| Unknown Wire Values | 137-143 | Removed `handle_task_sync` path; documented round-5 CF1 SKIP+ADVANCE invariant |
| Data Model Overview | 152-192 | Rewrote diagram: `swarm_tasks` → `shared_tasks`, added `node_outbox`, `node_op_log`, `node_task_assignments`, `activity` tables |
| Tasks section | 376-388 | Replaced ElectricSQL reference with WS activity stream (ADR-0007/SC7) |
| Sync Protocols | 400-532 | COMPLETELY REWRITTEN — replaced old WS message enums with actual variants from CONTRACT.md §A (`OpBatch`/`OpAck`/`LeaseHeartbeat`/`LeaseGrant`/`LeaseRevoked`/`Digest`/`DigestResult`); replaced old Task Sync Flow with op-batch/ack flow; replaced old Execution Log Sync (HiveSyncService polling) with durable outbox model |
| Offline Handling | 619-657 | Updated for durable outbox model, digest anti-entropy on reconnect, SKIP+ADVANCE for stale-token ops |
| Migration Notes | 699-716 | Replaced ElectricSQL migration notes with hive-redesign (ADR-0007) notes |
| Merge Operations SQL | 556 | `swarm_tasks` → `shared_tasks` |

### 2. `docs/swarm-hive-setup.mdx` (admin-facing)

| Section | Lines | Change |
|---------|-------|--------|
| Sync table | 318 | Tasks (hive-created): `ElectricSQL + WebSocket` → `WS activity stream (ADR-0007)`; Tasks (locally-created): `WebSocket (TaskSync)` → `Durable outbox → OpBatch/OpAck`; Task Attempts/Execution Processes: `HiveSyncService (background)` → `Durable outbox → OpBatch` |

### 3. `docs/architecture/db/database-overview.mdx` (developer-facing)

| Section | Lines | Change |
|---------|-------|--------|
| Architecture diagram | 28 | `ElectricSQL (real-time)` → `WS activity stream` |
| Data Flow Summary | 117-118 | Task definitions/status: `ElectricSQL` → `WS activity stream + durable outbox (ADR-0007)`; Execution logs: `WebSocket (streaming)` → `Durable outbox → OpBatch` |
| Compatibility | 146 | `ElectricSQL compatibility for real-time sync` → `Logical replication support for ElectricSQL (non-task sync)` |

### 4. `docs/architecture/db/database-synchronization.mdx` (developer-facing)

| Section | Lines | Change |
|---------|-------|--------|
| Sync methods table | 16 | Split ElectricSQL into `WS Activity Stream` (tasks, ADR-0007) and `ElectricSQL` (projects/nodes/logs) |
| Architecture diagram | 22-50 | Updated to show ElectricSQL for projects only, WS for tasks, added `node_outbox` |
| Synced Tables | 70-74 | Added Method column; `shared_tasks` → `WS activity stream + outbox (ADR-0007)` |
| Shape Configuration | 78-86 | Changed example from `shared_tasks` to `projects` |
| Message Types | 110-148 | Replaced old node→hive/hive→node message enums with actual variants from CONTRACT.md |
| ElectricSQL Sync section | 54 | Header → `ElectricSQL Sync (Non-Task Data)` |
| Sync Service Locations | 303-314 | Removed `ElectricTaskSync` row; updated `HiveClient`/`HiveSyncService`/`NodeRunner` purposes |
| Shape polling | 575-580 | `shared_tasks` → `projects`; header → `ElectricSQL Shape Polling (Non-Task Data)` |

### 5. `docs/architecture/db/functions/sqlite-task.mdx` (developer-facing)

| Section | Lines | Change |
|---------|-------|--------|
| Source references | 159, 426, 457, 484 | `services/electric_sync.rs` → `services/share/processor.rs` (WS activity stream, ADR-0007) |

### 6. `docs/architecture/db/functions/sqlite-supporting.mdx` (developer-facing)

| Section | Lines | Change |
|---------|-------|--------|
| Shared Tasks | 267-284 | `Being replaced by ElectricSQL shape subscriptions` → `Replaced by WS activity stream (ADR-0007); task sync now flows through durable outbox + op-batch/ack model` |

### 7. `docs/architecture/db/sqlite-local-schema.mdx` (developer-facing)

| Section | Lines | Change |
|---------|-------|--------|
| shared_tasks | 265 | `being replaced by ElectricSQL` → `Task sync now uses WS activity stream + durable outbox (ADR-0007)` |
| node_task_assignments | 288 | `Being replaced by ElectricSQL-based sync` → `Replaced by WS activity stream + durable outbox (ADR-0007)` |
| node_local_projects | 324 | `Will sync via ElectricSQL shapes` → `Will sync via ElectricSQL shapes (non-task data)` |

### 8. `docs/architecture/db/functions/postgresql-tasks.mdx` (developer-facing)

| Section | Lines | Change |
|---------|-------|--------|
| Intro | 8 | `nodes sync with via ElectricSQL` → `nodes sync with via the WS activity stream (ADR-0007)` |
| Soft delete | 253 | `ElectricSQL syncs deletion to nodes` → `WS activity stream syncs soft-delete to nodes (ADR-0007)` |
| ElectricSQL Integration section | 288-308 | Replaced entire section: removed `electric_sync_table('public', 'shared_tasks')` and shape subscription; replaced with WS activity stream description + note that non-task data still uses ElectricSQL |

## Sections verified as NOT stale (no changes needed)

- `docs/architecture/db/functions/sqlite-log-entry.mdx` — log entries still use ElectricSQL (non-task data, accurate)
- `docs/architecture/db/functions/postgresql-projects.mdx` lines 201-226 — ElectricSQL Integration for projects (FINE, projects still use ElectricSQL)
- `docs/architecture/db/postgresql-hive-schema.mdx` lines 463-472 — ElectricSQL configuration (FINE, ElectricSQL still exists for non-task tables)
- `docs/architecture/swarm-sync.mdx` lines 76-119 (Lease Fencing & Status Guards), 194-276 (Entity Linking), 659-698 (Performance/Security), 732-1119 (Archive Proxy, Orphaned Task Detection, etc.)

## Anchor classes

| Class | Status |
|-------|--------|
| Version / blurb | Not applicable (no version changes in hive-redesign) |
| Dependency / compatibility target | Not applicable |
| Config schema | Not applicable (no config key changes) |
| Environment variables | Not applicable (no env var changes) |
| Service / unit identity | Not applicable (no service name changes) |

## Unverified sections

None — all changed sections had clear source backing in the hive-redesign code (CONTRACT.md, ADR-0007, session.rs, node_outbox.rs, etc.).

## Placement

All docs placed in the existing topic tree convention. No new files created; all edits were in-place updates to existing docs.
