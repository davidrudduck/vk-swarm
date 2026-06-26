---
id: "405"
phase: 4
title: Read-only Hive sync-status view (extend /api/database/sync-status + Settings card)
status: ready
depends_on: []
parallel: false
conflicts_with: []
files:
  - crates/server/src/routes/database.rs
  - shared/types.ts
  - frontend/src/lib/api/database.ts
  - frontend/src/components/settings/HiveSyncStatusCard.tsx
  - frontend/src/pages/settings/SystemSettings.tsx
irreversible: false
scope_test: "N/A"
allowed_change: mixed
covers_criteria: [SC5]
---
## Failing test (write first)

`N/A — covered by: cargo check -p server + frontend tsc --noEmit/lint`. The new field plumbing and a
read-only card have no cheap behavioral unit test (the handler needs a full `DeploymentImpl`; the card
is presentational). Verified by `## Manual verification` (field present in JSON; card renders) + the
gate's compile/type-check.

## Change

**A. `crates/server/src/routes/database.rs` — extend `SyncStatusResponse` + populate it.**
- Anchor: `struct SyncStatusResponse` (L98) — add three optional fields after `node_id` (L110):
```bash
    /// Current node ID (if connected to Hive).
    pub node_id: Option<Uuid>,
    /// Hive WebSocket URL this node is configured to sync with (from VK_HIVE_URL), if any.
    pub hive_url: Option<String>,
    /// Human-readable node name (from VK_NODE_NAME), if configured.
    pub node_name: Option<String>,
    /// Most recent successful sync timestamp across synced entities (NULL = never synced).
    #[ts(type = "Date | null")]
    pub last_synced_at: Option<DateTime<Utc>>,
```

- Anchor: `async fn sync_status` final `Ok(ResponseJson(ApiResponse::success(SyncStatusResponse {`
  (L323). Read `VK_HIVE_URL`/`VK_NODE_NAME` from env (the node runner already loads them from env;
  reading here avoids a `services`-crate accessor). Before the `Ok(...)`, add:
```text
    let hive_url = std::env::var("VK_HIVE_URL").ok();
    let node_name = std::env::var("VK_NODE_NAME").ok();
```

  and compute `last_synced_at` (the spec REQUIRES a real last-synced — breakdown-review R6 — counts
  alone are not it). `execution_processes.hive_synced_at` is the representative highest-volume synced
  surface; add a scalar query (mirror the existing `count_unsynced_*` query style in this handler):
```text
    let last_synced_at = sqlx::query_scalar!(
        r#"SELECT MAX(hive_synced_at) as "last: DateTime<Utc>" FROM execution_processes"#
    )
    .fetch_one(&deployment.db().pool)
    .await
    .unwrap_or(None);
```
  (If review of the handler shows tasks/attempts also carry `hive_synced_at` and a cross-entity MAX is
  wanted, UNION them; the single-table MAX is the minimum that satisfies "last-synced". Record the
  choice in the ledger.)

  Then add `hive_url,`, `node_name,`, `last_synced_at,` to the struct literal:
```text
    Ok(ResponseJson(ApiResponse::success(SyncStatusResponse {
        unsynced_tasks,
        unsynced_attempts,
        unsynced_executions,
        unsynced_logs,
        is_connected,
        node_id,
        hive_url,
        node_name,
        last_synced_at,
    })))
```

**B. `shared/types.ts` — regenerate (do NOT hand-edit).**
- Run `npm run generate-types`. The generated `SyncStatusResponse` (L1415) gains
  `hive_url: string | null`, `node_name: string | null`, and `last_synced_at: Date | null`. List
  `shared/types.ts` in `files:` because the generator rewrites it; the diff must be exactly those three
  added fields.

**C. `frontend/src/lib/api/database.ts` — add a read accessor.**
- Add `SyncStatusResponse` to the `import type { … } from 'shared/types'` block.
- Add to the `databaseApi` object (mirror `getStats`):
```javascript
  /** Get Hive sync status (unsynced counts, connection, configured hive url/node name). */
  getSyncStatus: async (): Promise<SyncStatusResponse> => {
    const res = await makeRequest('/api/database/sync-status');
    return handleApiResponse<SyncStatusResponse>(res, 'fetch sync status');
  },
```

  (Confirm the exact `makeRequest`/`handleApiResponse` call shape against the existing `getStats`
  method in this file before writing — match it.)

**D. `frontend/src/components/settings/HiveSyncStatusCard.tsx` — new read-only card (create).**
- A presentational component: `useQuery({ queryKey: ['hiveSyncStatus'], queryFn: databaseApi.getSyncStatus })`,
  renders a `Card` showing (read-only, no mutations/buttons): connection state (`is_connected`),
  `node_name`, `hive_url`, `node_id`, `last_synced_at` (formatted; "never" when null), and the four
  `unsynced_*` counts. Mirror the `Card`/`CardHeader`/
  `CardContent` usage already in `SystemSettings.tsx`. No inputs, no actions — display only.
