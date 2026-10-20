import path from 'node:path'
import { defineConfig } from 'vite'
import tailwindcss from '@tailwindcss/vite'
import vue from '@vitejs/plugin-vue'

export default defineConfig({
  plugins: [vue(), tailwindcss()],
  resolve: {
    alias: {
      '@': path.resolve(import.meta.dirname, './src'),
    },
  },
  server: {
    port: 5174,
    proxy: {
      '/api/tv-scanner': {
        target: 'https://scanner.tradingview.com',
        changeOrigin: true,
        rewrite: (p) => p.replace(/^\/api\/tv-scanner/, ''),
      },
    },
  },
})
