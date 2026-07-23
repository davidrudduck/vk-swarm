import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { BoardView, type TaskRow as BoardTaskRow } from '@/ui/board';
import { TaskDrawer } from '@/ui/panels';
import type { TaskStatus } from '@/components/board';
import { tasksApi, type TaskActivity } from '@/lib/api/tasks';
import { organizationsApi } from '@/lib/api/organizations';
import { swarmProjectsApi } from '@/lib/api/swarmProjects';
import { ErrorBanner } from '@/components/ui/ErrorBanner';

/**
 * Row shape rendered by both `BoardView` (`node?: string`) and `TaskDrawer`
 * (`node: string`, required). Narrowing `node` to always-`string` here keeps a
 * single object assignable to both without an unsafe cast at the render site.
 */
interface Row extends BoardTaskRow {
  node: string;
}

/**
 * Wires `BoardView` + `TaskDrawer` to live data.
 *
 * REST is the primary data source (Electric collections are enhancement-only
 * per the task 306 known-gap ledger entry). Fetch chain: organizations ->
 * first org -> swarm projects for that org -> first project -> bulk tasks for
 * that project. This "pick the first org/project" selection is a placeholder
 * until an org/project switcher lands (ledgered decision, task 308).
 */
export function BoardPage() {
  const [selected, setSelected] = useState<{ task: Row; status: TaskStatus } | null>(null);

  const orgsQ = useQuery({ queryKey: ['orgs'], queryFn: organizationsApi.list });
  const orgId = orgsQ.data?.[0]?.id;

  const projectsQ = useQuery({
    queryKey: ['projects', orgId],
    queryFn: () => swarmProjectsApi.list(orgId!),
    enabled: !!orgId,
  });
  const projectId = projectsQ.data?.[0]?.id;

  const tasksQ = useQuery({
    queryKey: ['tasks', 'bulk', projectId],
    queryFn: () => tasksApi.bulk(projectId!),
    enabled: !!projectId,
  });
  const activities = tasksQ.data?.tasks ?? [];
  const columns = groupByStatus(activities);

  // Gate the error banner on ANY query in the chained orgs -> projects -> tasks
  // fetch: if an upstream query fails, the `enabled`-gated downstream query never
  // runs (it stays *pending*, not *error*), so checking only `tasksQ` would render
  // an authoritative empty board on an orgs/projects failure. (Codex review finding.)
  const isError = orgsQ.isError || projectsQ.isError || tasksQ.isError;

  return (
    <>
      {isError && <ErrorBanner message="Failed to load tasks. Check your connection and try again." />}
      <BoardView
        columns={columns}
        onAdd={() => {}}
        onOpen={(task, status) => setSelected({ task: task as Row, status })}
        selectedId={selected?.task.id}
      />
      <TaskDrawer
        task={selected?.task ?? null}
        status={selected?.status ?? 'todo'}
        onClose={() => setSelected(null)}
      />
    </>
  );
}

/**
 * Maps `TaskActivity[]` (the real `BulkSharedTasksResponse.tasks` shape --
 * `{ task, user }[]`, not a flat `Task[]`) into the `Record<TaskStatus, Row[]>`
 * shape `BoardView` expects.
 *
 * Field-gap (ledgered, task 308): the real hive `Task` interface
 * (`crates/remote/src/db/tasks.rs`) has no `source_node_id` or `labels`
 * fields. `node` falls back through `owner_name` -> `executing_node_id` ->
 * `owner_node_id`; `labels` is always `[]` until the backend adds label
 * support.
 */
const KNOWN_STATUSES: readonly TaskStatus[] = ['todo', 'inprogress', 'inreview', 'done', 'cancelled'];

/**
 * Normalizes a raw wire `task.status` string to the frontend `TaskStatus` union.
 *
 * The remote crate's `TaskStatus` serializes as **kebab-case**
 * (`crates/remote/src/db/tasks.rs:22-31` — `#[serde(rename_all = "kebab-case")]`),
 * emitting `"in-progress"` / `"in-review"` on the `/v1/tasks/bulk` wire, while the
 * frontend `TaskStatus` union (`components/board/StatusBadge.tsx`) uses the
 * hyphen-less `"inprogress"` / `"inreview"`. Without this bridge, `"in-progress"`
 * / `"in-review"` tasks fail the `status in out` gate in `groupByStatus` and are
 * silently dropped from every board column (adversarial review F1, CRITICAL).
 *
 * Fixed client-side (the Rust enum is authoritative and shared with other
 * consumers, so it is intentionally left untouched). Known values pass through;
 * an **unknown** status is dropped and logged via `console.warn` (decision: an
 * unrecognized status is more likely a contract drift to surface loudly than a
 * real column, and bucketing it into an arbitrary column would mislabel the task).
 * The `console.warn` keeps the drop non-silent so a future backend status is
 * caught in review/console rather than vanishing quietly.
 *
 * @returns the normalized `TaskStatus`, or `null` if the status is unknown (drop).
 */
export function normalizeStatus(raw: string | null | undefined): TaskStatus | null {
  switch (raw) {
    case 'in-progress':
      return 'inprogress';
    case 'in-review':
      return 'inreview';
  }
  const status = raw ?? 'todo';
  if ((KNOWN_STATUSES as readonly string[]).includes(status)) {
    return status as TaskStatus;
  }
  console.warn(`[BoardPage] Unknown task status "${status}" — task dropped from board`);
  return null;
}

function groupByStatus(activities: TaskActivity[]): Record<TaskStatus, Row[]> {
  const out: Record<TaskStatus, Row[]> = { todo: [], inprogress: [], inreview: [], done: [], cancelled: [] };
  for (const { task } of activities) {
    const status = normalizeStatus(task.status);
    if (status !== null) {
      out[status].push({
        id: task.id,
        title: task.title,
        description: task.description ?? undefined,
        node: task.owner_name ?? task.executing_node_id ?? task.owner_node_id ?? '',
        labels: [],
      });
    }
  }
  return out;
}
