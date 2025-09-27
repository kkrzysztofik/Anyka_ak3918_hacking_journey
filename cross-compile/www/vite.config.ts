import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

// https://vitejs.dev/config/
export default defineConfig(({ mode }) => ({
  plugins: [react()],
  esbuild: {
    // Use esbuild for TypeScript compilation (faster than tsc)
    target: 'es2020',
    // Enable type checking in type-check mode
    ...(mode === 'type-check' && {
      logLevel: 'warning'
    })
  },
  // External dependencies for CDN
  define: {
    // Define environment variables
    __APP_VERSION__: '"1.0.0"',
  },
  // Optimize dependencies
  optimizeDeps: {
    include: [
      'react-router-dom',
      '@reduxjs/toolkit',
      'react-redux',
      'axios',
      'lucide-react'
    ],
    exclude: ['@vite/client', '@vite/env', 'react', 'react-dom']
  },
  server: {
    host: '0.0.0.0',
    port: 3000,
    proxy: {
      // Proxy ONVIF requests to avoid CORS issues during development
      '/onvif': {
        target: 'http://192.168.1.100:8081',
        changeOrigin: true,
        secure: false,
      },
      // Proxy utilization requests
      '/utilization': {
        target: 'http://192.168.1.100:8081',
        changeOrigin: true,
        secure: false,
      },
      // Proxy snapshot requests
      '/snapshot': {
        target: 'http://192.168.1.100:8081',
        changeOrigin: true,
        secure: false,
      }
    }
  },
  build: {
    outDir: '../../SD_card_contents/anyka_hack/web_interface/www',
    sourcemap: true,
    rollupOptions: {
      external: ['react', 'react-dom'],
      output: {
        globals: {
          'react': 'React',
          'react-dom': 'ReactDOM'
        },
        manualChunks: (id) => {
          // Redux toolkit and related
          if (id.includes('@reduxjs/toolkit') || id.includes('react-redux')) {
            return 'redux-vendor';
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
          if (id.includes('VideoFeed') || id.includes('PTZControls')) {
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
        chunkFileNames: (chunkInfo) => {
          const facadeModuleId = chunkInfo.facadeModuleId ? chunkInfo.facadeModuleId.split('/').pop() : 'chunk';
          return `js/[name]-[hash].js`;
        },
        entryFileNames: 'js/[name]-[hash].js',
        assetFileNames: (assetInfo) => {
          const info = assetInfo.name.split('.');
          const ext = info[info.length - 1];
          if (/\.(css)$/.test(assetInfo.name)) {
            return `css/[name]-[hash].${ext}`;
          }
          return `assets/[name]-[hash].${ext}`;
        }
      }
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
  }
}))
