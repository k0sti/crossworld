#!/usr/bin/env bun
/**
 * Script to generate models.json index file from assets folder
 * Traverses assets/models/ directory and creates an index of all model files
 */

import { readdirSync, writeFileSync, statSync } from 'fs'
import { join, relative, extname } from 'path'

interface ModelEntry {
  name: string
  path: string
  type: 'vox' | 'glb'
  size: number
}

interface ModelsIndex {
  generated: string
  count: number
  models: ModelEntry[]
}

function traverseDirectory(dir: string, baseDir: string, models: ModelEntry[]): void {
  const entries = readdirSync(dir, { withFileTypes: true })

  for (const entry of entries) {
    const fullPath = join(dir, entry.name)

    if (entry.isDirectory()) {
      traverseDirectory(fullPath, baseDir, models)
    } else if (entry.isFile()) {
      const ext = extname(entry.name).toLowerCase()
      if (ext === '.vox' || ext === '.glb') {
        const relativePath = relative(baseDir, fullPath)
        const stats = statSync(fullPath)

        models.push({
          name: entry.name.replace(ext, ''),
          path: relativePath,
          type: ext.slice(1) as 'vox' | 'glb',
          size: stats.size
        })
      }
    }
  }
}

function generateModelsIndex(): void {
  const assetsDir = join(process.cwd(), 'assets')
  const modelsDir = join(assetsDir, 'models')
  const outputPath = join(assetsDir, 'models.json')

  const models: ModelEntry[] = []

  console.log('Scanning assets/models directory...')
  traverseDirectory(modelsDir, modelsDir, models)

  // Sort models by name
  models.sort((a, b) => a.name.localeCompare(b.name))

  const index: ModelsIndex = {
    generated: new Date().toISOString(),
    count: models.length,
    models
  }

  console.log(`Found ${models.length} models`)
  console.log(`  - VOX: ${models.filter(m => m.type === 'vox').length}`)
  console.log(`  - GLB: ${models.filter(m => m.type === 'glb').length}`)

  writeFileSync(outputPath, JSON.stringify(index, null, 2))
  console.log(`\nGenerated ${outputPath}`)
}

// Run the script
try {
  generateModelsIndex()
} catch (error) {
  console.error('Error generating models index:', error)
  process.exit(1)
}
