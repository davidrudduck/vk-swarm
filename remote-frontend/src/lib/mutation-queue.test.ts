// @vitest-environment node
import { describe, it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

describe('mutation queue module (SC10)', () => {
  it('exports enqueueMutation function', () => {
    const source = readFileSync(join(__dirname, 'mutation-queue.ts'), 'utf-8');
    expect(source).toContain('export async function enqueueMutation');
    expect(source).toContain('idb-keyval');
    expect(source).toContain("import { get, set } from 'idb-keyval'");
  });

  it('exports replayMutations function', () => {
    const source = readFileSync(join(__dirname, 'mutation-queue.ts'), 'utf-8');
    expect(source).toContain('export async function replayMutations');
    expect(source).toContain('get<MutationEntry[]>');
  });

  it('exports MutationEntry interface', () => {
    const source = readFileSync(join(__dirname, 'mutation-queue.ts'), 'utf-8');
    expect(source).toContain('export interface MutationEntry');
    expect(source).toContain('operation: string');
    expect(source).toContain('endpoint: string');
    expect(source).toContain('payload');
    expect(source).toContain('timestamp: number');
  });

  it('exports getQueueLength function', () => {
    const source = readFileSync(join(__dirname, 'mutation-queue.ts'), 'utf-8');
    expect(source).toContain('export async function getQueueLength');
  });
});
