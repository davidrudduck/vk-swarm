import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { renderHook, waitFor } from '@testing-library/react';
import { useNodeLogStream } from './useNodeLogStream';

/**
 * Task 013: the direct raw-log stream must be keyed by the EXECUTION PROCESS
 * id everywhere. The pre-013 hook fetched connection-info without the
 * `execution_process_id` query param and built the direct node URL from the
 * ASSIGNMENT id — a three-way identifier mix (assignment / local attempt /
 * execution process) that the Hive token repair locks to the exact process.
 */

const ASSIGNMENT_ID = '11111111-1111-4111-8111-111111111111';
const PROCESS_ID = '22222222-2222-4222-8222-222222222222';
const CONNECTION_TOKEN = 'test-connection-token';

let fetchMock: ReturnType<typeof vi.fn>;
const wsUrls: string[] = [];
// Sockets created by the hook, in creation order, so tests can drive
// opens/messages by hand (delayed-open race coverage).
const sockets: FakeWebSocket[] = [];
// When true, FakeWebSocket does NOT auto-open; the test calls onopen itself.
let manualOpen = false;

class FakeWebSocket {
  url: string;
  onopen: (() => void) | null = null;
  onerror: (() => void) | null = null;
  onclose: (() => void) | null = null;
  onmessage: ((event: { data: string }) => void) | null = null;

  constructor(url: string) {
    this.url = url;
    wsUrls.push(url);
    sockets.push(this);
    // Resolve as an opened direct connection so no relay fallback fires —
    // unless the test asked to open sockets by hand.
    queueMicrotask(() => {
      if (!manualOpen) this.onopen?.();
    });
  }

  close(): void {}
}

function connectionInfoResponse(
  directUrl: string | null = 'https://node.example.com'
): Response {
  return new Response(
    JSON.stringify({
      assignment_id: ASSIGNMENT_ID,
      node_id: '33333333-3333-4333-8333-333333333333',
      direct_url: directUrl,
      relay_url:
        'https://hive.example.com/v1/nodes/assignments/11111111-1111-4111-8111-111111111111/logs/ws',
      connection_token: CONNECTION_TOKEN,
      expires_at: '2026-08-23T00:00:00Z',
    }),
    { status: 200, headers: { 'content-type': 'application/json' } }
  );
}

