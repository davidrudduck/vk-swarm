/**
 * Shared API utilities for making HTTP requests and handling responses.
 */

import type { ApiResponse } from 'shared/types';

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

/**
 * Discriminator the server emits for `ApiError::HiveNotConfigured`.
 *
 * `into_response` builds the message as `format!("{}: {}", error_type, self)`
 * (`crates/server/src/error.rs`), so the body message is
 * `"HiveNotConfigured: This node is not connected to a hive"`. A Rust test pins
 * this prefix — if it changes, the backend suite fails rather than this guard
 * silently going dead.
 */
const HIVE_NOT_CONFIGURED_CODE = 'HiveNotConfigured';

/**
 * True when an error is the server's "this node is not connected to a hive"
 * response (`ApiError::HiveNotConfigured` -> HTTP 503).
 *
 * **Status alone is NOT sufficient.** `RemoteClientError::Http` forwards the
 * upstream status verbatim (`error.rs`: `StatusCode::from_u16(*status)`), so a
 * configured-but-DOWN hive returning 503 would otherwise be misread as "no hive
 * configured" — showing the not-connected UI for an outage, and (because callers
 * resolve rather than throw for this case) suppressing the retry that would let
 * it recover. The message discriminator separates the two: a forwarded upstream
 * 503 carries the hive's own body, not this code.
 */
export function isHiveNotConfigured(err: unknown): boolean {
  return (
    err instanceof ApiError &&
    err.status === 503 &&
    err.message.startsWith(HIVE_NOT_CONFIGURED_CODE)
  );
}

/** Request timeout in milliseconds (30 seconds) */
export const REQUEST_TIMEOUT_MS = 30000;

let unauthorizedHandler: (() => void) | null = null;

export function onUnauthorized(handler: () => void): () => void {
  unauthorizedHandler = handler;
  return () => {
    if (unauthorizedHandler === handler) unauthorizedHandler = null;
  };
}

export function notifyUnauthorized(): void {
  unauthorizedHandler?.();
}

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
 * Serialise a request body that may contain `bigint` values.
 *
 * ts-rs maps every Rust `i64` to a TypeScript `bigint`, so generated request
 * types (e.g. `ProposalItemInput.sort_order`) legitimately hold BigInts —
 * and plain `JSON.stringify` throws `TypeError: Do not know how to serialize
 * a BigInt` on them. Serde deserialises a JSON number into `i64` fine, so
 * emitting them as numbers is the correct wire form.
 *
 * A value outside the safe-integer range throws rather than being coerced:
 * `Number()` would silently lose precision, and a quoted string would be
 * rejected by serde (an `i64` field does not accept a JSON string), so both
 * alternatives corrupt or fail further from the cause.
 */
export const jsonBody = (payload: unknown): string =>
  JSON.stringify(payload, (_key, value) => {
    if (typeof value !== 'bigint') return value;
    if (
      value < BigInt(Number.MIN_SAFE_INTEGER) ||
      value > BigInt(Number.MAX_SAFE_INTEGER)
    ) {
      throw new RangeError(
        `Cannot serialise ${value}n to JSON without losing precision`
      );
    }
    return Number(value);
  });

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
    const response = await fetch(url, {
      ...options,
      headers,
      signal: options.signal
        ? // If caller provided a signal, combine with timeout
          anySignal([options.signal, controller.signal])
        : controller.signal,
    });
    // Browser-session 401 from `require_browser_session` is a bare empty 401 (no JSON
    // body). Hive/proxy 401s are `application/json`. Status alone is not sufficient —
    // the same class of bug the 503 path documents above.
    if (response.status === 401) {
      const contentType = response.headers.get('content-type') ?? '';
      if (!contentType.toLowerCase().includes('application/json')) {
        notifyUnauthorized();
      }
    }
    return response;
  } finally {
    clearTimeout(timeoutId);
  }
};

// Result types for typed error handling

export type Ok<T> = { success: true; data: T };
export type Err<E> = { success: false; error: E | undefined; message?: string };

/** Result type for endpoints that need typed errors */
export type Result<T, E> = Ok<T> | Err<E>;

/**
 * Handle API response and return a Result type for typed error handling.
 * Use this for endpoints where you need to inspect error_data.
 */
export const handleApiResponseAsResult = async <T, E>(
  response: Response
): Promise<Result<T, E>> => {
  if (!response.ok) {
    // HTTP error - no structured error data
    let errorMessage = `Request failed with status ${response.status}`;

    try {
      const errorData = await response.json();
      if (errorData.message) {
        errorMessage = errorData.message;
      }
    } catch {
      errorMessage = response.statusText || errorMessage;
    }

    return {
      success: false,
      error: undefined,
      message: errorMessage,
    };
  }

  const result: ApiResponse<T, E> = await response.json();

  if (!result.success) {
    return {
      success: false,
      error: result.error_data || undefined,
      message: result.message || undefined,
    };
  }

  return { success: true, data: result.data as T };
};

/**
 * Handle API response and throw ApiError on failure.
 * Use this for standard endpoints where errors should be thrown.
 */
export const handleApiResponse = async <T, E = T>(
  response: Response
): Promise<T> => {
  if (!response.ok) {
    let errorMessage = `Request failed with status ${response.status}`;

    try {
      const errorData = await response.json();
      if (errorData.message) {
        errorMessage = errorData.message;
      }
    } catch {
      // Fallback to status text if JSON parsing fails
      errorMessage = response.statusText || errorMessage;
    }

    console.error('[API Error]', {
      message: errorMessage,
      status: response.status,
      response,
      endpoint: response.url,
      timestamp: new Date().toISOString(),
    });
    throw new ApiError<E>(errorMessage, response.status, response);
  }

  if (response.status === 204) {
    return undefined as T;
  }

  const result: ApiResponse<T, E> = await response.json();

  if (!result.success) {
    // Check for error_data first (structured errors), then fall back to message
    if (result.error_data) {
      console.error('[API Error with data]', {
        error_data: result.error_data,
        message: result.message,
        status: response.status,
        response,
        endpoint: response.url,
        timestamp: new Date().toISOString(),
      });
      // Throw a properly typed error with the error data
      throw new ApiError<E>(
        result.message || 'API request failed',
        response.status,
        response,
        result.error_data
      );
    }

    console.error('[API Error]', {
      message: result.message || 'API request failed',
      status: response.status,
      response,
      endpoint: response.url,
      timestamp: new Date().toISOString(),
    });
    throw new ApiError<E>(
      result.message || 'API request failed',
      response.status,
      response
    );
  }

  return result.data as T;
};
