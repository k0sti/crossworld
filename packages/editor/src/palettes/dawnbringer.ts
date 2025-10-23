/**
 * DawnBringer palettes
 * Loaded from PNG palette images
 */

/**
 * Extract colors from a PNG image
 * @param imageUrl Path to the palette image
 * @returns Promise resolving to array of hex color strings
 */
export async function extractPaletteFromImage(imageUrl: string): Promise<string[]> {
  return new Promise((resolve, reject) => {
    const img = new Image()
    img.crossOrigin = 'anonymous'

    img.onload = () => {
      const canvas = document.createElement('canvas')
      const ctx = canvas.getContext('2d')

      if (!ctx) {
        reject(new Error('Failed to get canvas context'))
        return
      }

      canvas.width = img.width
      canvas.height = img.height
      ctx.drawImage(img, 0, 0)

      const imageData = ctx.getImageData(0, 0, img.width, img.height)
      const colors: string[] = []
      const seen = new Set<string>()

      // Extract unique colors from the image
      for (let i = 0; i < imageData.data.length; i += 4) {
        const r = imageData.data[i]
        const g = imageData.data[i + 1]
        const b = imageData.data[i + 2]
        const a = imageData.data[i + 3]

        // Skip transparent pixels
        if (a < 128) continue

        const hex = rgbToHex(r, g, b)
        if (!seen.has(hex)) {
          seen.add(hex)
          colors.push(hex)
        }
      }

      resolve(colors)
    }

    img.onerror = () => {
      reject(new Error(`Failed to load image: ${imageUrl}`))
    }

    img.src = imageUrl
  })
}

/**
 * Convert RGB to hex
 */
function rgbToHex(r: number, g: number, b: number): string {
  return '#' + [r, g, b].map(x => {
    const hex = x.toString(16)
    return hex.length === 1 ? '0' + hex : hex
  }).join('')
}

// Fallback palettes (in case image loading fails)
export const DAWNBRINGER_16 = [
  '#140c1c', '#442434', '#30346d', '#4e4a4e',
  '#854c30', '#346524', '#d04648', '#757161',
  '#597dce', '#d27d2c', '#8595a1', '#6daa2c',
  '#d2aa99', '#6dc2ca', '#dad45e', '#deeed6',
]

export const DAWNBRINGER_32 = [
  '#000000', '#222034', '#45283c', '#663931',
  '#8f563b', '#df7126', '#d9a066', '#eec39a',
  '#fbf236', '#99e550', '#6abe30', '#37946e',
  '#4b692f', '#524b24', '#323c39', '#3f3f74',
  '#306082', '#5b6ee1', '#639bff', '#5fcde4',
  '#cbdbfc', '#ffffff', '#9badb7', '#847e87',
  '#696a6a', '#595652', '#76428a', '#ac3232',
  '#d95763', '#d77bba', '#8f974a', '#8a6f30',
]

/**
 * Get palette by size
 * If you want to load from PNG, use extractPaletteFromImage() instead
 */
export function getDawnbringerPalette(size: 16 | 32): string[] {
  return size === 16 ? DAWNBRINGER_16 : DAWNBRINGER_32
}

/**
 * Get available palette sizes
 */
export function getDawnbringerSizes(): number[] {
  return [16, 32]
}
