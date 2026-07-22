---
id: "200"
phase: 2
title: Install vite-plugin-pwa + configure SW/manifest
status: done
depends_on: ["104"]
parallel: false
conflicts_with: []
files:
  - remote-frontend/package.json
  - remote-frontend/src/lib/pwa.test.ts
  - remote-frontend/vite.config.ts
  - remote-frontend/src/lib/pwa.ts
irreversible: false
scope_test: "remote-frontend/src/lib/pwa"
allowed_change: mixed
covers_criteria: [SC6, SC11]
---

## Failing test (write first)

Create `remote-frontend/src/lib/pwa.test.ts`:

```ts
// @vitest-environment node
import { describe, it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

describe('PWA registration module (SC6)', () => {
  it('exports registerSW function', () => {
    const source = readFileSync(join(__dirname, 'pwa.ts'), 'utf-8');
    expect(source).toContain('export function registerSW');
    expect(source).toContain('workbox-window');
    expect(source).toContain('registerSW');
  });

  it('refreshPrompt: update-need event', () => {
    const source = readFileSync(join(__dirname, 'pwa.ts'), 'utf-8');
    expect(source).toContain("'need-update'");
    expect(source).toContain('showRefreshPrompt');
  });
});

describe('vite.config.ts PWA plugin (SC6, SC11)', () => {
  it('has VitePWA plugin config', () => {
    const source = readFileSync(join(__dirname, '../../vite.config.ts'), 'utf-8');
    expect(source).toContain('VitePWA');
    expect(source).toContain('VK Swarm Console');
    expect(source).toContain('theme_color');
    expect(source).toContain('#0f172a');
    expect(source).toContain('manifest');
    expect(source).toContain('workbox');
    expect(source).toContain('injectManifest');
  });
});
```

## Change

### File: `remote-frontend/package.json` (EDIT — add dependencies)

- **Anchor:** `"dependencies"` block, line `"tailwind-merge": "^2.2.0"`
- **Before:**
  ```
  "tailwind-merge": "^2.2.0"
  ```
- **After:**
  ```
  "tailwind-merge": "^2.2.0",
  "workbox-window": "^8.0.0"
  ```

- **Anchor:** `"devDependencies"` block, line `"vite": "^8.0.7"`
- **Before:**
  ```
  "vite": "^8.0.7",
  ```
- **After:**
  ```
  "vite": "^8.0.7",
  "vite-plugin-pwa": "^1.0.1",
  ```

Then run `cd remote-frontend && npm install`.

### File: `remote-frontend/vite.config.ts` (EDIT — add VitePWA plugin)

- **Anchor:** the existing config object (entire file)

Before:
```ts
import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import path from 'path'

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
      shared: path.resolve(__dirname, './src/types/shared'),
    },
  },
  server: {
    port: 3002
  },
  test: {
    environment: 'jsdom',
    setupFiles: ['./src/setupTests.ts'],
    globals: true,
  },
})
```

After:
```ts
import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import { VitePWA } from 'vite-plugin-pwa'
import path from 'path'

export default defineConfig({
  plugins: [
    react(),
    VitePWA({
      registerType: 'autoUpdate',
      injectManifest: {
        injectionPoint: undefined,
      },
      workbox: {
        globPatterns: ['**/*.{js,css,html,ico,png,svg,woff2}'],
        runtimeCaching: [
          {
            urlPattern: ({ url }) => url.pathname.startsWith('/v1/'),
            handler: 'NetworkFirst',
            options: {
              cacheName: 'api-cache',
              expiration: { maxEntries: 100, maxAgeSeconds: 300 },
            },
          },
          {
            urlPattern: ({ url }) => url.pathname.startsWith('/assets/'),
            handler: 'CacheFirst',
            options: {
              cacheName: 'asset-cache',
              expiration: { maxEntries: 200, maxAgeSeconds: 604800 },
            },
          },
        ],
      },
      manifest: {
        name: 'VK Swarm Console',
        short_name: 'VK Swarm',
        theme_color: '#0f172a',
        background_color: '#0f172a',
        display: 'standalone',
        icons: [
          { src: '/icons/icon-192.png', sizes: '192x192', type: 'image/png' },
          {
            src: '/icons/icon-512.png',
            sizes: '512x512',
            type: 'image/png',
            purpose: 'maskable',
          },
        ],
      },
    }),
  ],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
      shared: path.resolve(__dirname, './src/types/shared'),
    },
  },
  server: {
    port: 3002,
  },
  test: {
    environment: 'jsdom',
    setupFiles: ['./src/setupTests.ts'],
    globals: true,
  },
})
```

### File: `remote-frontend/src/lib/pwa.ts` (CREATE)

```ts
import { Workbox, type WorkboxLifecycleWaitingEvent } from 'workbox-window';

export function registerSW() {
  if (!('serviceWorker' in navigator)) return;

  const wb = new Workbox('/sw.js');

  let showRefreshPrompt = false;

  wb.addEventListener('waiting', (_event: WorkboxLifecycleWaitingEvent) => {
    showRefreshPrompt = true;
  });

  wb.addEventListener('activated', (event: WorkboxLifecycleWaitingEvent) => {
    if (event.isUpdate && showRefreshPrompt) {
      window.location.reload();
    }
  });

  wb.register().catch((err) => {
    console.warn('SW registration failed:', err);
  });
}
```

## Allowed moves

- Edit `remote-frontend/package.json` in two places: add `workbox-window` to dependencies, add `vite-plugin-pwa` to devDependencies. Run `npm install`.
- Edit `remote-frontend/vite.config.ts`: add `VitePWA` import and plugin config exactly as shown.
- Create `remote-frontend/src/lib/pwa.ts` with the exact code above.
- Create `remote-frontend/src/lib/pwa.test.ts` with the exact code above.
- Do NOT create PWA icons. Placeholder SVGs are enough for dev — the `npm run build` will still produce the SW and manifest. If icons are missing, the build won't fail (PWA plugin treats missing icons as warnings). The implementer creates empty placeholder PNGs at `remote-frontend/public/icons/` if needed for build to pass.
- Do NOT call `registerSW()` from main.tsx yet — the PWA is configured but not auto-registered. The SW will be loaded in dev mode by vite-plugin-pwa automatically.

## STOP triggers

- `vite-plugin-pwa` version `^1.0.1` doesn't resolve with vite@8. If npm complains about peer dependency conflicts, try `vite-plugin-pwa@^0.21.0` (has wider vite peer range). Record the version used in the decisions ledger.
- `workbox-window` v8 requires node >= 18 (the workspace has it). If the import style (`import { Workbox }`) doesn't match the installed version's exports, check the package.json `exports` field and adjust the import.
- The vite.config.ts Before text doesn't match exactly — verify with `git diff`.

## Done when
`WAI_TYPECHECK_CMD="cd remote-frontend && npx tsc --noEmit" WAI_TEST_CMD="cd remote-frontend && npx vitest run src/lib/pwa" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-hive-ui-polish 200` exits 0.