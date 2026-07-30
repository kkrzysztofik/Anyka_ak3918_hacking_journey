import react from '@vitejs/plugin-react';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { defineConfig } from 'vite';

const __dirname = dirname(fileURLToPath(import.meta.url));

/**
 * Determines the chunk name for a given module ID.
 * This function encapsulates the chunking strategy to reduce cognitive complexity.
 * @param id - The module ID to determine the chunk for
 * @returns The chunk name or undefined for default chunking
 */
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
  if (id.includes('fast-xml-parser') || id.includes('dompurify')) return 'utils-vendor';
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
    },
  },
  plugins: [react()],
  resolve: {
    alias: {
      '@': resolve(__dirname, './src'),
    },
  },
  // Define environment variables
  define: {
    __APP_VERSION__: '"1.0.0"',
  },
  // Optimize dependencies
  optimizeDeps: {
    include: ['react', 'react-dom', 'react-router-dom', 'lucide-react'],
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
        chunkFileNames: () => {
          return `js/[name]-[hash].js`;
        },
        entryFileNames: 'js/[name]-[hash].js',
        assetFileNames: (assetInfo) => {
          // Use names array (preferred) or fall back to name if names is not available
          // NOSONAR: S1874 - name is still needed as fallback for older Rollup versions
          const assetName =
            assetInfo.names?.[0] ?? (assetInfo as { name?: string }).name ?? 'asset'; // NOSONAR
          const info = assetName.split('.');
          const ext = info[info.length - 1];
          if (/\.(css)$/.test(assetName)) {
            return `css/[name]-[hash].${ext}`;
          }
          return `assets/[name]-[hash].${ext}`;
        },
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
