import { useLiveQuery } from '@tanstack/react-db';
import {
  createTaskAssignmentsCollection,
  createNodesCollection,
  createProjectsCollection,
  type ElectricTaskAssignment,
} from '@/lib/electric';

// Reserved for task 306 (TaskDetail panel)
// import {
//   createTaskOutputLogsCollection,
//   createTaskProgressEventsCollection,
// } from '@/lib/electric';

const assignmentsCollection = createTaskAssignmentsCollection();
const nodesCollection = createNodesCollection();
const projectsCollection = createProjectsCollection();

const STATUS_COLUMNS = ['pending', 'in_progress', 'completed', 'failed'] as const;

export function TasksBoard() {
  const { data: assignments = [] } = useLiveQuery(assignmentsCollection);
  const { data: nodes = [] } = useLiveQuery(nodesCollection);
  const { data: projects = [] } = useLiveQuery(projectsCollection);

  const nodeNames = new Map(nodes.map((n: { id: string; name: string }) => [n.id, n.name]));
  const projectNames = new Map(projects.map((p: { id: string; name: string }) => [p.id, p.name]));

  const byStatus = new Map<string, ElectricTaskAssignment[]>();
  for (const status of STATUS_COLUMNS) byStatus.set(status, []);
  for (const a of assignments) {
    const bucket = byStatus.get(a.execution_status) ?? byStatus.get('pending');
    bucket?.push(a);
  }

  return (
    <div className="flex gap-4">
      {STATUS_COLUMNS.map((status) => (
        <div key={status} className="flex-1">
          <h2 className="text-lg font-semibold capitalize">{status.replace('_', ' ')}</h2>
          <ul>
            {(byStatus.get(status) ?? []).map((a) => (
              <li key={a.id} className="border p-2 my-2">
                <div>task {a.task_id}</div>
                <div>{nodeNames.get(a.node_id) ?? a.node_id}</div>
                <div>{projectNames.get(a.node_project_id) ?? a.node_project_id}</div>
              </li>
            ))}
          </ul>
        </div>
      ))}
    </div>
  );
}

export default TasksBoard;