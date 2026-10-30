/// <reference types="vitest/config" />
import path from 'node:path'
import tailwindcss from '@tailwindcss/vite'
import react from '@vitejs/plugin-react'
import { defineConfig, type Plugin } from 'vite'

// The daemon the dev server proxies `/api`, `/media` and `/avatars` to: a local
// `scripts/run-local.sh` by default, or a remote library
// (`scripts/run-spa-against-remote.sh`).
const apiTarget = process.env.YARRTUBE_API_URL ?? 'http://localhost:8080'

const PROXIED_PATHS = ['/api', '/media', '/avatars']

/**
 * Answers every proxied request but GET and HEAD with 405, so browsing a real
 * library from the dev server can't change it. Registered before Vite's own
 * middlewares, so blocked requests never reach the proxy.
 */
function readOnlyApi(): Plugin {
  return {
    name: 'yarrtube-read-only-api',
    configureServer(server) {
      server.middlewares.use((req, res, next) => {
        const proxied = PROXIED_PATHS.some((prefix) => req.url?.startsWith(prefix))
        if (proxied && req.method !== 'GET' && req.method !== 'HEAD') {
          res.statusCode = 405
          res.setHeader('Content-Type', 'application/json')
          res.end(JSON.stringify({ error: 'Read-only dev server: writes are disabled' }))
          return
        }
        next()
      })
    },
  }
}

// https://vite.dev/config/
export default defineConfig({
  plugins: [
    react(),
    tailwindcss(),
    process.env.YARRTUBE_API_READ_ONLY === '1' && readOnlyApi(),
  ],
  resolve: {
    alias: {
      '@': path.resolve(import.meta.dirname, 'src'),
    },
  },
  server: {
    proxy: Object.fromEntries(
      PROXIED_PATHS.map((prefix) => [prefix, { target: apiTarget, changeOrigin: true }]),
    ),
  },
  test: {
    environment: 'jsdom',
    globals: true,
    setupFiles: ['./src/test/setup.ts'],
    restoreMocks: true,
    css: false,
  },
})
