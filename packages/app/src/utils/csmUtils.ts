import * as logger from './logger';
import cubeInit from '@workspace/wasm-cube'
import * as cubeWasm from '@workspace/wasm-cube'
import type { CubeCoord } from '../types/cube-coord'

let wasmInitialized = false
let initPromise: Promise<void> | null = null

/**
 * Ensure WASM is initialized
 */
export async function ensureWasmInitialized(): Promise<void> {
  if (wasmInitialized) return

  if (initPromise) {
    await initPromise
    return
  }

  initPromise = cubeInit().then(() => {
    wasmInitialized = true
  })

  await initPromise
}

/**
 * Serialize model to CSM text from geometry controller
 */
export async function getModelCSM(geometryController?: any): Promise<string> {
  if (!geometryController) {
    throw new Error('Geometry controller required for CSM export')
  }

  try {
    const csmText = await geometryController.getCSM()
    return csmText
  } catch (error) {
    logger.error('common', '[CSM] Serialization error:', error)
    throw error
  }
}

/**
 * Load model from CSM text (uses cube WASM module for parsing)
 */
export async function loadModelFromCSM(
  csmText: string,
  modelId: string = 'world',
  totalDepth: number
): Promise<void> {
  await ensureWasmInitialized()
  try {
    const result = (cubeWasm as any).load_model_from_csm(modelId, csmText, totalDepth)
    // Check if result is an error
    if (result && typeof result === 'object' && 'error' in result) {
      throw new Error((result as any).error)
    }
  } catch (error) {
    logger.error('common', '[CSM] Load error:', error)
    throw error
  }
}

/**
 * Get mesh statistics from geometry controller
 */
export async function getModelStats(geometryController?: any): Promise<{
  vertexCount: number
  faceCount: number
  indexCount: number
}> {
  if (!geometryController) {
    logger.warn('common', '[CSM] No geometry controller provided for stats')
    return {
      vertexCount: 0,
      faceCount: 0,
      indexCount: 0
    }
  }

  try {
    const stats = geometryController.getStats()
    return {
      vertexCount: stats.vertices || 0,
      faceCount: stats.triangles || 0,
      indexCount: (stats.triangles || 0) * 3
    }
  } catch (error) {
    logger.error('common', '[CSM] Stats error:', error)
    return {
      vertexCount: 0,
      faceCount: 0,
      indexCount: 0
    }
  }
}

/**
 * Count non-empty lines in CSM code
 */
export function countCSMLines(csmText: string): number {
  return csmText
    .split('\n')
    .filter(line => line.trim() && !line.trim().startsWith('#'))
    .length
}

/**
 * Get CSM code size in bytes
 */
export function getCSMSize(csmText: string): number {
  return new Blob([csmText]).size
}

/**
 * Format bytes to human-readable string
 */
export function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return Math.round(bytes / Math.pow(k, i) * 100) / 100 + ' ' + sizes[i]
}

/**
 * Vec3-like structure for 3D vectors
 */
export interface Vec3 {
  x: number
  y: number
  z: number
}

/**
 * Raycast result from octree
 */
export interface RaycastResult {
  /** Octree coordinates of hit voxel */
  coord: CubeCoord
  /** World position of hit (in normalized [0, 1] space) */
  position: Vec3
  /** Surface normal */
  normal: Vec3
}

/**
 * Cast a ray through the octree and find the first non-empty voxel
 * @param modelId Model identifier
 * @param pos Ray origin in normalized [0, 1] cube space {x, y, z}
 * @param dir Ray direction (will be normalized) {x, y, z}
 * @returns RaycastResult if hit, null otherwise
 */
export async function raycastOctree(
  modelId: string,
  pos: Vec3,
  dir: Vec3
): Promise<RaycastResult | null> {
  await ensureWasmInitialized()
  try {
    const result = (cubeWasm as any).raycast_octree(
      modelId,
      pos.x,
      pos.y,
      pos.z,
      dir.x,
      dir.y,
      dir.z
    )

    // Check if result is null (no hit) or an error
    if (!result) return null
    if (typeof result === 'object' && 'error' in result) {
      throw new Error((result as any).error)
    }

    // Transform flat WASM result into structured result
    return {
      coord: {
        x: result.x,
        y: result.y,
        z: result.z,
        depth: result.depth
      },
      position: {
        x: result.world_x,
        y: result.world_y,
        z: result.world_z
      },
      normal: {
        x: result.normal_x,
        y: result.normal_y,
        z: result.normal_z
      }
    }
  } catch (error) {
    logger.error('common', '[Raycast] Error:', error)
    throw error
  }
}
