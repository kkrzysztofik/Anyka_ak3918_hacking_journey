import react from '@vitejs/plugin-react';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { defineConfig } from 'vite';

const __dirname = dirname(fileURLToPath(import.meta.url));

/**
 * Determines the vendor chunk name for node_modules dependencies.
 * Only vendor code is manually chunked — app source code is left to
 * Vite's default route-based splitting to avoid circular chunk dependencies.
 */
function getChunkName(id: string): string | undefined {
  if (!id.includes('node_modules')) return undefined;

  if (id.includes('@tanstack/react-query')) return 'query-vendor';
  // zod and its resolver must land together: @hookform/resolvers imports
  // `zod/v4/core` while app code imports `zod`, and Rolldown cannot dedupe
  // those across a manual chunk boundary.
  if (id.includes('/zod/') || id.includes('@hookform/resolvers')) return 'validation-vendor';
  if (
    id.includes('lucide-react') ||
    id.includes('@radix-ui') ||
    id.includes('react-hook-form') ||
    id.includes('@hookform') ||
    id.includes('sonner')
  )
    return 'ui-vendor';
  if (id.includes('fast-xml-parser')) return 'utils-vendor';
  // Keep the FLV demuxer out of the eager vendor chunk. LiveVideoPlayer loads
  // it with import(), so it must stay a separate async chunk.
  if (id.includes('mpegts.js')) return 'mpegts';
  return 'vendor';
}

// https://vitejs.dev/config/
export default defineConfig(() => ({
  // Vitest configuration
  test: {
    globals: true,
    environment: 'jsdom',
    setupFiles: ['./src/test/setup.ts'],
    include: [
      'src/**/*.{test,spec}.{js,mjs,cjs,ts,mts,cts,jsx,tsx}',
      'scripts/**/*.{test,spec}.mjs',
    ],
    testTimeout: 15000, // Increase timeout for complex UI interaction tests
    coverage: {
      provider: 'v8',
      reporter: ['text', 'json', 'json-summary', 'html', 'lcov'],
      // Include all source files in coverage report, even if not executed during tests
      // This ensures SonarQube sees all files (with 0% coverage if not tested) rather than missing them
      include: ['src/**/*.{ts,tsx}', 'scripts/**/*.mjs'],
      exclude: [
        'node_modules/',
        'src/test/',
        '**/*.test.{ts,tsx,mjs}',
        '**/*.spec.{ts,tsx,mjs}',
        '**/*.d.ts',
        '**/*.config.{ts,js}',
        '**/types/**',
      ],
      // A regression ratchet, not a target. Measured 2026-08-23 at
      // 91.39 / 89.74 / 88.26 / 78.02 and floored 1-2 points below, because a
      // threshold pinned to the exact measurement goes red on one added
      // uncovered `catch` and is fragile against local-vs-CI drift. A gate
      // that cries wolf gets disabled — which already happened to the
      // sonarqube job (`continue-on-error: true`), leaving this the only
      // enforcement the WebUI has. Raise the floors when coverage rises.
      thresholds: {
        lines: 90,
        statements: 88,
        functions: 87,
        branches: 76,
      },
    },
  },
  plugins: [react()],
  resolve: {
    alias: {
      '@': resolve(__dirname, './src'),
    },
  },
  // Optimize dependencies
  optimizeDeps: {
    include: ['react', 'react-dom', 'lucide-react'],
  },
  server: {
    // NOSONAR: S5332 - Binding to 0.0.0.0 is required for embedded device access
    host: '0.0.0.0', // NOSONAR
    port: 3000,
    proxy: {
      // Proxy ONVIF requests to avoid CORS issues during development
      // NOSONAR: S5332, S4830 - HTTP and secure:false are required for embedded camera devices
      '/onvif': {
        target: process.env.VITE_API_TARGET || 'http://192.168.2.198:80', // NOSONAR
        changeOrigin: true,
        secure: false, // NOSONAR
      },
      // Proxy utilization requests
      '/utilization': {
        target: process.env.VITE_API_TARGET || 'http://192.168.2.198:80', // NOSONAR
        changeOrigin: true,
        secure: false, // NOSONAR
      },
      // Proxy snapshot requests
      '/snapshot': {
        target: process.env.VITE_API_TARGET || 'http://192.168.2.198:80', // NOSONAR
        changeOrigin: true,
        secure: false, // NOSONAR
      },
      // Proxy HTTP-FLV live streams. The FLV server listens on 8080 while the
      // WebUI is served from 80, so this entry rewrites the port for dev only.
      // NOSONAR: S5332, S4830 - HTTP and secure:false are required for embedded camera devices
      '/live': {
        // URL parsing beats string replacement: it also normalizes targets
        // with no explicit port or a trailing slash.
        target: (() => {
          const url = new URL(process.env.VITE_API_TARGET || 'http://192.168.2.198:80'); // NOSONAR
          url.port = '8080';
          return url.toString();
        })(),
        changeOrigin: true,
        secure: false, // NOSONAR
      },
    },
  },
  build: {
    // Modern browsers only: Chrome 117+, Firefox 119+, Safari 17.4+, Edge 117+.
    // Was previously set under `esbuild`, which Vite 8 ignores (Oxc replaced it).
    target: 'es2024',
    outDir: '../../SD_card_contents/anyka_hack/onvif/www',
    emptyOutDir: true,
    sourcemap: false,
    rolldownOptions: {
      output: {
        minify: {
          compress: {
            dropConsole: true,
            dropDebugger: true,
          },
        },
        manualChunks: (id) => getChunkName(id),
        chunkFileNames: 'js/[name]-[hash].js',
        entryFileNames: 'js/[name]-[hash].js',
        assetFileNames: (info) =>
          info.names?.[0]?.endsWith('.css')
            ? 'css/[name]-[hash][extname]'
            : 'assets/[name]-[hash][extname]',
      },
    },
    // `vendor` (react-dom + react-router) is legitimately ~310 kB raw and is
    // not further splittable without hurting caching. Set above it so this
    // warning means something when it fires.
    chunkSizeWarningLimit: 350,
    // Oxc is the Vite 8 default minifier; Terser was an explicit opt-in and is
    // markedly slower. Oxc does not support property mangling, which this
    // project never used.
    minify: 'oxc',
  },
}));
