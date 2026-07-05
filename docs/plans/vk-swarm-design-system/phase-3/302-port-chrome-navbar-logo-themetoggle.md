---
id: "302"
phase: 3
title: Port Chrome (Navbar/Logo/ThemeToggle/NavIcon/NavTab/useBreakpoint)
status: ready
depends_on: ["202","207"]
parallel: false
conflicts_with: []
files:
  - remote-frontend/src/ui/chrome/Chrome.tsx
  - remote-frontend/src/ui/chrome/icons.tsx
  - remote-frontend/src/ui/chrome/useBreakpoint.ts
  - remote-frontend/src/ui/chrome/index.ts
  - remote-frontend/src/ui/chrome/chrome.test.tsx
irreversible: false
scope_test: "remote-frontend/src/ui/chrome/chrome.test.tsx"
allowed_change: create
covers_criteria: [SC7]
---

## Sibling alignment

Read `design-source/ui_kits/vk-swarm-app/chrome.jsx` (132 lines). It defines `Icon` (svg 24x24 stroke currentColor), `ICONS` map (plus/search/server/folder/activity/settings/menu/git/bolt/sun/moon as JSX fragments), `useBreakpoint()` hook (mobile<640/tablet640-1023/desktop≥1024 via resize listener), `Logo({compact})` (vks-wordmark with `.vk`+`.swarm` spans), `ThemeToggle({theme,onToggle})` (ghost icon button 34x34, sun/moon), `Navbar({project,view,onView,onNewTask,theme,onToggleTheme,onOpenSettings})` (header with borderBottom, 48px row + nav row with 3 NavTab: board/nodes/processes), `NavIcon({icon,title,onClick})` (ghost icon 34x34), `NavTab({active,onClick,icon,label})` (button, borderBottom 2px primary/transparent). The TS port splits these into modules: `icons.tsx` (Icon + ICONS), `useBreakpoint.ts` (hook), `Chrome.tsx` (Logo, ThemeToggle, Navbar, NavIcon, NavTab). Replace `window.Icon`/`window.ICONS` references with direct imports. Record any divergence in the ledger.

## Failing test (write first)

Create `remote-frontend/src/ui/chrome/chrome.test.tsx`:

```tsx
import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { Navbar, Logo, ThemeToggle, NavIcon, NavTab, Icon, ICONS } from './index';

describe('Chrome (SC7)', () => {
  it('Logo renders vks-wordmark with .vk and .swarm spans', () => {
    const { container } = render(<Logo />);
    expect(container.firstChild).toHaveClass('vks-wordmark');
    expect(container.querySelector('.vk')).toBeTruthy();
    expect(container.querySelector('.swarm')).toBeTruthy();
  });

  it('ThemeToggle emits a ghost icon button that calls onToggle on click', () => {
    const onToggle = vi.fn();
    const { container } = render(<ThemeToggle theme="dark" onToggle={onToggle} />);
    const btn = container.querySelector('button')!;
    expect(btn).toHaveClass('vks-btn--ghost');
    fireEvent.click(btn);
    expect(onToggle).toHaveBeenCalled();
  });

  it('Navbar renders 3 NavTabs (Board/Nodes/Processes) and calls onView', () => {
    const onView = vi.fn();
    render(<Navbar project="proj" view="board" onView={onView} onNewTask={() => {}} theme="dark" onToggleTheme={() => {}} onOpenSettings={() => {}} />);
    expect(screen.getByText('Board')).toBeTruthy();
    expect(screen.getByText('Nodes')).toBeTruthy();
    expect(screen.getByText('Processes')).toBeTruthy();
    fireEvent.click(screen.getByText('Nodes'));
    expect(onView).toHaveBeenCalledWith('nodes');
  });

  it('Navbar renders the New Task primary button calling onNewTask', () => {
    const onNewTask = vi.fn();
    render(<Navbar project="p" view="board" onView={() => {}} onNewTask={onNewTask} theme="dark" onToggleTheme={() => {}} onOpenSettings={() => {}} />);
    fireEvent.click(screen.getByText(/Task/));
    expect(onNewTask).toHaveBeenCalled();
  });

  it('NavIcon renders a ghost icon button', () => {
    const { container } = render(<NavIcon icon={ICONS.plus} title="Add" />);
    expect(container.querySelector('button')).toHaveClass('vks-btn--ghost');
  });

  it('NavTab applies borderBottom primary when active', () => {
    const { container } = render(<NavTab active onClick={() => {}} icon={ICONS.folder} label="L" />);
    const btn = container.querySelector('button') as HTMLElement;
    expect(btn.style.borderBottom).toContain('var(--primary)');
  });
});
```

