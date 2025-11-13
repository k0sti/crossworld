/**
 * World Panel Settings - Single Source of Truth
 *
 * This module provides centralized management of world panel settings including:
 * - Default values
 * - localStorage persistence keys
 * - Getter/setter functions
 */

export interface WorldPanelSettings {
  worldGridVisible: boolean;
  wireframeEnabled: boolean;
  texturesEnabled: boolean;
  avatarTexturesEnabled: boolean;
}

// Default values - single source of truth
export const DEFAULT_WORLD_PANEL_SETTINGS: WorldPanelSettings = {
  worldGridVisible: false,
  wireframeEnabled: false,
  texturesEnabled: false,
  avatarTexturesEnabled: false,
};

// localStorage keys
const STORAGE_KEYS = {
  worldGridVisible: 'worldPanel.worldGridVisible',
  wireframeEnabled: 'worldPanel.wireframeEnabled',
  texturesEnabled: 'worldPanel.texturesEnabled',
  avatarTexturesEnabled: 'worldPanel.avatarTexturesEnabled',
} as const;

/**
 * Get a setting from localStorage or return the default value
 */
export function getWorldPanelSetting<K extends keyof WorldPanelSettings>(
  key: K
): WorldPanelSettings[K] {
  const storageKey = STORAGE_KEYS[key];
  const saved = localStorage.getItem(storageKey);

  if (saved !== null) {
    try {
      return JSON.parse(saved);
    } catch (e) {
      console.error(`Failed to parse setting ${key} from localStorage:`, e);
    }
  }

  return DEFAULT_WORLD_PANEL_SETTINGS[key];
}

/**
 * Set a setting in localStorage
 */
export function setWorldPanelSetting<K extends keyof WorldPanelSettings>(
  key: K,
  value: WorldPanelSettings[K]
): void {
  const storageKey = STORAGE_KEYS[key];
  localStorage.setItem(storageKey, JSON.stringify(value));
}

/**
 * Get all settings from localStorage or defaults
 */
export function getAllWorldPanelSettings(): WorldPanelSettings {
  return {
    worldGridVisible: getWorldPanelSetting('worldGridVisible'),
    wireframeEnabled: getWorldPanelSetting('wireframeEnabled'),
    texturesEnabled: getWorldPanelSetting('texturesEnabled'),
    avatarTexturesEnabled: getWorldPanelSetting('avatarTexturesEnabled'),
  };
}
