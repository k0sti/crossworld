import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import path from 'path'
import { copyFileSync, mkdirSync, existsSync, readdirSync, readFileSync } from 'fs'

// Custom plugin to copy assets from project root and serve in dev
function copyAssetsPlugin() {
  return {
    name: 'copy-assets',
    configureServer(server) {
      // Serve assets from project root during dev
      const assetsRoot = path.resolve(__dirname, '../../assets')

      server.middlewares.use((req, res, next) => {
        if (req.url?.startsWith('/crossworld/assets/')) {
          // Strip query parameters
          const url = req.url.split('?')[0]
          const assetPath = url.replace('/crossworld/assets/', '')
          const fullPath = path.join(assetsRoot, assetPath)

          if (existsSync(fullPath)) {
            // Serve the file directly
            const content = readFileSync(fullPath)

            // Set appropriate content type
            if (fullPath.endsWith('.json')) {
              res.setHeader('Content-Type', 'application/json')
            } else if (fullPath.endsWith('.vox')) {
              res.setHeader('Content-Type', 'application/octet-stream')
            } else if (fullPath.endsWith('.glb')) {
              res.setHeader('Content-Type', 'model/gltf-binary')
            }

            res.end(content)
            return
          }
        }
        next()
      })
    },
    writeBundle() {
      const assetsRoot = path.resolve(__dirname, '../../assets')
      const outDir = path.resolve(__dirname, 'dist/assets')

      // Create output directory
      mkdirSync(outDir, { recursive: true })

      // Copy models.json
      copyFileSync(
        path.join(assetsRoot, 'models.json'),
        path.join(outDir, 'models.json')
      )

      // Copy model directories
      const modelTypes = ['vox', 'glb']
      for (const type of modelTypes) {
        const sourceDir = path.join(assetsRoot, 'models', type)
        const targetDir = path.join(outDir, 'models', type)
        mkdirSync(targetDir, { recursive: true })

        // Copy all files
        if (existsSync(sourceDir)) {
          const files = readdirSync(sourceDir)
          files.forEach(filename => {
            const sourcePath = path.join(sourceDir, filename)
            const targetPath = path.join(targetDir, filename)
            copyFileSync(sourcePath, targetPath)
          })
        }
      }
    }
  }
}

// https://vitejs.dev/config/
export default defineConfig({
  base: '/crossworld/',
  plugins: [
    react(),
    copyAssetsPlugin()
  ],
  worker: {
    format: 'es'
  },
  optimizeDeps: {
    exclude: ['@kixelated/hang'],
    esbuildOptions: {
      target: 'esnext'
    }
  },
  resolve: {
    alias: {
      '@workspace/wasm': path.resolve(__dirname, '../wasm/crossworld-world.js'),
      '@assets': path.resolve(__dirname, '../../assets')
    }
  },
  publicDir: false,
  // Serve assets from project root during dev
  server: {
    fs: {
      allow: ['..', '../..']
    }
  }
})
