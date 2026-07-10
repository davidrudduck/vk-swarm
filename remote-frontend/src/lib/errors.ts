/**
 * Parse an unknown error into a user-friendly string.
 *
 * Handles: Error (including ApiError with error_data), string, null, symbol,
 * objects with {error} or {message} keys, JSON-encoded bodies, circular refs,
 * primitive JSON values. Returns 'Failed' as the generic fallback.
 */
export function parseErrorMessage(err: unknown): string {
  let raw: string;
  if (err instanceof Error) {
    raw = err.message;
  } else if (typeof err === 'string') {
    raw = err || 'Failed';
  } else if (err == null) {
    return 'Failed';
  } else if (typeof err === 'symbol') {
    return 'Failed';
  } else {
    try {
      raw = JSON.stringify(err) ?? 'Failed';
    } catch {
      return 'Failed';
    }
  }
  if (!raw) return 'Failed';
  try {
    const parsed = JSON.parse(raw);
    if (typeof parsed === 'string' && parsed) return parsed;
    if (parsed !== null && typeof parsed === 'object') {
      if (typeof parsed.message === 'string' && parsed.message) return parsed.message;
      if (typeof parsed.error === 'string' && parsed.error) return parsed.error;
      return 'Failed';
    }
    return raw || 'Failed';
  } catch {
    return raw || 'Failed';
  }
}
