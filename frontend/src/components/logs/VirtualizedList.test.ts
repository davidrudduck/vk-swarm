import { describe, expect, it } from 'vitest';

import { getTailRenderSignature, mergeAppendOnlyItems } from './VirtualizedList';
import type { PatchTypeWithKey } from '@/hooks/useConversationHistory';

const stdoutItem = (
  patchKey: string,
  content: string,
  executionProcessId = 'process-1'
): PatchTypeWithKey => ({
  type: 'STDOUT',
  content,
  patchKey,
  executionProcessId,
});

describe('mergeAppendOnlyItems', () => {
  it('preserves previously loaded content when a later update omits it', () => {
    const previousItems = [
      stdoutItem('oldest', 'line 1'),
      stdoutItem('newer', 'line 2'),
      stdoutItem('loading', 'loading'),
    ];

    const nextItems = [
      stdoutItem('newer', 'line 2 updated'),
      stdoutItem('next_action', 'next'),
    ];

    expect(mergeAppendOnlyItems(previousItems, nextItems)).toEqual([
      stdoutItem('oldest', 'line 1'),
      stdoutItem('newer', 'line 2 updated'),
      stdoutItem('next_action', 'next'),
    ]);
  });

  it('adopts the latest chronological order when the next snapshot is a superset', () => {
    const previousItems = [stdoutItem('middle', 'line 2'), stdoutItem('tail', 'line 3')];

    const nextItems = [
      stdoutItem('head', 'line 1'),
      stdoutItem('middle', 'line 2'),
      stdoutItem('tail', 'line 3'),
    ];

    expect(mergeAppendOnlyItems(previousItems, nextItems)).toEqual(nextItems);
  });

  it('inserts newly discovered older rows ahead of retained rows from a partial snapshot', () => {
    const previousItems = [stdoutItem('middle', 'line 2'), stdoutItem('tail', 'line 3')];

    const nextItems = [
      stdoutItem('head', 'line 1'),
      stdoutItem('tail', 'line 3 updated'),
    ];

    expect(mergeAppendOnlyItems(previousItems, nextItems)).toEqual([
      stdoutItem('head', 'line 1'),
      stdoutItem('middle', 'line 2'),
      stdoutItem('tail', 'line 3 updated'),
    ]);
  });

  it('drops stale transient rows instead of persisting them forever', () => {
    const previousItems = [
      stdoutItem('line', 'line 1'),
      stdoutItem('loading', 'loading'),
    ];

    const nextItems = [
      stdoutItem('line', 'line 1'),
      stdoutItem('next_action', 'next'),
    ];

    expect(mergeAppendOnlyItems(previousItems, nextItems)).toEqual([
      stdoutItem('line', 'line 1'),
      stdoutItem('next_action', 'next'),
    ]);
  });
});

describe('getTailRenderSignature', () => {
  it('treats a replaced placeholder tail as new tail content for auto-follow', () => {
    expect(
      getTailRenderSignature([
        stdoutItem('line', 'line 1'),
        stdoutItem('loading', 'loading'),
      ])
    ).not.toEqual(
      getTailRenderSignature([
        stdoutItem('line', 'line 1'),
        stdoutItem('next', 'line 2'),
      ])
    );
  });

  it('treats in-place tail updates as new rendered tail content for auto-follow', () => {
    expect(
      getTailRenderSignature([
        stdoutItem('line', 'line 1'),
        stdoutItem('tail', 'tail line'),
      ])
    ).not.toEqual(
      getTailRenderSignature([
        stdoutItem('line', 'line 1'),
        stdoutItem('tail', 'tail line updated'),
      ])
    );
  });

  it('handles bigint fields in normalized tail entries', () => {
    const bigintTailItem = {
      type: 'NORMALIZED_ENTRY' as const,
      content: {
        entry_type: {
          type: 'execution_end',
          process_id: 'process-1',
          process_name: 'Coding Agent',
          started_at: '2026-04-18T10:00:00Z',
          ended_at: '2026-04-18T10:00:05Z',
          duration_seconds: BigInt(5),
          status: 'success',
        },
        content: '',
        timestamp: '2026-04-18T10:00:05Z',
        metadata: null,
      },
      patchKey: 'process-1:execution_end',
      executionProcessId: 'process-1',
    } as PatchTypeWithKey;

    expect(() => getTailRenderSignature([bigintTailItem])).not.toThrow();
  });
});
