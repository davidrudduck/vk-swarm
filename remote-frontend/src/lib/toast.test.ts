import { describe, it, expect, vi, beforeEach } from 'vitest';
import { toastError, toastSuccess, toast, Toaster } from './toast';

vi.mock('sonner', () => ({
  toast: { error: vi.fn(), success: vi.fn(), message: vi.fn() },
  Toaster: () => null,
}));

describe('toast wrapper (SC3, SC4)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('toastError calls toast.error with message', () => {
    toastError('Something broke');
    expect(toast.error).toHaveBeenCalledWith('Something broke', { action: undefined });
  });

  it('toastError passes retry action to toast.error', () => {
    const onClick = vi.fn();
    toastError('Assignment failed', { label: 'Retry', onClick });
    expect(toast.error).toHaveBeenCalledWith('Assignment failed', {
      action: { label: 'Retry', onClick },
    });
  });

  it('toastError uses default label when retry.label is omitted', () => {
    const onClick = vi.fn();
    toastError('Delete failed', { onClick });
    expect(toast.error).toHaveBeenCalledWith('Delete failed', {
      action: { label: 'Retry', onClick },
    });
  });

  it('toastSuccess calls toast.success with message', () => {
    toastSuccess('Task deleted');
    expect(toast.success).toHaveBeenCalledWith('Task deleted', { action: undefined });
  });

  it('toastSuccess passes undo action to toast.success', () => {
    const onClick = vi.fn();
    toastSuccess('Task assigned', { label: 'Undo', onClick });
    expect(toast.success).toHaveBeenCalledWith('Task assigned', {
      action: { label: 'Undo', onClick },
    });
  });

  it('re-exports toast from sonner', () => {
    expect(toast).toBeDefined();
  });

  it('re-exports Toaster from sonner', () => {
    expect(Toaster).toBeDefined();
  });
});