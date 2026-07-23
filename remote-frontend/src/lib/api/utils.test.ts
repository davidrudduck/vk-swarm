import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { makeRequest, anySignal, ApiError } from './utils';

const g = globalThis as Record<string, unknown>;
if (!g.localStorage) {
  const store: Record<string, string> = {};
  g.localStorage = {
    getItem: (key: string) => store[key] || null,
    setItem: (key: string, value: string) => {
      store[key] = value;
    },
    removeItem: (key: string) => {
      delete store[key];
    },
    clear: () => {
      Object.keys(store).forEach((key) => delete store[key]);
    },
    key: (index: number) => Object.keys(store)[index] || null,
    length: Object.keys(store).length,
  };
}

describe('anySignal', () => {
  it('returns a signal that is not aborted when no input signals are aborted', () => {
    const controller1 = new AbortController();
    const controller2 = new AbortController();

    const combined = anySignal([controller1.signal, controller2.signal]);

    expect(combined.aborted).toBe(false);
  });

  it('returns an aborted signal when one input signal is already aborted', () => {
    const controller1 = new AbortController();
    const controller2 = new AbortController();
    controller1.abort('reason-1');

    const combined = anySignal([controller1.signal, controller2.signal]);

    expect(combined.aborted).toBe(true);
  });

  it('aborts when the first signal aborts', () => {
    const controller1 = new AbortController();
    const controller2 = new AbortController();

    const combined = anySignal([controller1.signal, controller2.signal]);

    controller1.abort('reason-1');

    expect(combined.aborted).toBe(true);
  });

  it('aborts when the second signal aborts', () => {
    const controller1 = new AbortController();
    const controller2 = new AbortController();

    const combined = anySignal([controller1.signal, controller2.signal]);

    controller2.abort('reason-2');

    expect(combined.aborted).toBe(true);
  });

  it('handles empty signals array', () => {
    const combined = anySignal([]);

    expect(combined.aborted).toBe(false);
  });

  it('forwards abort reason from the triggering signal', () => {
    const controller = new AbortController();
    const combined = anySignal([controller.signal]);

    controller.abort('custom reason');

    expect(combined.aborted).toBe(true);
    expect(combined.reason).toBe('custom reason');
  });

  it('forwards the already-aborted signal reason in a mixed list', () => {
    const active = new AbortController();
    const controller = new AbortController();
    controller.abort('already-gone');

    const combined = anySignal([active.signal, controller.signal]);

    expect(combined.aborted).toBe(true);
    expect(combined.reason).toBe('already-gone');
  });
});

describe('ApiError', () => {
  it('creates an error with message and status', () => {
    const error = new ApiError('Test error', 404);

    expect(error.message).toBe('Test error');
    expect(error.status).toBe(404);
    expect(error.statusCode).toBe(404);
    expect(error.name).toBe('ApiError');
  });

  it('creates an error with response', () => {
    const mockResponse = { status: 500 } as Response;
    const error = new ApiError('Server error', 500, mockResponse);

    expect(error.response).toBe(mockResponse);
  });

  it('creates an error with error_data', () => {
    const errorData = { code: 'INVALID_TOKEN', details: 'Token expired' };
    const error = new ApiError('Auth error', 401, undefined, errorData);

    expect(error.error_data).toEqual(errorData);
  });

  it('is an instance of Error', () => {
    const error = new ApiError('Test error', 400);

    expect(error).toBeInstanceOf(Error);
    expect(error).toBeInstanceOf(ApiError);
  });

  it('creates error with minimum arguments (message only)', () => {
    const error = new ApiError('bare error');

    expect(error.message).toBe('bare error');
    expect(error.status).toBeUndefined();
    expect(error.statusCode).toBeUndefined();
    expect(error.response).toBeUndefined();
    expect(error.error_data).toBeUndefined();
    expect(error.name).toBe('ApiError');
  });
});

