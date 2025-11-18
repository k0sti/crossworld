import * as logger from '../utils/logger';
/**
 * Shared depth configuration for the entire application.
 * This is the single source of truth for macro and micro depth values.
 */

// localStorage keys
const STORAGE_KEYS = {
  macroDepth: 'worldConfig.macroDepth',
  microDepth: 'worldConfig.microDepth',
  borderDepth: 'worldConfig.borderDepth',
  seed: 'worldConfig.seed',
} as const;

// Default values
const DEFAULTS = {
  macroDepth: 4,
  microDepth: 0,
  borderDepth: 4,
  seed: 0,
} as const;

/**
 * Load a number from localStorage with validation
 */
function loadFromStorage(key: string, defaultValue: number, min: number, max: number): number {
  try {
    const saved = localStorage.getItem(key);
    if (saved !== null) {
      const value = parseInt(saved);
      if (!isNaN(value) && value >= min && value <= max) {
        return value;
      }
    }
  } catch (error) {
    logger.warn('common', `Failed to load ${key} from localStorage:`, error);
  }
  return defaultValue;
}

/**
 * Save a number to localStorage
 */
function saveToStorage(key: string, value: number): void {
  try {
    localStorage.setItem(key, value.toString());
  } catch (error) {
    logger.error('common', `Failed to save ${key} to localStorage:`, error);
  }
}

/** Current macro depth - octree subdivision levels */
let currentMacroDepth = loadFromStorage(STORAGE_KEYS.macroDepth, DEFAULTS.macroDepth, 1, 10);

/** Current micro depth - rendering scale depth */
let currentMicroDepth = loadFromStorage(STORAGE_KEYS.microDepth, DEFAULTS.microDepth, 0, 3);

/** Current border depth - number of border cube layers */
let currentBorderDepth = loadFromStorage(STORAGE_KEYS.borderDepth, DEFAULTS.borderDepth, 0, 5);

/** Current world generation seed - for deterministic world generation */
let currentSeed = loadFromStorage(STORAGE_KEYS.seed, DEFAULTS.seed, 0, 4294967295);

// Log loaded configuration
logger.log('common', `[depth-config] Loaded from localStorage: macro=${currentMacroDepth}, micro=${currentMicroDepth}, border=${currentBorderDepth}, seed=${currentSeed}`);

/** Callbacks to notify when depth changes */
const depthChangeListeners: Array<(macroDepth: number, microDepth: number, borderDepth: number) => void> = [];

/** Callbacks to notify when seed changes */
const seedChangeListeners: Array<(seed: number) => void> = [];

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
  saveToStorage(STORAGE_KEYS.macroDepth, depth);
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
  saveToStorage(STORAGE_KEYS.microDepth, depth);
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
  saveToStorage(STORAGE_KEYS.borderDepth, depth);
  notifyListeners();
}

/**
 * Get current seed
 */
export function getSeed(): number {
  return currentSeed;
}

/**
 * Set seed and notify listeners
 */
export function setSeed(seed: number): void {
  if (seed < 0 || seed > 4294967295) {
    logger.warn('common', `Invalid seed ${seed}, must be between 0 and 4294967295`);
    return;
  }
  currentSeed = seed;
  saveToStorage(STORAGE_KEYS.seed, seed);
  notifySeedListeners();
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
 * Subscribe to seed changes
 */
export function onSeedChange(callback: (seed: number) => void): () => void {
  seedChangeListeners.push(callback);
  // Return unsubscribe function
  return () => {
    const index = seedChangeListeners.indexOf(callback);
    if (index > -1) {
      seedChangeListeners.splice(index, 1);
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

/**
 * Notify all listeners of seed change
 */
function notifySeedListeners(): void {
  seedChangeListeners.forEach(callback => {
    callback(currentSeed);
  });
}
