import { useLiveQuery } from '@tanstack/react-db';
import { useState } from 'react';
import {
  createTaskAssignmentsCollection,
  createTaskOutputLogsCollection,
  createTaskProgressEventsCollection,
  createNodesCollection,
  createProjectsCollection,
  type ElectricTaskAssignment,
} from '@/lib/electric';

const assignmentsCollection = createTaskAssignmentsCollection();
const outputLogsCollection = createTaskOutputLogsCollection();
const progressEventsCollection = createTaskProgressEventsCollection();
const nodesCollection = createNodesCollection();
const projectsCollection = createProjectsCollection();

const STATUS_COLUMNS = ['pending', 'in_progress', 'completed', 'failed'] as const;

export function TasksBoard() {
  const { data: assignments = [] } = useLiveQuery(assignmentsCollection);
  const { data: nodes = [] } = useLiveQuery(nodesCollection);
  const { data: projects = [] } = useLiveQuery(projectsCollection);
  const [selectedAssignmentId, setSelectedAssignmentId] = useState<string | null>(null);

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
      <div className="flex gap-4 flex-1">
        {STATUS_COLUMNS.map((status) => (
          <div key={status} className="flex-1">
            <h2 className="text-lg font-semibold capitalize">{status.replace('_', ' ')}</h2>
            <ul>
              {(byStatus.get(status) ?? []).map((a) => (
                <li key={a.id} className="border p-2 my-2" onClick={() => setSelectedAssignmentId(a.id)}>
                  <div>task {a.task_id}</div>
                  <div>{nodeNames.get(a.node_id) ?? a.node_id}</div>
                  <div>{projectNames.get(a.node_project_id) ?? a.node_project_id}</div>
                </li>
              ))}
            </ul>
          </div>
        ))}
      </div>
      {selectedAssignmentId && (
        <div className="w-80 border-l p-4">
          <TaskDetail assignmentId={selectedAssignmentId} />
        </div>
      )}
    </div>
  );
}

export function TaskDetail({ assignmentId }: { assignmentId: string }) {
  const { data: logs = [] } = useLiveQuery(outputLogsCollection);
  const { data: events = [] } = useLiveQuery(progressEventsCollection);

  const assignmentLogs = logs.filter((l: { assignment_id: string }) => l.assignment_id === assignmentId);
  const assignmentEvents = events.filter((e: { assignment_id: string }) => e.assignment_id === assignmentId);

  if (assignmentLogs.length === 0 && assignmentEvents.length === 0) {
    return <div className="text-gray-500">No activity yet.</div>;
  }

  return (
    <div className="space-y-4">
      <section>
        <h3 className="font-semibold">Progress events</h3>
        <ul>
          {assignmentEvents.map((e: { id: string; event_type: string; message?: string | null }) => (
            <li key={e.id}>
              <span className="font-mono text-sm">{e.event_type}</span>
              {e.message ? ` — ${e.message}` : ''}
            </li>
          ))}
        </ul>
      </section>
      <section>
        <h3 className="font-semibold">Output logs</h3>
        <ul>
          {assignmentLogs.map((l: { id: string; output_type: string; content: string }) => (
            <li key={l.id}>
              <span className="font-mono text-xs uppercase">{l.output_type}</span>
              <pre className="whitespace-pre-wrap">{l.content}</pre>
            </li>
          ))}
        </ul>
      </section>
    </div>
  );
}

export default TasksBoard;