import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

// PWA browser build config — no Tauri dependencies.
export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      // Browser stub replaces the Tauri IPC module.
      '@tauri-apps/api/core': '/src/browser.ts',
    },
  },
  build: {
    outDir: 'dist-browser',
    target: ['es2021', 'chrome100', 'safari13'],
    minify: 'esbuild',
    sourcemap: false,
  },
  server: {
    port: 3000,
    open: true,
  },
})
