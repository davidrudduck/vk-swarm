import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { NodesView, ProcessesView } from './index';

const nodes = [
  { id: 'n1', name: 'justX', os: 'mac' as const, online: true, meta: '3 agents', rightCount: 3 },
  { id: 'n2', name: 'linux-01', os: 'linux' as const, online: true, meta: '1 agent', rightCount: 1 },
  { id: 'n3', name: 'winbox', os: 'windows' as const, online: false, meta: '4m ago', rightCount: 0 },
];

const processes = [
  { id: 'p1', name: 'claude-code · feat/auth', node: 'justX', state: 'running' as const, dur: '2m 14s' },
  { id: 'p2', name: 'pnpm test', node: 'linux-01', state: 'done' as const, dur: '1m 02s' },
];

describe('NodesView (SC7)', () => {
  it('renders an h2 heading and one NodeCard per node', () => {
    render(<NodesView nodes={nodes} />);
    expect(screen.getByRole('heading', { name: 'Hive' })).toBeTruthy();
    expect(screen.getByText('justX')).toBeTruthy();
    expect(screen.getByText('linux-01')).toBeTruthy();
    expect(screen.getByText('winbox')).toBeTruthy();
  });
});

describe('ProcessesView (SC7)', () => {
  it('renders an h2 heading and one row per process', () => {
    render(<ProcessesView processes={processes} />);
    expect(screen.getByRole('heading', { name: 'Processes' })).toBeTruthy();
    expect(screen.getByText('claude-code · feat/auth')).toBeTruthy();
    expect(screen.getByText('pnpm test')).toBeTruthy();
  });
  it('renders a vks-loader for running processes', () => {
    const { container } = render(<ProcessesView processes={processes} />);
    expect(container.querySelector('.vks-loader')).toBeTruthy();
  });
});
