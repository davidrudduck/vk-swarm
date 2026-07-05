---
id: "104"
phase: 1
title: Port texture utilities DOM test
status: ready
depends_on: ["103"]
parallel: false
conflicts_with: []
files:
  - remote-frontend/src/styles/tokens/textures.test.tsx
irreversible: false
scope_test: "remote-frontend/src/styles/tokens/textures.test.tsx"
allowed_change: create
covers_criteria: [SC3]
---

## Failing test (write first)

Create `remote-frontend/src/styles/tokens/textures.test.tsx`:

```tsx
// @vitest-environment node
import { describe, it, expect } from 'vitest';
import { render } from '@testing-library/react';
import { readFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const base = readFileSync(join(__dirname, 'base.css'), 'utf-8');

describe('texture utilities (SC3)', () => {
  it('defines all 8 texture utility classes in base.css', () => {
    for (const cls of [
      '.vks-diagonal-lines', '.vks-ansi-dither', '.vks-ansi-weave',
      '.vks-ansi-grid', '.vks-scanlines', '.vks-dashed',
      '.vks-wordmark', '.vks-eyebrow',
    ]) {
      expect(base).toContain(cls);
    }
  });

  it('.vks-scanlines adds an ::after pseudo-element', () => {
    expect(base).toMatch(/\.vks-scanlines::after/);
  });

  it('.vks-wordmark styles .vk and .swarm spans', () => {
    expect(base).toContain('.vks-wordmark .vk');
    expect(base).toContain('.vks-wordmark .swarm');
  });

  it('renders a div with the vks-ansi-dither class', () => {
    const { container } = render(<div className="vks-ansi-dither" data-testid="tex" />);
    expect(container.firstChild).toHaveClass('vks-ansi-dither');
  });

  it('renders a vks-wordmark with .vk and .swarm children', () => {
    const { container } = render(
      <span className="vks-wordmark">
        <span className="vk">vk</span>
        <span className="swarm">swarm</span>
      </span>
    );
    expect(container.querySelector('.vk')).toBeTruthy();
    expect(container.querySelector('.swarm')).toBeTruthy();
  });
});
```

## Change

### File: `remote-frontend/src/styles/tokens/textures.test.tsx` (CREATE)
Create exactly as written above. This test asserts (a) the 8 texture utility classes are present in `base.css` (which task 103 copied), (b) `.vks-scanlines::after` exists, (c) `.vks-wordmark .vk`/`.swarm` selectors exist, and (d) two DOM renders exercise the classes (using `@testing-library/react` already a devDep from task 100).

No CSS file is created or edited in this task — `base.css` (copied in task 103) already contains the texture utilities. This task ONLY adds the DOM-level test that gives SC3 its verifiable hook.

## Allowed moves

- Create `remote-frontend/src/styles/tokens/textures.test.tsx` exactly as written above.
- No other file may be touched. Do NOT edit `frontend/` (SC9). Do NOT re-copy `base.css` (task 103 owns it).

## STOP triggers

- `base.css` does not contain one of the 8 texture utility classes (would mean task 103 copied the wrong file → STOP, escalate to task 103).
- `@testing-library/react` is not installed (would mean task 100 drifted → STOP, escalate).

## Done when
`WAI_TYPECHECK_CMD="cd remote-frontend && npx tsc --noEmit" WAI_TEST_CMD="cd remote-frontend && npx vitest run src/styles/tokens/textures.test.tsx" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-design-system 104` exits 0.