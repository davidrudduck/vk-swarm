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
          {
            urlPattern: ({ url }) => ['/', '/login', '/oauth/callback'].includes(url.pathname) || url.pathname.startsWith('/invitations/'),
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
    exclude: ['**/node_modules/**', '**/e2e/**', '**/dist/**', 'scripts/**'],
  },
})
