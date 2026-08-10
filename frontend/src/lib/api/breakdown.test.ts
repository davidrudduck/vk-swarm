import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { breakdownApi } from './breakdown';
import { ApiError } from './utils';

/**
 * These tests drive the REAL `makeRequest` + `handleApiResponse` against a
 * stubbed `fetch`, deliberately unlike every other breakdown test in the repo.
 *
 * `useBreakdown.test.ts`, `BreakdownReviewDialog.test.tsx` and
 * `TaskCard.breakdown.test.tsx` all `vi.mock('@/lib/api/breakdown')`, so until
 * this file existed nothing verified the request URLs, the HTTP verbs, the body
 * serialisation, or the response unwrapping. A wrong verb or a body that cannot
 * be serialised would pass the entire suite and fail only in the browser.
 */

type FetchArgs = [input: string, init?: RequestInit];

/** Build a real `Response` carrying the server's `ApiResponse<T>` envelope. */
function apiResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'Content-Type': 'application/json' },
  });
}

function ok<T>(data: T): Response {
  return apiResponse({ success: true, data, error_data: null, message: null });
}

const TASK_ID = '11111111-1111-4111-8111-111111111111';
const PROPOSAL_ID = '22222222-2222-4222-8222-222222222222';

const proposal = {
  id: PROPOSAL_ID,
  task_id: TASK_ID,
  status: 'draft',
  execution_process_id: null,
  error: null,
  created_at: '2026-08-10T00:00:00Z',
  updated_at: '2026-08-10T00:00:00Z',
};

let fetchMock: ReturnType<typeof vi.fn>;

/** The `fetch` call recorded for assertion. */
function lastCall(): FetchArgs {
  const { calls } = fetchMock.mock;
  return calls[calls.length - 1] as FetchArgs;
}

beforeEach(() => {
  fetchMock = vi.fn();
  vi.stubGlobal('fetch', fetchMock);
});

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe('breakdownApi request wiring', () => {
  it('get() issues a GET to the task breakdown endpoint', async () => {
    fetchMock.mockResolvedValue(ok({ proposal, items: [] }));

    await breakdownApi.get(TASK_ID);

    const [url, init] = lastCall();
    expect(url).toBe(`/api/tasks/${TASK_ID}/breakdown`);
    // makeRequest passes no explicit method for reads; fetch defaults to GET.
    expect(init?.method).toBeUndefined();
  });

  it('trigger() POSTs to the same task breakdown endpoint', async () => {
    fetchMock.mockResolvedValue(ok(proposal));

    await breakdownApi.trigger(TASK_ID);

    const [url, init] = lastCall();
    expect(url).toBe(`/api/tasks/${TASK_ID}/breakdown`);
    expect(init?.method).toBe('POST');
  });

  it('accept() POSTs to the proposal accept endpoint', async () => {
    fetchMock.mockResolvedValue(ok([]));

    await breakdownApi.accept(PROPOSAL_ID);

    const [url, init] = lastCall();
    expect(url).toBe(`/api/breakdown-proposals/${PROPOSAL_ID}/accept`);
    expect(init?.method).toBe('POST');
  });

  it('discard() POSTs to the proposal discard endpoint', async () => {
    fetchMock.mockResolvedValue(ok(proposal));

    await breakdownApi.discard(PROPOSAL_ID);

    const [url, init] = lastCall();
    expect(url).toBe(`/api/breakdown-proposals/${PROPOSAL_ID}/discard`);
    expect(init?.method).toBe('POST');
  });

  it('retry() POSTs to the proposal retry endpoint', async () => {
    fetchMock.mockResolvedValue(ok(proposal));

    await breakdownApi.retry(PROPOSAL_ID);

    const [url, init] = lastCall();
    expect(url).toBe(`/api/breakdown-proposals/${PROPOSAL_ID}/retry`);
    expect(init?.method).toBe('POST');
  });

  it('dependencies() issues a GET to the task dependencies endpoint', async () => {
    fetchMock.mockResolvedValue(ok([]));

    await breakdownApi.dependencies(TASK_ID);

    const [url] = lastCall();
    expect(url).toBe(`/api/tasks/${TASK_ID}/dependencies`);
  });

  it('sends a JSON Content-Type so the server parses the body', async () => {
    fetchMock.mockResolvedValue(ok([]));

    await breakdownApi.putItems(PROPOSAL_ID, { items: [] });

    const [, init] = lastCall();
    expect(new Headers(init?.headers).get('Content-Type')).toBe(
      'application/json'
    );
  });
});

