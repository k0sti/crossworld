import * as logger from '../utils/logger';
/**
 * Shared depth configuration for the entire application.
 * This is the single source of truth for macro and micro depth values.
 */

/** Current macro depth - octree subdivision levels */
let currentMacroDepth = 4;

/** Current micro depth - rendering scale depth */
let currentMicroDepth = 0;

/** Current border depth - number of border cube layers */
let currentBorderDepth = 4;

/** Callbacks to notify when depth changes */
const depthChangeListeners: Array<(macroDepth: number, microDepth: number, borderDepth: number) => void> = [];

/**
 * Get current macro depth
 */
export function getMacroDepth(): number {
  return currentMacroDepth;
}

/**
 * Get current micro depth
 */
export function getMicroDepth(): number {
  return currentMicroDepth;
}

/**
 * Get current border depth
 */
export function getBorderDepth(): number {
  return currentBorderDepth;
}

/**
 * Get total depth (macro + micro)
 */
export function getTotalDepth(): number {
  return currentMacroDepth + currentMicroDepth;
}

/**
 * Get the base depth (macro + border)
 * This is the depth where voxels are 1 unit in size
 */
export function getBaseDepth(): number {
  return currentMacroDepth + currentBorderDepth;
}

/**
 * Convert cursor depth (relative to base) to absolute octree depth
 *
 * Cursor depth interpretation:
 * - cursorDepth = 0: unit cubes at base depth (macro + border)
 * - cursorDepth < 0: larger voxels (e.g., -1 = 2x2x2 unit cubes)
 * - cursorDepth > 0: smaller voxels (subdivisions, up to micro_depth)
 *
 * @param cursorDepth Relative cursor depth
 * @returns Absolute octree depth
 */
export function cursorDepthToAbsolute(cursorDepth: number): number {
  return getBaseDepth() + cursorDepth;
}

/**
 * Convert absolute octree depth to cursor depth (relative to base)
 *
 * @param absoluteDepth Absolute octree depth
 * @returns Relative cursor depth
 */
export function absoluteDepthToCursor(absoluteDepth: number): number {
  return absoluteDepth - getBaseDepth();
}

/**
 * Get minimum cursor depth (largest voxels)
 * This is -(macro + border) to allow scaling down to depth 0 (entire world)
 */
export function getMinCursorDepth(): number {
  return -(currentMacroDepth + currentBorderDepth);
}

/**
 * Get maximum cursor depth (smallest voxels)
 * This is micro_depth (subdivisions beyond base depth)
 */
export function getMaxCursorDepth(): number {
  return currentMicroDepth;
}

/**
 * Set macro depth and notify listeners
 */
export function setMacroDepth(depth: number): void {
  if (depth < 1 || depth > 10) {
    logger.warn('common', `Invalid macro depth ${depth}, must be between 1 and 10`);
    return;
  }
  currentMacroDepth = depth;
  notifyListeners();
}

/**
 * Set micro depth and notify listeners
 */
export function setMicroDepth(depth: number): void {
  if (depth < 0 || depth > 3) {
    logger.warn('common', `Invalid micro depth ${depth}, must be between 0 and 3`);
    return;
  }
  currentMicroDepth = depth;
  notifyListeners();
}

/**
 * Set border depth and notify listeners
 */
export function setBorderDepth(depth: number): void {
  if (depth < 0 || depth > 5) {
    logger.warn('common', `Invalid border depth ${depth}, must be between 0 and 5`);
    return;
  }
  currentBorderDepth = depth;
  notifyListeners();
}

/**
 * Subscribe to depth changes
 */
export function onDepthChange(callback: (macroDepth: number, microDepth: number, borderDepth: number) => void): () => void {
  depthChangeListeners.push(callback);
  // Return unsubscribe function
  return () => {
    const index = depthChangeListeners.indexOf(callback);
    if (index > -1) {
      depthChangeListeners.splice(index, 1);
    }
  };
}

/**
 * Notify all listeners of depth change
 */
function notifyListeners(): void {
  depthChangeListeners.forEach(callback => {
    callback(currentMacroDepth, currentMicroDepth, currentBorderDepth);
  });
}
