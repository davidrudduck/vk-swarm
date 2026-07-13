---
id: "203"
phase: 2
title: "Create dialog a11y tests (role, aria-modal, focus trap, Escape)"
status: ready
depends_on: ["201"]
parallel: false
conflicts_with: []
files:
  - remote-frontend/src/components/ui/dialog.test.tsx
irreversible: false
scope_test: "remote-frontend/src/components/ui/dialog.test.tsx"
allowed_change: create
covers_criteria: [SC3]
---
## Failing test (write first)
This task IS the test file.

## Change
- **File:** remote-frontend/src/components/ui/dialog.test.tsx
- **Anchor:** N/A (new file)
- **Before:** (file does not exist)
- **After:**
```tsx
import { describe, it, expect } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from './dialog';
import { useState } from 'react';

function TestDialog({ uncloseable = false }: { uncloseable?: boolean }) {
  const [open, setOpen] = useState(true);
  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogContent uncloseable={uncloseable}>
        <DialogHeader>
          <DialogTitle>Test Dialog</DialogTitle>
          <DialogDescription>Test description</DialogDescription>
        </DialogHeader>
        <DialogFooter>
          <button onClick={() => setOpen(false)}>Close</button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

describe('Dialog accessibility', () => {
  it('renders with role="dialog"', () => {
    render(<TestDialog />);
    expect(screen.getByRole('dialog')).toBeInTheDocument();
  });

  it('renders with aria-modal="true"', () => {
    render(<TestDialog />);
    expect(screen.getByRole('dialog')).toHaveAttribute('aria-modal', 'true');
  });

  it('closes on Escape when not uncloseable', () => {
    render(<TestDialog />);
    const dialog = screen.getByRole('dialog');
    fireEvent.keyDown(dialog, { key: 'Escape' });
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('does NOT close on Escape when uncloseable', () => {
    render(<TestDialog uncloseable />);
    const dialog = screen.getByRole('dialog');
    fireEvent.keyDown(dialog, { key: 'Escape' });
    expect(screen.getByRole('dialog')).toBeInTheDocument();
  });

  it('does NOT close on overlay click when uncloseable', () => {
    render(<TestDialog uncloseable />);
    const overlay = document.querySelector('[data-state]');
    if (overlay) {
      fireEvent.pointerDown(overlay);
    }
    expect(screen.getByRole('dialog')).toBeInTheDocument();
  });

  it('hides close button when uncloseable', () => {
    render(<TestDialog uncloseable />);
    expect(screen.queryByRole('button', { name: 'Close' })).not.toBeInTheDocument();
  });

  it('shows close button when not uncloseable', () => {
    render(<TestDialog />);
    expect(screen.getByRole('button', { name: 'Close' })).toBeInTheDocument();
  });
});
```

## Allowed moves
Create `remote-frontend/src/components/ui/dialog.test.tsx` with the exact content above.

## STOP triggers
- If Radix dialog does not render `role="dialog"` (should be automatic)
- If `aria-modal` is not present (should be automatic with Radix)
- If Escape handling doesn't work (check Radix version)

## Manual verification (record in decisions-ledger)
```bash
cd remote-frontend && npx vitest run src/components/ui/dialog.test.tsx
# Expected: all 7 tests pass
```

## Done when
- `dialog.test.tsx` exists with 7 tests
- All tests pass
- role="dialog", aria-modal="true", Escape handling, uncloseable behavior all verified
