/**
 * HSV palette generator
 * Generates colors based on Hue-Saturation-Value color model
 */

export interface HSVColor {
  h: number // Hue (0-360)
  s: number // Saturation (0-100)
  v: number // Value (0-100)
}

export interface RGBColor {
  r: number // Red (0-255)
  g: number // Green (0-255)
  b: number // Blue (0-255)
}

/**
 * Convert HSV to RGB
 */
export function hsvToRgb(h: number, s: number, v: number): RGBColor {
  const s_norm = s / 100
  const v_norm = v / 100

  const c = v_norm * s_norm
  const x = c * (1 - Math.abs(((h / 60) % 2) - 1))
  const m = v_norm - c

  let r = 0, g = 0, b = 0

  if (h >= 0 && h < 60) {
    r = c; g = x; b = 0
  } else if (h >= 60 && h < 120) {
    r = x; g = c; b = 0
  } else if (h >= 120 && h < 180) {
    r = 0; g = c; b = x
  } else if (h >= 180 && h < 240) {
    r = 0; g = x; b = c
  } else if (h >= 240 && h < 300) {
    r = x; g = 0; b = c
  } else {
    r = c; g = 0; b = x
  }

  return {
    r: Math.round((r + m) * 255),
    g: Math.round((g + m) * 255),
    b: Math.round((b + m) * 255)
  }
}

/**
 * Convert RGB to hex
 */
export function rgbToHex(r: number, g: number, b: number): string {
  return '#' + [r, g, b].map(x => {
    const hex = x.toString(16)
    return hex.length === 1 ? '0' + hex : hex
  }).join('')
}

/**
 * Generate HSV palette with specified size
 */
export function generateHSVPalette(size: number): string[] {
  const colors: string[] = []

  // Always include black and white
  colors.push('#000000') // Black
  colors.push('#FFFFFF') // White

  const remainingSize = size - 2
  const hueStep = 360 / Math.ceil(remainingSize / 2)

  // Generate bright and dark variants
  for (let i = 0; i < remainingSize; i++) {
    const hue = (i * hueStep) % 360
    const saturation = 100
    const value = i % 2 === 0 ? 100 : 50 // Alternate between bright and dark

    const rgb = hsvToRgb(hue, saturation, value)
    colors.push(rgbToHex(rgb.r, rgb.g, rgb.b))
  }

  return colors.slice(0, size)
}
