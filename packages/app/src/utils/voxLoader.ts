import init, { load_vox_from_bytes, type GeometryData } from '@workspace/wasm'

/**
 * Load a .vox file from a URL and generate Three.js geometry
 * @param url URL to the .vox file
 * @param userNpub Optional user npub for color customization
 * @returns GeometryData with vertices, indices, normals, and colors
 */
export async function loadVoxFromUrl(url: string, userNpub?: string): Promise<GeometryData> {
  // Ensure WASM is initialized
  await init()

  // Fetch the .vox file
  const response = await fetch(url)
  if (!response.ok) {
    throw new Error(`Failed to fetch .vox file: ${response.statusText}`)
  }

  const arrayBuffer = await response.arrayBuffer()
  const bytes = new Uint8Array(arrayBuffer)

  // Load and parse the .vox file
  const geometryData = load_vox_from_bytes(bytes, userNpub)
  return geometryData
}

/**
 * Load a .vox file from a File object (e.g., from file input)
 * @param file File object containing .vox data
 * @param userNpub Optional user npub for color customization
 * @returns GeometryData with vertices, indices, normals, and colors
 */
export async function loadVoxFromFile(file: File, userNpub?: string): Promise<GeometryData> {
  // Ensure WASM is initialized
  await init()

  // Read file as ArrayBuffer
  const arrayBuffer = await file.arrayBuffer()
  const bytes = new Uint8Array(arrayBuffer)

  // Load and parse the .vox file
  const geometryData = load_vox_from_bytes(bytes, userNpub)
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
    console.error('Failed to load .vox from Nostr profile:', error)
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
