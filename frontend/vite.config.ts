import { execSync } from 'node:child_process'
import path from 'node:path'
import { defineConfig } from 'vite'
import tailwindcss from '@tailwindcss/vite'
import vue from '@vitejs/plugin-vue'

function getGitCommitHash(): string {
  try {
    return execSync('git rev-parse --short=6 HEAD').toString().trim()
  } catch {
    return process.env.VITE_GIT_COMMIT_HASH || 'dev'
  }
}

export default defineConfig({
  plugins: [vue(), tailwindcss()],
  define: {
    'import.meta.env.VITE_GIT_COMMIT_HASH': JSON.stringify(getGitCommitHash()),
  },
  server: {
    headers: {
      'Cross-Origin-Opener-Policy': 'same-origin-allow-popups',
    },
  },
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
})
