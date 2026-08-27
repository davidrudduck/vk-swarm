import { afterEach, describe, expect, it, vi } from 'vitest';

import { browserAuthApi } from './browserAuth';
import { ApiError } from './utils';

const okEnvelope = (data: unknown) =>
  new Response(JSON.stringify({ success: true, data }), {
    status: 200,
    headers: { 'content-type': 'application/json' },
  });

describe('browserAuthApi', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('getState GETs /api/auth/state and unwraps BrowserAuthState', async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValue(
        okEnvelope({ authorized: true, oauth_available: true })
      );
    vi.stubGlobal('fetch', fetchMock);

    await expect(browserAuthApi.getState()).resolves.toEqual({
      authorized: true,
      oauth_available: true,
    });
    expect(String(fetchMock.mock.calls[0][0])).toContain('/api/auth/state');
    expect(fetchMock.mock.calls[0][1]?.method ?? 'GET').toBe('GET');
  });

  it('startLogin POSTs provider and return_to to /api/auth/handoff/init', async () => {
    const fetchMock = vi.fn().mockResolvedValue(
      okEnvelope({
        handoff_id: 'h1',
        authorize_url: 'https://hive.example/authorize',
      })
    );
    vi.stubGlobal('fetch', fetchMock);

    await expect(
      browserAuthApi.startLogin('hive', 'http://node.example/')
    ).resolves.toEqual({
      handoff_id: 'h1',
      authorize_url: 'https://hive.example/authorize',
    });
    expect(String(fetchMock.mock.calls[0][0])).toContain(
      '/api/auth/handoff/init'
    );
    expect(fetchMock.mock.calls[0][1]).toMatchObject({ method: 'POST' });
    expect(JSON.parse(fetchMock.mock.calls[0][1].body)).toEqual({
      provider: 'hive',
      return_to: 'http://node.example/',
    });
  });

  it('getState throws ApiError when the envelope is unsuccessful', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue(
        new Response(JSON.stringify({ success: false, message: 'nope' }), {
          status: 500,
          headers: { 'content-type': 'application/json' },
        })
      )
    );

    await expect(browserAuthApi.getState()).rejects.toBeInstanceOf(ApiError);
  });

  it('logout POSTs /api/auth/browser/logout', async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValue(new Response(null, { status: 204 }));
    vi.stubGlobal('fetch', fetchMock);

    await expect(browserAuthApi.logout()).resolves.toBeUndefined();
    expect(String(fetchMock.mock.calls[0][0])).toContain(
      '/api/auth/browser/logout'
    );
    expect(fetchMock.mock.calls[0][1]).toMatchObject({ method: 'POST' });
  });

  it('logout throws ApiError when the response is not ok', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue(
        new Response(JSON.stringify({ message: 'boom' }), {
          status: 500,
          headers: { 'content-type': 'application/json' },
        })
      )
    );

    await expect(browserAuthApi.logout()).rejects.toMatchObject({
      status: 500,
      message: 'Logout failed with status 500',
    });
  });

  it('disconnectHive POSTs /api/auth/logout', async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValue(new Response(null, { status: 204 }));
    vi.stubGlobal('fetch', fetchMock);

    await expect(browserAuthApi.disconnectHive()).resolves.toBeUndefined();
    expect(String(fetchMock.mock.calls[0][0])).toContain('/api/auth/logout');
    expect(String(fetchMock.mock.calls[0][0])).not.toContain(
      '/api/auth/browser/logout'
    );
    expect(fetchMock.mock.calls[0][1]).toMatchObject({ method: 'POST' });
  });

  it('disconnectHive throws ApiError when the response is not ok', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue(
        new Response(JSON.stringify({ message: 'boom' }), {
          status: 502,
          headers: { 'content-type': 'application/json' },
        })
      )
    );

    await expect(browserAuthApi.disconnectHive()).rejects.toMatchObject({
      status: 502,
      message: 'Logout failed with status 502',
    });
  });
});
