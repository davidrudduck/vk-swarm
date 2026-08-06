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
        // CRITICAL: without this, generateSW's default navigateFallback
        // ('index.html') registers an UNCONDITIONAL NavigationRoute that
        // answers every top-level navigation — including the OAuth popup's
        // /v1/oauth/{provider}/start and /callback legs — with the precached
        // SPA shell, so the handoff never reaches the server (F-2026-08-03-01/
        // -02; deploy verification 2026-08-06 proved the api-cache exclusion
        // alone was insufficient). /v1/* are server endpoints, never SPA
        // routes: all /v1/ navigations must fall through to the network.
        navigateFallbackDenylist: [/^\/v1\//],
        runtimeCaching: [
          {
            // Cache `/v1/` REST responses, EXCLUDING `/v1/shape/*` (Electric
            // long-poll/streaming — adversarial review F3) and `/v1/oauth/*` (the
            // OAuth redirect chain; SW interception breaks sign-in on hive and
            // node — F-2026-08-03-02). Excluded requests bypass the SW entirely.
            // KEEP THIS ARROW SELF-CONTAINED: Workbox generateSW serializes it
            // into sw.js via toString(); an imported identifier would be
            // undefined at SW runtime. Mirrored + unit-tested in
            // src/lib/swCachePredicate.ts (drift-guarded, see task 102 evidence).
            urlPattern: ({ url }) =>
              url.pathname.startsWith('/v1/') &&
              !url.pathname.startsWith('/v1/shape') &&
              !url.pathname.startsWith('/v1/oauth'),
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
          {
            urlPattern: ({ url }) => {
              const path = url.pathname
              if (path === '/oauth/callback' || (path.startsWith('/invitations/') && path.endsWith('/complete'))) return false
              return ['/', '/login'].includes(path) || path.startsWith('/invitations/')
            },
            handler: 'StaleWhileRevalidate',
            options: {
              cacheName: 'shell-cache',
              expiration: { maxEntries: 10, maxAgeSeconds: 86400 },
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
    restoreMocks: true,
    exclude: ['**/node_modules/**', '**/e2e/**', '**/dist/**', '**/scripts/**'],
  },
})