## Change

### File: `remote-frontend/src/ui/chrome/icons.tsx` (CREATE)
```tsx
import type { ReactElement } from 'react';

export function Icon({ d, size = 16, stroke = 1.6, fill = 'none' }: { d: ReactElement; size?: number; stroke?: number; fill?: string }) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill={fill} stroke="currentColor"
      strokeWidth={stroke} strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      {d}
    </svg>
  );
}

export const ICONS = {
  plus: <><path d="M12 5v14M5 12h14" /></>,
  search: <><circle cx="11" cy="11" r="7" /><path d="M21 21l-4.3-4.3" /></>,
  server: <><rect x="3" y="4" width="18" height="7" rx="1.5" /><rect x="3" y="13" width="18" height="7" rx="1.5" /><path d="M7 7.5h.01M7 16.5h.01" /></>,
  folder: <><path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" /></>,
  activity: <><path d="M22 12h-4l-3 9L9 3l-3 9H2" /></>,
  settings: <><circle cx="12" cy="12" r="3" /><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-2.82 1.17V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 8 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.6 14H4.5a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 6 8.6a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 10 4.6h.09A1.65 1.65 0 0 0 11.4 3h.09a2 2 0 0 1 4 0v.09A1.65 1.65 0 0 0 16 4.6a1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 8z" /></>,
  menu: <><path d="M4 6h16M4 12h16M4 18h16" /></>,
  git: <><circle cx="6" cy="6" r="2.5" /><circle cx="6" cy="18" r="2.5" /><circle cx="18" cy="9" r="2.5" /><path d="M6 8.5v7M18 11.5c0 4-6 1.5-6 4.5" /></>,
  bolt: <><path d="M13 2L4.5 13H11l-1 9 8.5-11H12z" /></>,
  sun: <><circle cx="12" cy="12" r="4" /><path d="M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4" /></>,
  moon: <><path d="M21 12.8A9 9 0 1 1 11.2 3a7 7 0 0 0 9.8 9.8z" /></>,
} as const;
```

### File: `remote-frontend/src/ui/chrome/useBreakpoint.ts` (CREATE)
```ts
import { useState, useEffect } from 'react';

export type Breakpoint = 'mobile' | 'tablet' | 'desktop';

export function useBreakpoint(): Breakpoint {
  const get = () => (typeof window === 'undefined' ? 'desktop'
    : window.innerWidth < 640 ? 'mobile'
    : window.innerWidth < 1024 ? 'tablet' : 'desktop');
  const [bp, setBp] = useState<Breakpoint>(get);
  useEffect(() => {
    const on = () => setBp(get());
    window.addEventListener('resize', on);
    return () => window.removeEventListener('resize', on);
  }, []);
  return bp;
}
```

### File: `remote-frontend/src/ui/chrome/Chrome.tsx` (CREATE)
TypeScript port of `Logo`, `ThemeToggle`, `Navbar`, `NavIcon`, `NavTab` from `chrome.jsx`. Imports `Icon`, `ICONS` from `./icons` and `useBreakpoint` from `./useBreakpoint`. Each component preserves the inline-style approach of the source. `NavbarProps { project: string; view: 'board' | 'nodes' | 'processes'; onView: (v: 'board' | 'nodes' | 'processes') => void; onNewTask: () => void; theme: 'dark' | 'light'; onToggleTheme: () => void; onOpenSettings: () => void }`.

### File: `remote-frontend/src/ui/chrome/index.ts` (CREATE)
`export { Navbar, Logo, ThemeToggle, NavIcon, NavTab } from './Chrome'; export { Icon, ICONS } from './icons'; export { useBreakpoint } from './useBreakpoint'; export type { Breakpoint } from './useBreakpoint';`

### File: `remote-frontend/src/ui/chrome/chrome.test.tsx` (CREATE)
Create exactly as written in Failing test above.

## Allowed moves

- Create the 5 files as specified.
- Preserve the inline-style approach of the source JSX verbatim (do NOT rewrite as Tailwind).
- No other file may be touched. Do NOT edit `frontend/` (SC9).

## STOP triggers

- The design-source JSX differs from the recorded version.
- `vks-btn` / `vks-btn--ghost` / `vks-btn--icon` / `vks-btn--primary` / `vks-btn--sm` / `vks-wordmark` classes are absent from `components.css` (task 201 drift → STOP).

## Done when
`WAI_TYPECHECK_CMD="cd remote-frontend && npx tsc --noEmit" WAI_TEST_CMD="cd remote-frontend && npx vitest run src/ui/chrome/chrome.test.tsx" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-design-system 302` exits 0.