/**
 * Shared depth configuration for the entire application.
 * This is the single source of truth for macro and micro depth values.
 */

/** Current macro depth - octree subdivision levels */
let currentMacroDepth = 3;

/** Current micro depth - rendering scale depth (fixed at 0) */
const currentMicroDepth = 0;

/** Callbacks to notify when depth changes */
const depthChangeListeners: Array<(macroDepth: number, microDepth: number) => void> = [];

/**
 * Get current macro depth
 */
export function getMacroDepth(): number {
  return currentMacroDepth;
}

/**
 * Get current micro depth (always 0)
 */
export function getMicroDepth(): number {
  return currentMicroDepth;
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
    console.warn(`Invalid macro depth ${depth}, must be between 1 and 10`);
    return;
  }
  currentMacroDepth = depth;
  notifyListeners();
}

/**
 * Subscribe to depth changes
 */
export function onDepthChange(callback: (macroDepth: number, microDepth: number) => void): () => void {
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
    callback(currentMacroDepth, currentMicroDepth);
  });
}
