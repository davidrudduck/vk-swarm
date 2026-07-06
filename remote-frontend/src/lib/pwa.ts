import { Workbox } from 'workbox-window';

export function registerSW() {
  if (!('serviceWorker' in navigator)) return;

  const wb = new Workbox('/sw.js');

  let showRefreshPrompt = false;

  wb.addEventListener('waiting', () => {
    showRefreshPrompt = true;
  });

  wb.addEventListener('activated', (event: { isUpdate?: boolean }) => {
    if (event.isUpdate && showRefreshPrompt) {
      window.location.reload();
    }
  });

  wb.register().catch((err: unknown) => {
    console.warn('SW registration failed:', err);
  });
}
