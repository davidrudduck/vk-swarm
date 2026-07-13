import { describe, it, expect } from 'vitest';
import { parseErrorMessage } from './errors';
import { ApiError } from './api/utils';

describe('parseErrorMessage', () => {
  it('returns error.message for Error instances', () => {
    expect(parseErrorMessage(new Error('boom'))).toBe('boom');
  });

  it('returns err.message for ApiError instances (error_data is not used)', () => {
    const err = new ApiError('denied', 403, undefined, { code: 'E_DENIED' });
    expect(parseErrorMessage(err)).toBe('denied');
  });

  it('returns string as-is', () => {
    expect(parseErrorMessage('plain failure')).toBe('plain failure');
  });

  it('returns "Failed" for empty string', () => {
    expect(parseErrorMessage('')).toBe('Failed');
  });

  it('returns "Failed" for null', () => {
    expect(parseErrorMessage(null)).toBe('Failed');
  });

  it('returns "Failed" for undefined', () => {
    expect(parseErrorMessage(undefined)).toBe('Failed');
  });

  it('returns "Failed" for symbol', () => {
    expect(parseErrorMessage(Symbol('x'))).toBe('Failed');
  });

  it('returns "Failed" for plain object with no message/error keys', () => {
    expect(parseErrorMessage({ code: 'E_DENIED' })).toBe('Failed');
  });

  it('extracts message from JSON-encoded Error body with {message} key', () => {
    expect(parseErrorMessage(new Error('{"message":"server denied"}'))).toBe('server denied');
  });

  it('extracts error from JSON-encoded Error body with {error} key', () => {
    expect(parseErrorMessage(new Error('{"error":"not found"}'))).toBe('not found');
  });

  it('extracts string primitive from JSON-encoded Error body', () => {
    expect(parseErrorMessage(new Error('"just a string"'))).toBe('just a string');
  });

  it('returns raw string for Error whose message is a JSON number', () => {
    expect(parseErrorMessage(new Error('42'))).toBe('42');
  });

  it('returns raw string for Error whose message is JSON true', () => {
    expect(parseErrorMessage(new Error('true'))).toBe('true');
  });

  it('returns raw string for Error whose message is JSON null', () => {
    expect(parseErrorMessage(new Error('null'))).toBe('null');
  });

  it('returns "Failed" for circular reference object', () => {
    const circular: Record<string, unknown> = {};
    circular.self = circular;
    expect(parseErrorMessage(circular)).toBe('Failed');
  });

  it('returns "Failed" for object with {message: ""}', () => {
    expect(parseErrorMessage(new Error('{"message":""}'))).toBe('Failed');
  });

  it('returns "Failed" for object with {error: ""}', () => {
    expect(parseErrorMessage(new Error('{"error":""}'))).toBe('Failed');
  });

  it('prefers {message} over {error} when both present', () => {
    expect(parseErrorMessage(new Error('{"message":"msg","error":"err"}'))).toBe('msg');
  });

  it('returns "Failed" for object with nested non-string message', () => {
    expect(parseErrorMessage(new Error('{"message":123}'))).toBe('Failed');
  });

  it('returns "Failed" for object with toJSON that throws', () => {
    const obj = {
      toJSON() {
        throw new Error('serialization explosion');
      },
    };
    expect(parseErrorMessage(obj)).toBe('Failed');
  });

  it('returns "Failed" for non-serializable value (BigInt)', () => {
    expect(parseErrorMessage(BigInt(9007199254740991))).toBe('Failed');
  });

  it('returns "Failed" for Error whose message is empty string', () => {
    expect(parseErrorMessage(new Error(''))).toBe('Failed');
  });

  it('returns "Failed" for Error whose message is just whitespace', () => {
    expect(parseErrorMessage(new Error('   '))).toBe('Failed');
  });

  it('returns raw when JSON.parse succeeds but result is a number primitive', () => {
    expect(parseErrorMessage(new Error('3.14'))).toBe('3.14');
  });

  it('returns raw string for Error whose message is JSON boolean false', () => {
    expect(parseErrorMessage(new Error('false'))).toBe('false');
  });

  it('returns error from JSON body with only {error} key', () => {
    expect(parseErrorMessage(new Error('{"error":"db connection refused"}'))).toBe('db connection refused');
  });

  it('returns "Failed" for object with {message: null} in JSON body', () => {
    expect(parseErrorMessage(new Error('{"message":null}'))).toBe('Failed');
  });

  it('returns "Failed" for JSON array body', () => {
    expect(parseErrorMessage(new Error('["one","two"]'))).toBe('Failed');
  });

  it('returns "Failed" for a function (JSON.stringify yields undefined)', () => {
    expect(parseErrorMessage(() => {})).toBe('Failed');
  });

  it('returns "Failed" for a class constructor', () => {
    class Foo {}
    expect(parseErrorMessage(Foo)).toBe('Failed');
  });
});
