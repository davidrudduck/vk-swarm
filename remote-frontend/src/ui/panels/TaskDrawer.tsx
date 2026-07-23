import { useState } from 'react';
import { Button, Badge, Tabs } from '@/components/core';
import { StatusBadge } from '@/components/board';
import type { TaskStatus } from '@/components/board';
import { Icon } from '@/ui/chrome';
import { DiffPanel } from './DiffPanel';
import type { DiffLine } from './DiffPanel';
import { LogsPanel } from './LogsPanel';
import type { LogLine } from './LogsPanel';
import { AttemptsPanel } from './AttemptsPanel';
import type { AttemptRow } from './AttemptsPanel';

export type TaskRow = { id: string; title: string; node: string; labels?: string[] };

export interface TaskDrawerProps {
  task: TaskRow | null;
  status: TaskStatus;
  onClose: () => void;
  diffLines?: DiffLine[];
  logs?: LogLine[];
  attempts?: AttemptRow[];
}

/** Ported from design-source panels.jsx:44-91 (TaskDrawer). window.VKSwarmDesignSystem_067861
 * references replaced with direct component imports; window.Icon replaced with @/ui/chrome Icon.
 * SEED data removed from child panels — diffLines/logs/attempts props added (divergence recorded
 * in the decisions-ledger) so task 308 can wire real data. */
export function TaskDrawer({ task, status, onClose, diffLines = [], logs = [], attempts = [] }: TaskDrawerProps) {
  const [tab, setTab] = useState('diff');
  if (!task) return null;
  return (
    <>
      <div
        onClick={onClose}
        style={{ position: 'absolute', inset: 0, background: 'var(--surface-overlay)', zIndex: 10 }}
      />
      <aside
        style={{
          position: 'absolute',
          top: 0,
          right: 0,
          bottom: 0,
          width: 460,
          maxWidth: '90%',
          zIndex: 11,
          background: 'var(--surface-card)',
          borderLeft: '1px solid var(--border-strong)',
          boxShadow: 'var(--shadow-lg)',
          display: 'flex',
          flexDirection: 'column',
        }}
      >
        <div style={{ padding: '16px 18px', borderBottom: '1px solid var(--border)' }}>
          <div style={{ display: 'flex', alignItems: 'flex-start', gap: 10 }}>
            <StatusBadge status={status} showLabel={false} />
            <h3 style={{ fontSize: 'var(--text-lg)', fontWeight: 600, margin: 0, flex: 1, lineHeight: 1.3 }}>
              {task.title}
            </h3>
            <button
              className="vks-btn vks-btn--ghost vks-btn--icon"
              onClick={onClose}
              aria-label="Close"
              style={{ height: 28, width: 28 }}
            >
              <Icon
                d={
                  <>
                    <path d="M6 6l12 12M18 6L6 18" />
                  </>
                }
                size={16}
              />
            </button>
          </div>
          <div style={{ display: 'flex', gap: 6, marginTop: 12, flexWrap: 'wrap' }}>
            <Badge variant="outline" dot>
              {status === 'inprogress' ? 'In Progress' : status}
            </Badge>
            <Badge variant="secondary">{task.node}</Badge>
            {(task.labels || []).map((l) => (
              <Badge key={l} variant="outline">
                {l}
              </Badge>
            ))}
          </div>
        </div>
        <div style={{ padding: '14px 18px' }}>
          <Tabs
            value={tab}
            onValueChange={setTab}
            tabs={[
              { value: 'diff', label: 'Diff' },
              { value: 'logs', label: 'Logs' },
              { value: 'attempts', label: 'Attempts' },
            ]}
          />
        </div>
        <div style={{ flex: 1, overflowY: 'auto', padding: '0 18px 18px' }}>
          {tab === 'diff' && <DiffPanel lines={diffLines} />}
          {tab === 'logs' && <LogsPanel lines={logs} />}
          {tab === 'attempts' && <AttemptsPanel attempts={attempts} />}
        </div>
        <div style={{ padding: 16, borderTop: '1px solid var(--border)', display: 'flex', gap: 8 }}>
          <Button variant="primary" size="sm" style={{ flex: 1 }}>
            Merge
          </Button>
          <Button variant="outline" size="sm">
            Rebase
          </Button>
          <Button variant="ghost" size="sm">
            Open in IDE
          </Button>
        </div>
      </aside>
    </>
  );
}
