import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { BoardView, type TaskRow as BoardTaskRow } from '@/ui/board';
import { TaskDrawer } from '@/ui/panels';
import type { TaskStatus } from '@/components/board';
import { tasksApi, type TaskActivity } from '@/lib/api/tasks';
import { organizationsApi } from '@/lib/api/organizations';
import { swarmProjectsApi } from '@/lib/api/swarmProjects';

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

  return (
    <>
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
function groupByStatus(activities: TaskActivity[]): Record<TaskStatus, Row[]> {
  const out: Record<TaskStatus, Row[]> = { todo: [], inprogress: [], inreview: [], done: [], cancelled: [] };
  for (const { task } of activities) {
    const status = (task.status ?? 'todo') as TaskStatus;
    if (status in out) {
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