describe('breakdownApi.putItems body serialisation', () => {
  /**
   * `ProposalItemInput.sort_order` and `depends_on_indices` are generated from
   * Rust `i64`, so ts-rs types them as `bigint` and `BreakdownReviewDialog`
   * builds them with `BigInt(index)`. Plain `JSON.stringify` THROWS on a BigInt
   * ("Do not know how to serialize a BigInt"), which would make every edit in
   * the review dialog fail silently. Nothing else in the suite catches this:
   * the dialog tests mock this module out entirely.
   */
  it('serialises bigint sort_order and dependency indices as JSON numbers', async () => {
    fetchMock.mockResolvedValue(ok([]));

    await breakdownApi.putItems(PROPOSAL_ID, {
      items: [
        {
          title: 'first',
          description: 'does a thing',
          sort_order: BigInt(0),
          depends_on_indices: [],
        },
        {
          title: 'second',
          description: null,
          sort_order: BigInt(1),
          depends_on_indices: [BigInt(0)],
        },
      ],
    });

    const [url, init] = lastCall();
    expect(url).toBe(`/api/breakdown-proposals/${PROPOSAL_ID}/items`);
    expect(init?.method).toBe('PUT');
    expect(JSON.parse(init?.body as string)).toEqual({
      items: [
        {
          title: 'first',
          description: 'does a thing',
          sort_order: 0,
          depends_on_indices: [],
        },
        {
          title: 'second',
          description: null,
          sort_order: 1,
          depends_on_indices: [0],
        },
      ],
    });
  });

  it('sends an empty item list verbatim (deleting every item)', async () => {
    fetchMock.mockResolvedValue(ok([]));

    await breakdownApi.putItems(PROPOSAL_ID, { items: [] });

    const [, init] = lastCall();
    expect(JSON.parse(init?.body as string)).toEqual({ items: [] });
  });
});

describe('breakdownApi response handling', () => {
  it('unwraps the ApiResponse envelope and returns data', async () => {
    const items = [
      {
        id: 'item-1',
        proposal_id: PROPOSAL_ID,
        title: 'first',
        description: null,
        sort_order: 0,
        depends_on_item_ids: '[]',
        created_at: '2026-08-10T00:00:00Z',
      },
    ];
    fetchMock.mockResolvedValue(ok({ proposal, items }));

    const result = await breakdownApi.get(TASK_ID);

    expect(result?.proposal.id).toBe(PROPOSAL_ID);
    expect(result?.items).toHaveLength(1);
  });

  /**
   * Pins the server contract for "task exists but has no proposal": HTTP 200
   * with `data: null`, never 204. Task 301 fixed this deliberately after a
   * CodeRabbit finding, and `useBreakdownProposal` reads `data?.proposal` — a
   * 204 would surface as `undefined` and the badge logic would misbehave.
   */
  it('returns null when the task has no proposal (200 + data:null)', async () => {
    fetchMock.mockResolvedValue(ok(null));

    await expect(breakdownApi.get(TASK_ID)).resolves.toBeNull();
  });

  it('returns the created child tasks from accept()', async () => {
    fetchMock.mockResolvedValue(ok([{ id: 'child-1' }, { id: 'child-2' }]));

    const tasks = await breakdownApi.accept(PROPOSAL_ID);

    expect(tasks).toHaveLength(2);
  });

  it('throws ApiError carrying the server message on success:false', async () => {
    fetchMock.mockResolvedValue(
      apiResponse({
        success: false,
        data: null,
        error_data: null,
        message: 'A draft proposal already exists for this task',
      })
    );

    await expect(breakdownApi.trigger(TASK_ID)).rejects.toThrow(
      'A draft proposal already exists for this task'
    );
  });

  it('throws ApiError with the HTTP status on a non-ok response', async () => {
    vi.spyOn(console, 'error').mockImplementation(() => {});
    fetchMock.mockResolvedValue(
      apiResponse({ message: 'Task not found' }, 404)
    );

    await expect(breakdownApi.get(TASK_ID)).rejects.toMatchObject({
      name: 'ApiError',
      status: 404,
    });
  });

  it('propagates the underlying error when the network fails', async () => {
    fetchMock.mockRejectedValue(new TypeError('Failed to fetch'));

    await expect(breakdownApi.accept(PROPOSAL_ID)).rejects.toThrow(
      'Failed to fetch'
    );
  });

  it('surfaces a rejection rather than a value when the proposal is gone', async () => {
    vi.spyOn(console, 'error').mockImplementation(() => {});
    fetchMock.mockResolvedValue(apiResponse({ message: 'gone' }, 404));

    await expect(breakdownApi.discard(PROPOSAL_ID)).rejects.toBeInstanceOf(
      ApiError
    );
  });
});
