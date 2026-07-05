---
id: "306"
phase: 3
title: Add TaskDetail panel showing attempts (output logs) + progress events
status: done
depends_on: ["304", "305"]
parallel: false
conflicts_with: []
files:
  - remote-frontend/src/pages/Tasks.tsx
  - remote-frontend/src/pages/Tasks.test.tsx
irreversible: false
scope_test: "remote-frontend/src/pages/Tasks.test.tsx"
allowed_change: edit
covers_criteria: [SC2]
---
## Failing test (write first)

Append to `remote-frontend/src/pages/Tasks.test.tsx`, in the existing `describe('TasksBoard', ...)` block or a new `describe('TaskDetail', ...)` block:

```tsx
import { TasksBoard, TaskDetail } from './Tasks';

// extend the mocks above to include output logs + progress events for assignment a1
// (add to the createTaskOutputLogsCollection mock):
//   { _mockRows: [
//     { id: 'log-1', assignment_id: 'a1', output_type: 'stdout', content: 'Running...', timestamp: '2026-07-04T00:01:00Z', created_at: '2026-07-04T00:01:00Z', execution_process_id: null },
//   ] }
// (add to the createTaskProgressEventsCollection mock):
//   { _mockRows: [
//     { id: 'evt-1', assignment_id: 'a1', event_type: 'agent_started', message: 'claude started', metadata: null, timestamp: '2026-07-04T00:00:30Z', created_at: '2026-07-04T00:00:30Z' },
//   ] }

describe('TaskDetail', () => {
  it('renders output logs and progress events for the selected assignment', () => {
    render(<TaskDetail assignmentId="a1" />);
    // Output logs
    expect(screen.getByText('Running...')).toBeInTheDocument();
    // Progress events
    expect(screen.getByText('claude started')).toBeInTheDocument();
    expect(screen.getByText(/agent_started/i)).toBeInTheDocument();
  });

  it('renders an empty state when no logs or events exist', () => {
    render(<TaskDetail assignmentId="a-nonexistent" />);
    expect(screen.getByText(/no logs yet|no activity/i)).toBeInTheDocument();
  });
});
```

Test fails red — `TaskDetail` not yet exported.

## Change

### File: `remote-frontend/src/pages/Tasks.tsx`

**Sibling alignment:** Read the existing `TasksBoard` component in the same file (written in 305). It uses `useCollection` against the 3 task collections. `TaskDetail` MUST reuse the same module-scope collection instances (don't re-create them inside the component) so the board and detail panel share data. Justify any divergence in the decisions ledger.

Append after `TasksBoard`:

```tsx
export function TaskDetail({ assignmentId }: { assignmentId: string }) {
  const { data: logs } = useCollection(outputLogsCollection);
  const { data: events } = useCollection(progressEventsCollection);

  const assignmentLogs = logs.filter((l) => l.assignment_id === assignmentId);
  const assignmentEvents = events.filter((e) => e.assignment_id === assignmentId);

  if (assignmentLogs.length === 0 && assignmentEvents.length === 0) {
    return <div className="text-gray-500">No activity yet.</div>;
  }

  return (
    <div className="space-y-4">
      <section>
        <h3 className="font-semibold">Progress events</h3>
        <ul>
          {assignmentEvents.map((e) => (
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
          {assignmentLogs.map((l) => (
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
```

Wire the board to open the detail panel: in `TasksBoard`, wrap each `<li>` row with an `onClick` that sets a `selectedAssignmentId` state, and render `<TaskDetail assignmentId={selectedAssignmentId} />` in a side panel. (Add `useState` import.)

### File: `remote-frontend/src/pages/Tasks.test.tsx`

Append the `TaskDetail` describe block + extend the mocks per the failing-test section above.

## Allowed moves
- Add `TaskDetail` export to `remote-frontend/src/pages/Tasks.tsx`.
- Wire `TasksBoard` to render `TaskDetail` on row click (add `useState`).
- Extend the test file with the `TaskDetail` tests + mock rows.
- No other file. No server changes.

## STOP triggers
- The `outputLogsCollection` / `progressEventsCollection` module-scope instances from 305 are not in scope — HALT; restructure so board + detail share the same collection instances.

## Manual verification (record in decisions-ledger)
```bash
cd remote-frontend && npx vitest run src/pages/Tasks.test.tsx
cd remote-frontend && npx tsc --noEmit
cd remote-frontend && npm run lint
```
All exit 0.

## Done when
`WAI_TYPECHECK_CMD="cd remote-frontend && npx tsc --noEmit" WAI_TEST_CMD="cd remote-frontend && npx vitest run src/pages/Tasks.test.tsx" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-hive-ui 306` exits 0