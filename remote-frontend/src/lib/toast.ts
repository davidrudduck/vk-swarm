import { toast, Toaster } from 'sonner';

export function toastError(
  message: string,
  retry?: { label?: string; onClick: () => void },
) {
  toast.error(message, {
    action: retry
      ? { label: retry.label ?? 'Retry', onClick: retry.onClick }
      : undefined,
  });
}

export function toastSuccess(
  message: string,
  undo?: { label?: string; onClick: () => void },
) {
  toast.success(message, {
    action: undo
      ? { label: undo.label ?? 'Undo', onClick: undo.onClick }
      : undefined,
  });
}

export { toast, Toaster };
