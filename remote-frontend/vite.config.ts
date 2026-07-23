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
            // Cache `/v1/` REST responses, but EXCLUDE `/v1/shape/*` (the Electric
            // proxy base). Electric shape traffic is long-poll/streaming; letting
            // Workbox's NetworkFirst cache it would serve stale/partial real-time
            // data (adversarial review F3). Shape requests bypass the SW cache.
            urlPattern: ({ url }) =>
              url.pathname.startsWith('/v1/') && !url.pathname.startsWith('/v1/shape'),
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
