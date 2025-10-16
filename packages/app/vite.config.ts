import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import path from 'path'

// https://vitejs.dev/config/
export default defineConfig({
  base: '/crossworld/',
  plugins: [react()],
  worker: {
    format: 'es'
  },
  resolve: {
    alias: {
      '@workspace/wasm': path.resolve(__dirname, '../wasm/crossworld-world.js')
    }
  }
})
