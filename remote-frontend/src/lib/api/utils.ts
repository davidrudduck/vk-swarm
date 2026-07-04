/**
 * Shared API utilities for making HTTP requests and handling responses.
 */

// Local ApiResponse type (remote-frontend has no shared/types alias yet)
export interface ApiResponse<T, E = T> {
  success: boolean;
  data?: T;
  error_data?: E;
  message?: string;
}

/**
 * Custom error class for API errors with typed error data.
 */
export class ApiError<E = unknown> extends Error {
  public status?: number;
  public error_data?: E;

  constructor(
    message: string,
    public statusCode?: number,
    public response?: Response,
    error_data?: E
  ) {
    super(message);
    this.name = 'ApiError';
    this.status = statusCode;
    this.error_data = error_data;
  }
}

/** Request timeout in milliseconds (30 seconds) */
export const REQUEST_TIMEOUT_MS = 30000;

/**
 * Helper to combine multiple AbortSignals (first one to abort wins).
 */
export function anySignal(signals: AbortSignal[]): AbortSignal {
  const controller = new AbortController();
  for (const signal of signals) {
    if (signal.aborted) {
      controller.abort(signal.reason);
      break;
    }
    signal.addEventListener('abort', () => controller.abort(signal.reason), {
      once: true,
    });
  }
  return controller.signal;
}

/**
 * Make an HTTP request with timeout and default headers.
 */
export const makeRequest = async (url: string, options: RequestInit = {}) => {
  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), REQUEST_TIMEOUT_MS);

  const headers = new Headers(options.headers ?? {});
  if (!headers.has('Content-Type')) {
    headers.set('Content-Type', 'application/json');
  }

  try {
    return await fetch(url, {
      ...options,
      headers,
      signal: options.signal
        ? anySignal([options.signal, controller.signal])
        : controller.signal,
    });
  } finally {
    clearTimeout(timeoutId);
  }
};

