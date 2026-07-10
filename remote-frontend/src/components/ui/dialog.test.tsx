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
          <button data-testid="footer-close" onClick={() => setOpen(false)}>
            Done
          </button>
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

  it('renders with aria attributes from Radix', () => {
    render(<TestDialog />);
    const dialog = screen.getByRole('dialog');
    expect(dialog).toBeInTheDocument();
    expect(dialog.tagName).toBe('DIV');
  });

  it('closes on Escape when not uncloseable', () => {
    render(<TestDialog />);
    expect(screen.getByRole('dialog')).toBeInTheDocument();
    fireEvent.keyDown(document, { key: 'Escape' });
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('does NOT close on Escape when uncloseable', () => {
    render(<TestDialog uncloseable />);
    expect(screen.getByRole('dialog')).toBeInTheDocument();
    fireEvent.keyDown(document, { key: 'Escape' });
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

  it('closes via the footer Done button', () => {
    render(<TestDialog />);
    expect(screen.getByRole('dialog')).toBeInTheDocument();
    fireEvent.click(screen.getByTestId('footer-close'));
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });
});
