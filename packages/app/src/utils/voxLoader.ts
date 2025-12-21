import * as logger from './logger';
import type { GeometryData } from 'crossworld-world'
import cubeInit from 'cube'
import * as cubeWasm from 'cube'

/** Result from WasmCubeBox.generateMesh() */
interface MeshResult {
  vertices: number[];
  indices: number[];
  normals: number[];
  colors: number[];
  uvs: number[];
  material_ids: number[];
}

/** Loaded vox model with bounds information */
export interface VoxModel {
  geometry: GeometryData;
  /** Model size in voxels (X, Y, Z) */
  size: { x: number; y: number; z: number };
  /** Octree depth */
  depth: number;
  /** Octree size (2^depth) */
  octreeSize: number;
}

/**
 * Load a .vox file from a URL and generate Three.js geometry with bounds
 * @param url URL to the .vox file
 * @param _userNpub Optional user npub for color customization (not used - kept for API compatibility)
 * @returns VoxModel with geometry and bounds information
 */
export async function loadVoxBoxFromUrl(url: string, _userNpub?: string): Promise<VoxModel> {
  // Ensure WASM is initialized
  await cubeInit()

  // Fetch the .vox file
  const response = await fetch(url)
  if (!response.ok) {
    throw new Error(`Failed to fetch .vox file: ${response.statusText}`)
  }

  const arrayBuffer = await response.arrayBuffer()
  const bytes = new Uint8Array(arrayBuffer)

  // Load .vox file into WasmCubeBox (preserves model bounds)
  // @ts-ignore - WASM module exports loadVoxBox
  const wasmCubeBox = cubeWasm.loadVoxBox(bytes)

  // Get model bounds
  const size = {
    x: wasmCubeBox.sizeX,
    y: wasmCubeBox.sizeY,
    z: wasmCubeBox.sizeZ,
  }
  const depth = wasmCubeBox.depth
  const octreeSize = wasmCubeBox.octreeSize

  // Generate mesh from the cube box (null palette = original colors)
  const result = wasmCubeBox.generateMesh(null) as MeshResult | { error: string }

  if ('error' in result) {
    logger.error('common', `[voxLoader] Failed to parse VOX file from ${url}: ${result.error}`)
    throw new Error(`Failed to parse VOX file: ${result.error}`)
  }

  const geometry = {
    vertices: new Float32Array(result.vertices),
    indices: new Uint32Array(result.indices),
    normals: new Float32Array(result.normals),
    colors: new Float32Array(result.colors),
    uvs: new Float32Array(result.uvs),
    materialIds: new Uint8Array(result.material_ids),
  } as GeometryData

  // Log warning if geometry is empty
  if (geometry.vertices.length === 0 || geometry.indices.length === 0) {
    logger.warn('common', `[voxLoader] VOX file loaded but has no geometry: ${url}`, {
      vertices: geometry.vertices.length / 3,
      indices: geometry.indices.length,
      fileSize: bytes.length,
      size,
    })
  } else {
    logger.log('common', `[voxLoader] VOX file loaded successfully: ${url}`, {
      vertices: geometry.vertices.length / 3,
      triangles: geometry.indices.length / 3,
      size,
      depth,
    })
  }

  return { geometry, size, depth, octreeSize }
}

/**
 * Load a .vox file from a URL and generate Three.js geometry
 * @deprecated Use loadVoxBoxFromUrl() instead, which returns bounds information
 * @param url URL to the .vox file
 * @param _userNpub Optional user npub for color customization (not used - kept for API compatibility)
 * @param maxDepth Maximum octree depth (default: 3 for 8x8x8 avatars). Higher = supports larger models but increases scale.
 * @returns GeometryData with vertices, indices, normals, and colors
 */
