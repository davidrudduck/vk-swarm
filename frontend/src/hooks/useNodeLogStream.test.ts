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

class FakeWebSocket {
  url: string;
  onopen: (() => void) | null = null;
  onerror: (() => void) | null = null;
  onclose: (() => void) | null = null;
  onmessage: ((event: { data: string }) => void) | null = null;

  constructor(url: string) {
    this.url = url;
    wsUrls.push(url);
    // Resolve as an opened direct connection so no relay fallback fires.
    queueMicrotask(() => this.onopen?.());
  }

  close(): void {}
}

function connectionInfoResponse(): Response {
  return new Response(
    JSON.stringify({
      assignment_id: ASSIGNMENT_ID,
      node_id: '33333333-3333-4333-8333-333333333333',
      direct_url: 'https://node.example.com',
      relay_url: 'https://hive.example.com/v1/nodes/assignments/11111111-1111-4111-8111-111111111111/logs/ws',
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
    vi.stubGlobal('WebSocket', FakeWebSocket);
  });

  afterEach(() => {
    vi.unstubAllGlobals();
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
});
