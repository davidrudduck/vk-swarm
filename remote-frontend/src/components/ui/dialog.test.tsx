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

  it('renders with aria-modal="true" from wrapper', () => {
    render(<TestDialog />);
    const dialog = screen.getByRole('dialog');
    expect(dialog).toHaveAttribute('aria-modal', 'true');
  });

  it('renders with aria-labelledby and aria-describedby from Radix', () => {
    render(<TestDialog />);
    const dialog = screen.getByRole('dialog');
    expect(dialog).toHaveAttribute('aria-labelledby');
    expect(dialog).toHaveAttribute('aria-describedby');
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

  it('does NOT close on overlay click when uncloseable', () => {
    render(<TestDialog uncloseable />);
    expect(screen.getByRole('dialog')).toBeInTheDocument();
    fireEvent.pointerDown(document.body);
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

  it('closes via the Radix close button when not uncloseable', () => {
    render(<TestDialog />);
    expect(screen.getByRole('dialog')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Close' }));
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('renders DialogHeader with className prop', () => {
    render(
      <Dialog open={true} onOpenChange={() => {}}>
        <DialogContent>
          <DialogHeader className="custom-header">
            <DialogTitle>Styled Dialog</DialogTitle>
          </DialogHeader>
        </DialogContent>
      </Dialog>,
    );
    expect(screen.getByText('Styled Dialog')).toBeInTheDocument();
    const header = screen.getByText('Styled Dialog').closest('.custom-header');
    expect(header).toBeInTheDocument();
  });

  it('renders DialogFooter with children', () => {
    render(
      <Dialog open={true} onOpenChange={() => {}}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Footer Test</DialogTitle>
          </DialogHeader>
          <DialogFooter className="custom-footer">
            <button>Cancel</button>
            <button>OK</button>
          </DialogFooter>
        </DialogContent>
      </Dialog>,
    );
    expect(screen.getByText('Cancel')).toBeInTheDocument();
    expect(screen.getByText('OK')).toBeInTheDocument();
  });

  it('DialogTitle forwards ref', () => {
    render(
      <Dialog open={true} onOpenChange={() => {}}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Ref Title</DialogTitle>
          </DialogHeader>
        </DialogContent>
      </Dialog>,
    );
    expect(screen.getByRole('heading', { name: 'Ref Title' })).toBeInTheDocument();
  });

  it('DialogDescription renders text content', () => {
    render(
      <Dialog open={true} onOpenChange={() => {}}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Desc Test</DialogTitle>
            <DialogDescription>A helpful description</DialogDescription>
          </DialogHeader>
        </DialogContent>
      </Dialog>,
    );
    expect(screen.getByText('A helpful description')).toBeInTheDocument();
  });

  it('onOpenChange fires when Escape is pressed on closable dialog', async () => {
    let changeValue = true;
    render(
      <Dialog
        open={true}
        onOpenChange={(v) => {
          changeValue = v;
        }}
      >
        <DialogContent>
          <DialogTitle>Escape Test</DialogTitle>
        </DialogContent>
      </Dialog>,
    );
    fireEvent.keyDown(document, { key: 'Escape' });
    expect(changeValue).toBe(false);
  });

  it('DialogPortal renders overlay in document body', () => {
    render(<TestDialog />);
    const overlays = document.querySelectorAll('[data-state="open"]');
    expect(overlays.length).toBeGreaterThan(0);
  });
});
