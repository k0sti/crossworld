import { defineConfig } from 'vite'
import path from 'path'
import { copyFileSync, mkdirSync, existsSync, readdirSync, readFileSync, statSync } from 'fs'

function copyAssetsPlugin() {
  return {
    name: 'copy-assets',
    configureServer(server: any) {
      const assetsRoot = path.resolve(__dirname, '../../assets')
      server.middlewares.use((req: any, res: any, next: any) => {
        if (req.url?.startsWith('/crossworld/assets/')) {
          const url = req.url.split('?')[0]
          const assetPath = url.replace('/crossworld/assets/', '')
          const fullPath = path.join(assetsRoot, assetPath)
          if (existsSync(fullPath)) {
            const content = readFileSync(fullPath)
            if (fullPath.endsWith('.json')) res.setHeader('Content-Type', 'application/json')
            else if (fullPath.endsWith('.vox')) res.setHeader('Content-Type', 'application/octet-stream')
            else if (fullPath.endsWith('.glb')) res.setHeader('Content-Type', 'model/gltf-binary')
            else if (fullPath.endsWith('.webp')) res.setHeader('Content-Type', 'image/webp')
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
      mkdirSync(outDir, { recursive: true })

      for (const f of ['avatars.json', 'models.json', 'materials.json']) {
        const src = path.join(assetsRoot, f)
        if (existsSync(src)) copyFileSync(src, path.join(outDir, f))
      }

      for (const dir of ['models/vox', 'models/glb', 'textures5']) {
        const sourceDir = path.join(assetsRoot, dir)
        const targetDir = path.join(outDir, dir)
        mkdirSync(targetDir, { recursive: true })
        if (existsSync(sourceDir)) {
          for (const file of readdirSync(sourceDir)) {
            const srcPath = path.join(sourceDir, file)
            if (statSync(srcPath).isFile()) {
              copyFileSync(srcPath, path.join(targetDir, file))
            }
          }
        }
      }
    }
  }
}

export default defineConfig({
  base: '/crossworld/',
  plugins: [copyAssetsPlugin()],
  worker: { format: 'es' },
  optimizeDeps: {
    esbuildOptions: { target: 'esnext' }
  },
  resolve: {
    alias: {
      '@workspace/wasm-world': path.resolve(__dirname, '../wasm-world/crossworld-world.js'),
      '@workspace/wasm-cube': path.resolve(__dirname, '../wasm-cube/cube.js'),
      '@assets': path.resolve(__dirname, '../../assets')
    }
  },
  server: {
    host: '0.0.0.0',
    port: 5174,
    fs: { allow: ['..', '../..'] }
  }
})