describe('useNodeLogStream direct raw-log URL contract', () => {
  beforeEach(() => {
    fetchMock = vi.fn();
    vi.stubGlobal('fetch', fetchMock);
    wsUrls.length = 0;
    sockets.length = 0;
    manualOpen = false;
    vi.stubGlobal('WebSocket', FakeWebSocket);
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('requires both hook arguments at the type level', () => {
    // Optional second param makes Parameters['length'] `1 | 2`; required is `2`.
    type HookArity = Parameters<typeof useNodeLogStream>['length'];
    type Equal<X, Y> =
      (<T>() => T extends X ? 1 : 2) extends <T>() => T extends Y ? 1 : 2
        ? true
        : false;
    const secondArgumentIsRequired: Equal<HookArity, 2> = true;
    expect(secondArgumentIsRequired).toBe(true);
  });

  it('sends execution_process_id on the connection-info request and keys the direct WS by the process id', async () => {
    fetchMock.mockResolvedValue(connectionInfoResponse());

    const { result } = renderHook(() =>
      useNodeLogStream(ASSIGNMENT_ID, PROCESS_ID)
    );

    await waitFor(() => expect(result.current.connectionType).toBe('direct'));

    expect(fetchMock).toHaveBeenCalledTimes(1);
    const requestedUrl = fetchMock.mock.calls[0][0] as string;
    expect(requestedUrl).toBe(
      `/v1/nodes/assignments/${ASSIGNMENT_ID}/connection-info?execution_process_id=${PROCESS_ID}`
    );

    expect(wsUrls).toHaveLength(1);
    const directUrl = wsUrls[0];
    expect(directUrl).toBe(
      `wss://node.example.com/api/execution-processes/${PROCESS_ID}/raw-logs/ws?token=${CONNECTION_TOKEN}`
    );
    // The assignment id must never appear where the process id belongs.
    expect(directUrl).not.toContain(ASSIGNMENT_ID);
  });

  it('never substitutes the assignment id for the process id in either URL', async () => {
    fetchMock.mockResolvedValue(connectionInfoResponse());

    const { result } = renderHook(() =>
      useNodeLogStream(ASSIGNMENT_ID, PROCESS_ID)
    );

    await waitFor(() => expect(result.current.connectionType).toBe('direct'));

    const requestedUrl = fetchMock.mock.calls[0][0] as string;
    // The query param carries the process id, not the assignment id.
    expect(requestedUrl).toContain(`execution_process_id=${PROCESS_ID}`);
    expect(requestedUrl).not.toContain(`execution_process_id=${ASSIGNMENT_ID}`);
    // The path segment before /raw-logs/ws is the process id.
    expect(requestedUrl).toContain(
      `/v1/nodes/assignments/${ASSIGNMENT_ID}/connection-info`
    );
  });

  it('attempts no remote stream while either id is undefined', async () => {
    const { result: onlyAssignment } = renderHook(() =>
      useNodeLogStream(ASSIGNMENT_ID, undefined)
    );
    await waitFor(() =>
      expect(onlyAssignment.current.connectionType).toBe('disconnected')
    );

    const { result: onlyProcess } = renderHook(() =>
      useNodeLogStream(undefined, PROCESS_ID)
    );
    await waitFor(() =>
      expect(onlyProcess.current.connectionType).toBe('disconnected')
    );

    expect(fetchMock).not.toHaveBeenCalled();
    expect(wsUrls).toHaveLength(0);
  });

  it('drops a stale in-flight connect when the execution process id changes mid-fetch', async () => {
    // Regression (PR #478): the first connect's connection-info fetch is slow.
    // While it is pending, the process id changes and a NEW lifecycle starts
    // and connects. When the stale fetch finally resolves, the generation
    // guard must bail before it opens a second WebSocket keyed by the OLD
    // process id and clobbers the new lifecycle's socket.
    const PROCESS_ID_2 = '44444444-4444-4444-8444-444444444444';
    let releaseFirst: (value: Response) => void = () => {};
    const firstFetch = new Promise<Response>((resolve) => {
      releaseFirst = resolve;
    });
    fetchMock.mockImplementationOnce(() => firstFetch);
    fetchMock.mockImplementation(() =>
      Promise.resolve(connectionInfoResponse())
    );

    const { result, rerender } = renderHook(
      ({ processId }: { processId: string | undefined }) =>
        useNodeLogStream(ASSIGNMENT_ID, processId),
      { initialProps: { processId: PROCESS_ID } }
    );

    // Let the stale connect reach its first await (the pending fetch).
    await Promise.resolve();

    // Switch lifecycles: the second connect resolves normally and connects.
    rerender({ processId: PROCESS_ID_2 });
    await waitFor(() => expect(result.current.connectionType).toBe('direct'));

    // Now the stale fetch completes — its connect must bail on the guard.
    releaseFirst(connectionInfoResponse());
    await Promise.resolve();
    await Promise.resolve();

    // Exactly one WebSocket, and it is keyed by the NEW process id.
    expect(wsUrls).toHaveLength(1);
    expect(wsUrls[0]).toContain(PROCESS_ID_2);
    expect(wsUrls[0]).not.toContain(PROCESS_ID);
  });

  it('ignores a delayed old WebSocket that resolves after the lifecycle changed', async () => {
    // Regression (PR #478 follow-up): lifecycle A's direct socket can take
    // seconds to open. If the process id changes while that open is pending,
    // the stale socket resolving later must NOT flip connectionType or clear
    // the active lifecycle's logs before it is closed.
    const PROCESS_ID_2 = '44444444-4444-4444-8444-444444444444';
    manualOpen = true;
    // A gets a direct_url; the new lifecycle gets none, so it ends on relay.
    fetchMock.mockImplementationOnce(() =>
      Promise.resolve(connectionInfoResponse())
    );
    fetchMock.mockImplementation(() =>
      Promise.resolve(connectionInfoResponse(null))
    );

    const { result, rerender } = renderHook(
      ({ processId }: { processId: string | undefined }) =>
        useNodeLogStream(ASSIGNMENT_ID, processId),
      { initialProps: { processId: PROCESS_ID } }
    );

    // Lifecycle A fetches its info and constructs (but does not open) its
    // direct socket.
    await waitFor(() => expect(sockets).toHaveLength(1));

    // Switch lifecycles: B has no direct_url, so it connects via relay.
    rerender({ processId: PROCESS_ID_2 });
    await waitFor(() => expect(sockets).toHaveLength(2));
    sockets[1].onopen?.();
    await waitFor(() => expect(result.current.connectionType).toBe('relay'));

    // B receives one log entry.
    sockets[1].onmessage?.({
      data: JSON.stringify({
        type: 'logs',
        entries: [
          {
            id: 1,
            output_type: 'stdout',
            content: 'hello from the active lifecycle',
            timestamp: '2026-08-27T00:00:00Z',
          },
        ],
      }),
    });
    await waitFor(() => expect(result.current.logs).toHaveLength(1));

    // The OLD direct socket finally opens. The stale connect must close it
    // without touching the active lifecycle's state. The macrotask flush
    // gives both the stale continuation AND any (buggy) state write time to
    // land and render — with the bug, connectionType flips to 'direct' and
    // the received log entry is wiped.
    sockets[0].onopen?.();
    await new Promise((resolve) => setTimeout(resolve, 20));

    expect(result.current.connectionType).toBe('relay');
    expect(result.current.logs).toHaveLength(1);
    expect(result.current.error).toBeNull();
    // Exactly two sockets: A's stale direct (old process id) and B's relay.
    expect(wsUrls).toHaveLength(2);
    expect(wsUrls[0]).toContain(PROCESS_ID);
    expect(wsUrls[1]).toBe(
      `wss://hive.example.com/v1/nodes/assignments/${ASSIGNMENT_ID}/logs/ws?token=${CONNECTION_TOKEN}`
    );
  });
});
