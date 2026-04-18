import { describe, expect, it } from 'vitest';

import {
  getTailRenderSignature,
  mergeAppendOnlyItems,
  mergeRunningAppendOnlyItems,
} from './appendOnlyTimeline';
import type { PatchTypeWithKey } from '@/hooks/useConversationHistory';
import type { CommandExitStatus } from 'shared/types';

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

const commandRunItem = ({
  patchKey,
  output,
  exitStatus = null,
  status = 'created',
}: {
  patchKey: string;
  output: string;
  exitStatus?: CommandExitStatus | null;
  status?: 'created' | 'success' | 'failed';
}): PatchTypeWithKey => ({
  type: 'NORMALIZED_ENTRY',
  content: {
    entry_type: {
      type: 'tool_use',
      tool_name: 'Bash',
      action_type: {
        action: 'command_run',
        command: 'echo hello',
        result: {
          output,
          exit_status: exitStatus,
        },
      },
      status: { status },
    },
    content: 'Bash',
    timestamp: null,
    metadata: null,
  },
  patchKey,
  executionProcessId: 'process-1',
});

const assistantMessageItem = (
  patchKey: string,
  content: string
): PatchTypeWithKey => ({
  type: 'NORMALIZED_ENTRY',
  content: {
    entry_type: {
      type: 'assistant_message',
    },
    content,
    timestamp: null,
    metadata: null,
  },
  patchKey,
  executionProcessId: 'process-1',
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
    const previousItems = [
      stdoutItem('middle', 'line 2'),
      stdoutItem('tail', 'line 3'),
    ];

    const nextItems = [
      stdoutItem('head', 'line 1'),
      stdoutItem('middle', 'line 2'),
      stdoutItem('tail', 'line 3'),
    ];

    expect(mergeAppendOnlyItems(previousItems, nextItems)).toEqual(nextItems);
  });

  it('inserts newly discovered older rows ahead of retained rows from a partial snapshot', () => {
    const previousItems = [
      stdoutItem('middle', 'line 2'),
      stdoutItem('tail', 'line 3'),
    ];

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

describe('mergeRunningAppendOnlyItems', () => {
  it('appends a new rendered row when a streamed entry updates an existing logical key', () => {
    const previousItems = [stdoutItem('process-1:0', 'hello')];
    const nextItems = [stdoutItem('process-1:0', 'hello world')];
    let revision = 0;

    expect(
      mergeRunningAppendOnlyItems(
        previousItems,
        nextItems,
        () => {
          revision += 1;
          return revision;
        },
        [stdoutItem('process-1:0', 'hello')]
      )
    ).toEqual([
      stdoutItem('process-1:0', 'hello'),
      stdoutItem('process-1:0::append:1', 'hello world'),
    ]);
  });

  it('does not duplicate a streamed entry when the rendered content is unchanged', () => {
    const previousItems = [stdoutItem('process-1:0', 'hello')];
    const nextItems = [stdoutItem('process-1:0', 'hello')];

    expect(
      mergeRunningAppendOnlyItems(previousItems, nextItems, () => {
        throw new Error('revision should not advance');
      })
    ).toEqual(previousItems);
  });

  it('preserves append-only revisions across multiple streamed updates to the same logical key', () => {
    const previousItems = [
      stdoutItem('process-1:0', 'hello'),
      stdoutItem('process-1:0::append:1', 'hello world'),
    ];
    const nextItems = [stdoutItem('process-1:0', 'hello world!')];
    let revision = 1;

    expect(
      mergeRunningAppendOnlyItems(
        previousItems,
        nextItems,
        () => {
          revision += 1;
          return revision;
        },
        [stdoutItem('process-1:0', 'hello world')]
      )
    ).toEqual([
      stdoutItem('process-1:0', 'hello'),
      stdoutItem('process-1:0::append:1', 'hello world'),
      stdoutItem('process-1:0::append:2', 'hello world!'),
    ]);
  });

  it('appends inserted earlier snapshot items at the end like a chat log', () => {
    const previousItems = [
      commandRunItem({ patchKey: 'process-1:0', output: 'tool output' }),
    ];
    const nextItems = [
      stdoutItem('process-1:0', 'assistant reply'),
      commandRunItem({ patchKey: 'process-1:1', output: 'tool output' }),
    ];
    let revision = 0;

    expect(
      mergeRunningAppendOnlyItems(previousItems, nextItems, () => {
        revision += 1;
        return revision;
      })
    ).toEqual([
      commandRunItem({ patchKey: 'process-1:0', output: 'tool output' }),
      stdoutItem('process-1:0::append:1', 'assistant reply'),
    ]);
  });

  it('continues appending later live rows after an earlier snapshot insert', () => {
    let revision = 0;
    const getNextRevision = () => {
      revision += 1;
      return revision;
    };

    const afterInsert = mergeRunningAppendOnlyItems(
      [commandRunItem({ patchKey: 'process-1:0', output: 'tool output' })],
      [
        stdoutItem('process-1:0', 'assistant reply'),
        commandRunItem({ patchKey: 'process-1:1', output: 'tool output' }),
      ],
      getNextRevision
    );

    expect(
      mergeRunningAppendOnlyItems(
        afterInsert,
        [
          stdoutItem('process-1:0', 'assistant reply'),
          commandRunItem({ patchKey: 'process-1:1', output: 'tool output' }),
          stdoutItem('process-1:2', 'final reply'),
        ],
        getNextRevision,
        [
          stdoutItem('process-1:0', 'assistant reply'),
          commandRunItem({ patchKey: 'process-1:1', output: 'tool output' }),
        ]
      )
    ).toEqual([
      commandRunItem({ patchKey: 'process-1:0', output: 'tool output' }),
      stdoutItem('process-1:0::append:1', 'assistant reply'),
      stdoutItem('process-1:2', 'final reply'),
    ]);
  });

  it('ignores shorter replay snapshots instead of appending stale rows', () => {
    const previousItems = [
      stdoutItem('process-1:0', 'hello'),
      stdoutItem('process-1:1', 'world'),
    ];
    const nextItems = [stdoutItem('process-1:0', 'hello')];

    expect(
      mergeRunningAppendOnlyItems(previousItems, nextItems, () => {
        throw new Error('revision should not advance');
      })
    ).toEqual(previousItems);
  });

  it('ignores stale replay snapshots and keeps only the current transient footer', () => {
    const previousItems = [
      stdoutItem('process-1:0', 'hello'),
      stdoutItem('process-1:1', 'world'),
      stdoutItem('loading', 'loading'),
    ];
    const nextItems = [
      stdoutItem('process-1:0', 'hello'),
      stdoutItem('loading', 'loading'),
    ];

    expect(
      mergeRunningAppendOnlyItems(previousItems, nextItems, () => {
        throw new Error('revision should not advance');
      })
    ).toEqual([
      stdoutItem('process-1:0', 'hello'),
      stdoutItem('process-1:1', 'world'),
      stdoutItem('loading', 'loading'),
    ]);
  });

  it('suppresses non-tail corrections instead of appending duplicate rows', () => {
    const previousItems = [
      stdoutItem('process-1:0', 'hello'),
      stdoutItem('process-1:1', 'world'),
    ];
    const nextItems = [
      stdoutItem('process-1:0', 'HELLO'),
      stdoutItem('process-1:1', 'world'),
    ];

    expect(
      mergeRunningAppendOnlyItems(previousItems, nextItems, () => {
        throw new Error('revision should not advance');
      })
    ).toEqual(previousItems);
  });

  it('appends command-run updates as new rows without overwriting earlier output', () => {
    const previousItems = [
      commandRunItem({ patchKey: 'process-1:0', output: 'hello' }),
    ];
    const nextItems = [
      commandRunItem({
        patchKey: 'process-1:0',
        output: 'hello\nworld',
        exitStatus: { type: 'exit_code', code: 0 },
        status: 'success',
      }),
    ];
    let revision = 0;

    expect(
      mergeRunningAppendOnlyItems(previousItems, nextItems, () => {
        revision += 1;
        return revision;
      })
    ).toEqual([
      commandRunItem({ patchKey: 'process-1:0', output: 'hello' }),
      commandRunItem({
        patchKey: 'process-1:0::append:1',
        output: 'hello\nworld',
        exitStatus: { type: 'exit_code', code: 0 },
        status: 'success',
      }),
    ]);
  });

  it('appends streaming assistant message growth instead of rewriting prior text', () => {
    const previousItems = [assistantMessageItem('process-1:0', 'Thinking')];
    const nextItems = [
      assistantMessageItem('process-1:0', 'Thinking through the next step'),
    ];
    let revision = 0;

    expect(
      mergeRunningAppendOnlyItems(previousItems, nextItems, () => {
        revision += 1;
        return revision;
      })
    ).toEqual([
      assistantMessageItem('process-1:0', 'Thinking'),
      assistantMessageItem(
        'process-1:0::append:1',
        'Thinking through the next step'
      ),
    ]);
  });

  it('suppresses command-run regressions instead of appending stale output', () => {
    const previousItems = [
      commandRunItem({
        patchKey: 'process-1:0',
        output: 'hello\nworld',
        exitStatus: { type: 'exit_code', code: 0 },
        status: 'success',
      }),
    ];
    const nextItems = [
      commandRunItem({ patchKey: 'process-1:0', output: 'hello' }),
    ];

    expect(
      mergeRunningAppendOnlyItems(previousItems, nextItems, () => {
        throw new Error('revision should not advance');
      })
    ).toEqual(previousItems);
  });
});
