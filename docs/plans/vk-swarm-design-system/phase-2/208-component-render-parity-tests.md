---
id: "208"
phase: 2
title: Wire components.css into index.css + per-component render parity tests
status: ready
depends_on: ["202","203","204","205","206","207"]
parallel: false
conflicts_with: []
files:
  - remote-frontend/src/index.css
  - remote-frontend/src/components/render-parity.test.tsx
irreversible: false
scope_test: "remote-frontend/src/components/render-parity.test.tsx"
allowed_change: mixed
covers_criteria: [SC1, SC4, SC5, SC6]
---

## Failing test (write first)

Create `remote-frontend/src/components/render-parity.test.tsx`:

```tsx
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
```

## Change

### File: `remote-frontend/src/index.css` (EDIT)
- **Anchor:** the file as it exists after task 105 (5 `@import` lines + 3 `@tailwind` lines).
- **Before:** (last `@import` is `base.css`, then a blank line, then `@tailwind base;...`)
- **After:** insert `@import './styles/components.css';` AFTER the `@import './styles/tokens/base.css';` line and BEFORE the blank line + `@tailwind base;`. The full file becomes:
  ```css
  @import './styles/tokens/fonts.css';
  @import './styles/tokens/colors.css';
  @import './styles/tokens/typography.css';
  @import './styles/tokens/spacing.css';
  @import './styles/tokens/base.css';
  @import './styles/components.css';

  @tailwind base;
  @tailwind components;
  @tailwind utilities;
  ```

### File: `remote-frontend/src/components/render-parity.test.tsx` (CREATE)
Create exactly as written in Failing test above.

## Allowed moves

- Edit `remote-frontend/src/index.css` to insert the single `@import './styles/components.css';` line in the position shown.
- Create the `.test.tsx` file exactly as written above.
- No other file may be touched. Do NOT edit `frontend/` (SC9).

## STOP triggers

- `remote-frontend/src/index.css` is not the 5-import + 3-tailwind form (would mean task 105 drifted → STOP, escalate).
- `components.css` does not exist at `remote-frontend/src/styles/components.css` (would mean task 201 drifted → STOP, escalate).
- A component import path does not resolve (would mean tasks 202-207 drifted → STOP, escalate).

## Done when
`WAI_TYPECHECK_CMD="cd remote-frontend && npx tsc --noEmit" WAI_TEST_CMD="cd remote-frontend && npx vitest run src/components/render-parity.test.tsx" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-design-system 208` exits 0.