import react from '@vitejs/plugin-react';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { defineConfig } from 'vite';
import viteCompression from 'vite-plugin-compression';

const __dirname = dirname(fileURLToPath(import.meta.url));

// https://vitejs.dev/config/
export default defineConfig(({ mode }) => ({
  // Vitest configuration
  test: {
    globals: true,
    environment: 'jsdom',
    setupFiles: ['./src/test/setup.ts'],
    include: ['src/**/*.{test,spec}.{js,mjs,cjs,ts,mts,cts,jsx,tsx}'],
    coverage: {
      provider: 'v8',
      reporter: ['text', 'json', 'json-summary', 'html', 'lcov'],
      exclude: ['node_modules/', 'src/test/'],
    },
  },
  plugins: [
    react(),
    viteCompression({ algorithm: 'gzip' }),
    viteCompression({ algorithm: 'brotliCompress', ext: '.br' }),
  ],
  resolve: {
    alias: {
      '@': resolve(__dirname, './src'),
    },
  },
  esbuild: {
    // Use esbuild for TypeScript compilation (faster than tsc)
    // Target ES2024 for modern browser support (Chrome 117+, Firefox 119+, Safari 17.4+, Edge 117+)
    target: 'es2024',
    // Enable type checking in type-check mode
    ...(mode === 'type-check' && {
      logLevel: 'warning',
    }),
  },
  // Define environment variables
  define: {
    __APP_VERSION__: '"1.0.0"',
  },
  // Optimize dependencies
  optimizeDeps: {
    include: ['react', 'react-dom', 'react-router-dom', 'axios', 'lucide-react'],
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
    outDir: '../../SD_card_contents/anyka_hack/onvif/www',
    emptyOutDir: true,
    sourcemap: true,
    rollupOptions: {
      output: {
        manualChunks: (id) => {
          // TanStack Query and related
          if (id.includes('@tanstack/react-query')) {
            return 'query-vendor';
          }

          // Router libraries
          if (id.includes('react-router-dom')) {
            return 'router-vendor';
          }

          // UI libraries
          if (id.includes('lucide-react')) {
            return 'ui-vendor';
          }

          // HTTP and networking
          if (id.includes('axios')) {
            return 'http-vendor';
          }

          // XML parsing and utilities
          if (id.includes('fast-xml-parser') || id.includes('dompurify')) {
            return 'utils-vendor';
          }

          // ONVIF services and related components
          if (id.includes('/services/') || id.includes('onvif')) {
            return 'onvif-services';
          }

          // Device management components
          if (id.includes('DeviceService') || id.includes('SystemInfo')) {
            return 'device-components';
          }

          // Video and PTZ components
          if (id.includes('node_modules')) {
            return 'camera-components';
          }

          // Store slices
          if (id.includes('/store/slices/')) {
            return 'store-slices';
          }

          // Utilities and helpers
          if (id.includes('/utils/') || id.includes('/config/')) {
            return 'app-utils';
          }

          // Default chunk for other modules
          if (id.includes('node_modules')) {
            return 'vendor';
          }
        },
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
    // Optimize chunk size
    chunkSizeWarningLimit: 1000,
    // Enable minification
    minify: 'terser',
    terserOptions: {
      compress: {
        drop_console: true,
        drop_debugger: true,
      },
    },
  },
}));
