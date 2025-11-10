import * as logger from './logger';
import cubeInit from 'cube'
import * as cubeWasm from 'cube'
import type { CubeCoord } from '../types/cube-coord'
import { ensureCubeWasmInitialized } from './cubeWasm'

/**
 * Ensure WASM is initialized
 * @deprecated Use ensureCubeWasmInitialized from ./cubeWasm instead
 */
export async function ensureWasmInitialized(): Promise<void> {
  await ensureCubeWasmInitialized()
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
 * Load world from CSM text (this function is deprecated - CSM loading should go through GeometryController)
 * For now, this is a stub that validates the CSM but doesn't load it into the old model system
 */
export async function loadModelFromCSM(
  csmText: string,
  _modelId: string = 'world',
  _totalDepth: number
): Promise<void> {
  await ensureWasmInitialized()
  try {
    // Validate CSM syntax using the new API
    // @ts-ignore - WASM module exports validateCsm
    const error = cubeWasm.validateCsm(csmText)
    if (error) {
      throw new Error(error.error)
    }
    logger.log('common', '[CSM] CSM validated successfully')
    // Note: Actual loading should be done through GeometryController.setRoot()
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
 *
 * DEPRECATED: This function used the old global octree API.
 * Use raycastWasm() from cubeWasm.ts with a WasmCube instance instead.
 *
 * @param modelId Model identifier
 * @param pos Ray origin in normalized [0, 1] cube space {x, y, z}
 * @param dir Ray direction (will be normalized) {x, y, z}
 * @returns RaycastResult if hit, null otherwise
 */
export async function raycastOctree(
  _modelId: string,
  _pos: Vec3,
  _dir: Vec3
): Promise<RaycastResult | null> {
  logger.warn('common', '[Raycast] raycastOctree is deprecated - use raycastWasm with WasmCube instance')
  return null
}
