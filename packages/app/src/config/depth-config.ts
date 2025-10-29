import * as logger from '../utils/logger';
/**
 * Shared depth configuration for the entire application.
 * This is the single source of truth for macro and micro depth values.
 */

/** Current macro depth - octree subdivision levels */
let currentMacroDepth = 3;

/** Current micro depth - rendering scale depth */
let currentMicroDepth = 0;

/** Current border depth - number of border cube layers */
let currentBorderDepth = 0;

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
