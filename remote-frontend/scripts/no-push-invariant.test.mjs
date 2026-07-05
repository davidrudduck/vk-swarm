import { test } from 'node:test';
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join } from 'node:path';

const ROOT = join(import.meta.dirname, '..', 'src');

function listTsFiles(dir) {
  const out = [];
  for (const entry of readdirSync(dir)) {
    const full = join(dir, entry);
    if (statSync(full).isDirectory()) out.push(...listTsFiles(full));
    else if (/\.(ts|tsx)$/.test(entry)) out.push(full);
  }
  return out;
}

const PUSH_PATTERNS = /\bWebSocket\b|\bEventSource\b|text\/event-stream|\/\.websocket|\bSSE\b/;

test('no new push channels (WebSocket/EventSource/SSE) in the hive frontend source', () => {
  const files = listTsFiles(ROOT);
  const violations = [];
  for (const f of files) {
    const src = readFileSync(f, 'utf8');
    const lines = src.split('\n');
    lines.forEach((line, i) => {
      if (PUSH_PATTERNS.test(line) && !/^\s*(\/\/|\/\*|\*)/.test(line)) {
        violations.push(`${f}:${i + 1}: ${line.trim()}`);
      }
    });
  }
  if (violations.length > 0) {
    throw new Error('Push-channel invariant violated (SC5 forbids new WS/SSE):\n' + violations.join('\n'));
  }
});