export async function loadVoxFromUrl(url: string, _userNpub?: string, maxDepth: number = 3): Promise<GeometryData> {
  // Ensure WASM is initialized
  await cubeInit()

  // Fetch the .vox file
  const response = await fetch(url)
  if (!response.ok) {
    throw new Error(`Failed to fetch .vox file: ${response.statusText}`)
  }

  const arrayBuffer = await response.arrayBuffer()
  const bytes = new Uint8Array(arrayBuffer)

  // Load .vox file into WasmCube (centered alignment)
  // @ts-ignore - WASM module exports loadVox
  const wasmCube = cubeWasm.loadVox(bytes, 0.5, 0.5, 0.5)

  // Generate mesh from the cube (null palette = original colors)
  // maxDepth determines resolution: 2^maxDepth = max voxels per axis
  // Common values: 4 (16x16x16), 5 (32x32x32), 6 (64x64x64)
  const result = wasmCube.generateMesh(null, maxDepth) as MeshResult | { error: string }

  if ('error' in result) {
    logger.error('common', `[voxLoader] Failed to parse VOX file from ${url}: ${result.error}`)
    throw new Error(`Failed to parse VOX file: ${result.error}`)
  }

  const geometryData = {
    vertices: new Float32Array(result.vertices),
    indices: new Uint32Array(result.indices),
    normals: new Float32Array(result.normals),
    colors: new Float32Array(result.colors),
    uvs: new Float32Array(result.uvs),
    materialIds: new Uint8Array(result.material_ids),
  } as GeometryData

  // Log warning if geometry is empty
  if (geometryData.vertices.length === 0 || geometryData.indices.length === 0) {
    logger.warn('common', `[voxLoader] VOX file loaded but has no geometry: ${url}`, {
      vertices: geometryData.vertices.length / 3,
      indices: geometryData.indices.length,
      fileSize: bytes.length,
    })
  } else {
    logger.log('common', `[voxLoader] VOX file loaded successfully: ${url}`, {
      vertices: geometryData.vertices.length / 3,
      triangles: geometryData.indices.length / 3,
    })
  }

  return geometryData
}

/**
 * Load a .vox file from a File object with bounds information
 * @param file File object containing .vox data
 * @param _userNpub Optional user npub for color customization (not used - kept for API compatibility)
 * @returns VoxModel with geometry and bounds information
 */
export async function loadVoxBoxFromFile(file: File, _userNpub?: string): Promise<VoxModel> {
  // Ensure WASM is initialized
  await cubeInit()

  // Read file as ArrayBuffer
  const arrayBuffer = await file.arrayBuffer()
  const bytes = new Uint8Array(arrayBuffer)

  // Load .vox file into WasmCubeBox (preserves model bounds)
  // @ts-ignore - WASM module exports loadVoxBox
  const wasmCubeBox = cubeWasm.loadVoxBox(bytes)

  // Get model bounds
  const size = {
    x: wasmCubeBox.sizeX,
    y: wasmCubeBox.sizeY,
    z: wasmCubeBox.sizeZ,
  }
  const depth = wasmCubeBox.depth
  const octreeSize = wasmCubeBox.octreeSize

  // Generate mesh from the cube box (null palette = original colors)
  const result = wasmCubeBox.generateMesh(null) as MeshResult | { error: string }

  if ('error' in result) {
    logger.error('common', `[voxLoader] Failed to parse VOX file from ${file.name}: ${result.error}`)
    throw new Error(`Failed to parse VOX file: ${result.error}`)
  }

  const geometry = {
    vertices: new Float32Array(result.vertices),
    indices: new Uint32Array(result.indices),
    normals: new Float32Array(result.normals),
    colors: new Float32Array(result.colors),
    uvs: new Float32Array(result.uvs),
    materialIds: new Uint8Array(result.material_ids),
  } as GeometryData

  // Log warning if geometry is empty
  if (geometry.vertices.length === 0 || geometry.indices.length === 0) {
    logger.warn('common', `[voxLoader] VOX file loaded but has no geometry: ${file.name}`, {
      vertices: geometry.vertices.length / 3,
      indices: geometry.indices.length,
      fileSize: bytes.length,
      size,
    })
  } else {
    logger.log('common', `[voxLoader] VOX file loaded successfully: ${file.name}`, {
      vertices: geometry.vertices.length / 3,
      triangles: geometry.indices.length / 3,
      size,
      depth,
    })
  }

  return { geometry, size, depth, octreeSize }
}