describe('makeRequest', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    localStorage.clear();
    g.fetch = vi.fn();
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.clearAllMocks();
    localStorage.clear();
    vi.useRealTimers();
  });

  it('makes a fetch request with default headers', async () => {
    const mockFetch = vi.mocked(g.fetch) as ReturnType<typeof vi.fn>;
    mockFetch.mockResolvedValueOnce({
      ok: true,
      status: 200,
    } as Response);

    await makeRequest('http://localhost/api/test');

    expect(mockFetch).toHaveBeenCalledWith(
      'http://localhost/api/test',
      expect.objectContaining({
        headers: expect.any(Headers),
      })
    );
  });

  it('sets Content-Type header when not provided', async () => {
    const mockFetch = vi.mocked(g.fetch) as ReturnType<typeof vi.fn>;
    mockFetch.mockResolvedValueOnce({
      ok: true,
      status: 200,
    } as Response);

    await makeRequest('http://localhost/api/test');

    const call = mockFetch.mock.calls[0];
    const headers = call[1]?.headers as Headers;
    expect(headers.get('Content-Type')).toBe('application/json');
  });

  it('does not override Content-Type when already set', async () => {
    const mockFetch = vi.mocked(g.fetch) as ReturnType<typeof vi.fn>;
    mockFetch.mockResolvedValueOnce({
      ok: true,
      status: 200,
    } as Response);

    await makeRequest('http://localhost/api/test', {
      headers: { 'Content-Type': 'text/plain' },
    });

    const call = mockFetch.mock.calls[0];
    const headers = call[1]?.headers as Headers;
    expect(headers.get('Content-Type')).toBe('text/plain');
  });

  it('adds Authorization header from localStorage when not provided', async () => {
    localStorage.setItem('access_token', 'test-token-123');

    const mockFetch = vi.mocked(g.fetch) as ReturnType<typeof vi.fn>;
    mockFetch.mockResolvedValueOnce({
      ok: true,
      status: 200,
    } as Response);

    await makeRequest('http://localhost/api/test');

    const call = mockFetch.mock.calls[0];
    const headers = call[1]?.headers as Headers;
    expect(headers.get('Authorization')).toBe('Bearer test-token-123');
  });

  it('does not override Authorization when already set', async () => {
    localStorage.setItem('access_token', 'stored-token');

    const mockFetch = vi.mocked(g.fetch) as ReturnType<typeof vi.fn>;
    mockFetch.mockResolvedValueOnce({
      ok: true,
      status: 200,
    } as Response);

    await makeRequest('http://localhost/api/test', {
      headers: { Authorization: 'Bearer custom-token' },
    });

    const call = mockFetch.mock.calls[0];
    const headers = call[1]?.headers as Headers;
    expect(headers.get('Authorization')).toBe('Bearer custom-token');
  });

  it('does not add Authorization header when no token in localStorage', async () => {
    const mockFetch = vi.mocked(g.fetch) as ReturnType<typeof vi.fn>;
    mockFetch.mockResolvedValueOnce({
      ok: true,
      status: 200,
    } as Response);

    await makeRequest('http://localhost/api/test');

    const call = mockFetch.mock.calls[0];
    const headers = call[1]?.headers as Headers;
    expect(headers.get('Authorization')).toBeNull();
  });

  it('clears timeout on successful response', async () => {
    const clearTimeoutSpy = vi.spyOn(globalThis, 'clearTimeout');

    const mockFetch = vi.mocked(g.fetch) as ReturnType<typeof vi.fn>;
    mockFetch.mockResolvedValueOnce({
      ok: true,
      status: 200,
    } as Response);

    await makeRequest('http://localhost/api/test');

    expect(clearTimeoutSpy).toHaveBeenCalled();
  });

  it('passes through request options', async () => {
    const mockFetch = vi.mocked(g.fetch) as ReturnType<typeof vi.fn>;
    mockFetch.mockResolvedValueOnce({
      ok: true,
      status: 200,
    } as Response);

    await makeRequest('http://localhost/api/test', {
      method: 'POST',
      body: JSON.stringify({ key: 'value' }),
    });

    expect(mockFetch).toHaveBeenCalledWith(
      'http://localhost/api/test',
      expect.objectContaining({
        method: 'POST',
        body: JSON.stringify({ key: 'value' }),
      })
    );
  });

  it('uses provided AbortSignal', async () => {
    const controller = new AbortController();

    const mockFetch = vi.mocked(g.fetch) as ReturnType<typeof vi.fn>;
    mockFetch.mockResolvedValueOnce({
      ok: true,
      status: 200,
    } as Response);

    await makeRequest('http://localhost/api/test', {
      signal: controller.signal,
    });

    expect(mockFetch).toHaveBeenCalledWith(
      'http://localhost/api/test',
      expect.objectContaining({
        signal: expect.anything(),
      })
    );
  });

  it('aborts request when timeout expires', async () => {
    const mockFetch = vi.mocked(g.fetch) as ReturnType<typeof vi.fn>;
    mockFetch.mockImplementationOnce((_url, init) => {
      const signal = init?.signal as AbortSignal | undefined;
      if (!signal) throw new Error('Expected an abort signal');
      return new Promise<Response>((_, reject) => {
        signal.addEventListener('abort', () => reject(signal.reason), { once: true });
      });
    });

    const promise = makeRequest('http://localhost/api/test');
    // Attach the rejection assertion before advancing timers so the promise
    // never has a tick where it's rejected with no handler attached yet
    // (advancing fake timers synchronously triggers the abort/rejection,
    // and attaching `expect(...).rejects` only afterwards leaves a window
    // where Node can flag it as an unhandled rejection).
    const assertion = expect(promise).rejects.toThrow('Request timed out');

    await vi.advanceTimersByTimeAsync(30_001);

    await assertion;
  });

  it('clears timeout on fetch failure (non-timeout)', async () => {
    const clearTimeoutSpy = vi.spyOn(globalThis, 'clearTimeout');

    const mockFetch = vi.mocked(g.fetch) as ReturnType<typeof vi.fn>;
    mockFetch.mockRejectedValueOnce(new Error('Network error'));

    await expect(makeRequest('http://localhost/api/test')).rejects.toThrow('Network error');
    expect(clearTimeoutSpy).toHaveBeenCalled();
  });
});
