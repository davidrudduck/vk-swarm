import { describe, it, expect } from 'vitest';
import { render } from '@testing-library/react';
import { Button, Badge, Card, CardHeader, CardTitle, CardContent, Input, Switch, Checkbox, Tabs, Select, Loader } from '@/components/core';
import { StatusBadge, TaskCard, NodeCard } from '@/components/board';
import { SettingsSection, SettingsRow } from '@/components/settings';
import { readFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const indexCss = readFileSync(join(__dirname, '..', 'index.css'), 'utf-8');

describe('index.css wires components.css (SC1)', () => {
  it("@imports components.css", () => {
    expect(indexCss).toContain("@import './styles/components.css';");
  });
});

describe('render parity — every component mounts without throwing (SC4/SC5/SC6)', () => {
  it('core: Button, Badge, Card compound, Input, Switch, Checkbox, Tabs, Select, Loader', () => {
    expect(() => render(<Button>ok</Button>)).not.toThrow();
    expect(() => render(<Badge>new</Badge>)).not.toThrow();
    expect(() => render(<Card><CardHeader><CardTitle>t</CardTitle></CardHeader><CardContent>c</CardContent></Card>)).not.toThrow();
    expect(() => render(<Input />)).not.toThrow();
    expect(() => render(<Switch />)).not.toThrow();
    expect(() => render(<Checkbox />)).not.toThrow();
    expect(() => render(<Tabs tabs={[{ value: 'a', label: 'A' }]} />)).not.toThrow();
    expect(() => render(<Select options={[{ value: 'a', label: 'A' }]} />)).not.toThrow();
    expect(() => render(<Loader />)).not.toThrow();
  });

  it('board: StatusBadge, TaskCard, NodeCard', () => {
    expect(() => render(<StatusBadge status="done" />)).not.toThrow();
    expect(() => render(<TaskCard title="t" status="inprogress" node="n" labels={['a']} days={1} attempt="running" />)).not.toThrow();
    expect(() => render(<NodeCard name="n" os="linux" online />)).not.toThrow();
  });

  it('settings: SettingsSection, SettingsRow (stacked + inline + nested)', () => {
    expect(() => render(<SettingsSection title="t"><div /></SettingsSection>)).not.toThrow();
    expect(() => render(<SettingsRow label="l" control={<input />} />)).not.toThrow();
    expect(() => render(<SettingsRow label="l" inline control={<input />} />)).not.toThrow();
    expect(() => render(<SettingsRow label="l" nested control={<input />} />)).not.toThrow();
  });
});