- This new component sits beside the existing sibling `frontend/src/components/settings/BackupsSection.tsx`
  (same directory). READ it first (see Sibling alignment) and mirror its `useQuery` + `Card` + `*Api`
  from `@/lib/api` + `shared/types` structure. Do NOT add it to the `settings/index.ts` barrel unless
  `BackupsSection` is also barrel-exported (it is NOT mounted via the barrel — `SystemSettings` imports
  by path); import `HiveSyncStatusCard` by path to match, so `settings/index.ts` stays untouched.

**E. `frontend/src/pages/settings/SystemSettings.tsx` — mount the card.**
- Add `import { HiveSyncStatusCard } from '@/components/settings/HiveSyncStatusCard';`
- Render `<HiveSyncStatusCard />` once inside the existing `return (…)` card stack (e.g. alongside the
  first `<Card>` at L299). One JSX line; do not restructure the page.

## Allowed moves

- Backend: add ONLY the two struct fields + the two env reads + two literal fields in `sync_status`.
  Do NOT touch other handlers or `count_unsynced` queries.
- Regenerate `shared/types.ts` via the generator only.
- Frontend: add ONE api method; create ONE presentational card; add ONE import + ONE JSX mount line.
- Read-only: the card issues NO mutations and exposes NO force-resync/connect controls.
- Do NOT touch the sync layer (SC5d): no `upsert_remote_task`, publisher, WS runner, or
  `remote_*`/`shared_task_id` writer. (Reading `VK_HIVE_URL`/`VK_NODE_NAME` from env is config-read, not
  sync plumbing.)
- Do NOT repurpose/rewrite `SwarmSettings.tsx` here — it pulls in remote `@/components/swarm` sections
  (entangled). The ADR's "repurpose SwarmSettings read-only" is a larger follow-up; this task delivers
  the read-only view as a self-contained Settings card instead. (Discrepancy noted in report.)

## Sibling alignment

Two siblings — read BOTH before writing, list their guards, justify any divergence:

1. **API accessor** — `databaseApi.getStats` in the same `database.ts`. Match its `makeRequest` +
   `handleApiResponse` shape exactly; `getSyncStatus` must not invent a different fetch pattern.
2. **Settings card** — `frontend/src/components/settings/BackupsSection.tsx` (same directory as the new
   card). It embodies the read-from-`*Api`-via-`useQuery`-into-a-`Card` pattern. Read it; list its
   guards (loading state via `Loader2`, `shared/types` typing, `@/lib/api` import). The justified
   divergence: `HiveSyncStatusCard` is strictly READ-ONLY — it omits BackupsSection's mutations
   (`useMutation`, create/delete/restore buttons, `ConfirmDialog`). That omission is the point (ADR-0002
   "keep, read-only"), not an oversight.

## STOP triggers

- `SyncStatusResponse` struct at L98 does not match (fields changed) — re-anchor.
- `npm run generate-types` changes more than the three intended fields in `shared/types.ts` (means an
  unrelated Rust type drifted) — STOP and investigate; do not commit unrelated type churn.
- `databaseApi`/`makeRequest`/`handleApiResponse` have a different shape than `getStats` uses — match
  the real one; do not guess.
- Mounting the card requires restructuring `SystemSettings.tsx` beyond one import + one JSX line — STOP.

## Manual verification (record in decisions-ledger)

1. `cargo check -p server` → 0.
2. `npm run generate-types:check` → 0 (types current after regen) OR confirm the diff is exactly the
   two new fields.
3. `cd frontend && npx tsc --noEmit && npm run lint` → 0.
4. With `VK_HIVE_URL`/`VK_NODE_NAME` set, `curl -s localhost:$BACKEND_PORT/api/database/sync-status`
   shows `hive_url`/`node_name` populated; with them unset, both are `null`. Record the sample JSON.

## Done when

`WAI_TYPECHECK_CMD="cargo check -p server && cd frontend && npx tsc --noEmit" WAI_TEST_CMD="cd frontend && npm run lint" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-node-foundations 405` exits 0

> Trap 1/2: server crate + frontend toolchain. This task ADDS a `query_scalar!(MAX(hive_synced_at))`
> (the `last_synced_at` field), so `query!` must check a live schema — **precondition: export
> `DATABASE_URL=sqlite://<repo>/dev_assets/db.sqlite`** to a migrated dev DB (Trap 2). Do NOT
> `cargo sqlx prepare` here (it churns the tracked `.sqlx` cache the gate rejects; regen is a
> `/wai:close` step). The `#[ts(export)]` change is picked up by `npm run generate-types` (step B);
> `shared/types.ts` diff adds THREE fields (`hive_url`, `node_name`, `last_synced_at`).