/**
 * Load a .vox file from a File object (e.g., from file input)
 * @deprecated Use loadVoxBoxFromFile() instead, which returns bounds information
 * @param file File object containing .vox data
 * @param _userNpub Optional user npub for color customization (not used - kept for API compatibility)
 * @param maxDepth Maximum octree depth (default: 3 for 8x8x8 avatars). Higher = supports larger models but increases scale.
 * @returns GeometryData with vertices, indices, normals, and colors
 */
export async function loadVoxFromFile(file: File, _userNpub?: string, maxDepth: number = 3): Promise<GeometryData> {
  // Ensure WASM is initialized
  await cubeInit()

  // Read file as ArrayBuffer
  const arrayBuffer = await file.arrayBuffer()
  const bytes = new Uint8Array(arrayBuffer)

  // Load .vox file into WasmCube (centered alignment)
  // @ts-ignore - WASM module exports loadVox
  const wasmCube = cubeWasm.loadVox(bytes, 0.5, 0.5, 0.5)

  // Generate mesh from the cube (null palette = original colors)
  // maxDepth determines resolution: 2^maxDepth = max voxels per axis
  // Common values: 4 (16x16x16), 5 (32x32x32), 6 (64x64x64)
  const result = wasmCube.generateMesh(null, maxDepth) as MeshResult | { error: string }

  if ('error' in result) {
    logger.error('common', `[voxLoader] Failed to parse VOX file from ${file.name}: ${result.error}`)
    throw new Error(`Failed to parse VOX file: ${result.error}`)
  }

  const geometryData = {
    vertices: new Float32Array(result.vertices),
    indices: new Uint32Array(result.indices),
    normals: new Float32Array(result.normals),
    colors: new Float32Array(result.colors),
    uvs: new Float32Array(result.uvs),
    materialIds: new Uint8Array(result.material_ids),
  } as GeometryData

  // Log warning if geometry is empty
  if (geometryData.vertices.length === 0 || geometryData.indices.length === 0) {
    logger.warn('common', `[voxLoader] VOX file loaded but has no geometry: ${file.name}`, {
      vertices: geometryData.vertices.length / 3,
      indices: geometryData.indices.length,
      fileSize: bytes.length,
    })
  } else {
    logger.log('common', `[voxLoader] VOX file loaded successfully: ${file.name}`, {
      vertices: geometryData.vertices.length / 3,
      triangles: geometryData.indices.length / 3,
    })
  }

  return geometryData
}

/**
 * Load a .vox file from a Nostr profile tag
 * Looks for a tag with format: ["vox_avatar", "url"]
 * @param profileEvent Nostr kind 0 (profile) event
 * @param userNpub Optional user npub for color customization
 * @returns GeometryData or null if no vox_avatar tag found
 */
export async function loadVoxFromNostrProfile(
  profileEvent: any,
  userNpub?: string
): Promise<GeometryData | null> {
  if (!profileEvent?.tags) {
    return null
  }

  // Find vox_avatar tag
  const voxTag = profileEvent.tags.find(
    (tag: string[]) => tag[0] === 'vox_avatar' && tag[1]
  )

  if (!voxTag || !voxTag[1]) {
    return null
  }

  const voxUrl = voxTag[1]
  try {
    return await loadVoxFromUrl(voxUrl, userNpub)
  } catch (error) {
    logger.error('common', 'Failed to load .vox from Nostr profile:', error)
    return null
  }
}

/**
 * Example vox_avatar tag format for Nostr profiles:
 *
 * In your kind 0 (profile metadata) event, add a tag:
 * ["vox_avatar", "https://example.com/myavatar.vox"]
 *
 * This allows users to specify their custom MagicaVoxel avatar model.
 */
