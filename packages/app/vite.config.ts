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
            } else if (fullPath.endsWith('.webp')) {
              res.setHeader('Content-Type', 'image/webp')
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

      // Copy avatars.json, models.json, and materials.json
      copyFileSync(
        path.join(assetsRoot, 'avatars.json'),
        path.join(outDir, 'avatars.json')
      )
      copyFileSync(
        path.join(assetsRoot, 'models.json'),
        path.join(outDir, 'models.json')
      )
      copyFileSync(
        path.join(assetsRoot, 'materials.json'),
        path.join(outDir, 'materials.json')
      )

      // Copy model and texture directories
      const assetTypes = [
        { dir: 'models/vox', name: 'vox models' },
        { dir: 'models/glb', name: 'glb models' },
        { dir: 'textures5', name: 'textures5' }
      ]

      for (const assetType of assetTypes) {
        const sourceDir = path.join(assetsRoot, assetType.dir)
        const targetDir = path.join(outDir, assetType.dir)
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
      '@workspace/wasm-world': path.resolve(__dirname, '../wasm-world/crossworld-world.js'),
      '@workspace/wasm-cube': path.resolve(__dirname, '../wasm-cube/cube.js'),
      '@assets': path.resolve(__dirname, '../../assets')
    }
  },
  publicDir: 'public',
  // Serve assets from project root during dev
  server: {
    host: '0.0.0.0',  // Listen on all interfaces
    fs: {
      allow: ['..', '../..']
    }
  }
})
