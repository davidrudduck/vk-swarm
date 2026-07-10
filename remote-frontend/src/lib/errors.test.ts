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
});
