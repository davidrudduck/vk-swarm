import { describe, it, expect } from 'vitest';
import type { ApiResponse } from 'shared/types';

describe('shared alias', () => {
  it('resolves shared/types', () => {
    const x: ApiResponse<string> = { success: true, data: 'ok', error_data: null, message: null };
    expect(x.data).toBe('ok');
  });
});
