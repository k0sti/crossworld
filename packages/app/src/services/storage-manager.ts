/**
 * Storage Manager Service
 *
 * THIS IS THE SINGLE SOURCE OF TRUTH FOR ALL PERSISTENT DATA STORAGE.
 * Manages both localStorage and sessionStorage in a centralized way.
 *
 * All storage keys are defined here to prevent duplication and ensure consistency.
 */

import * as logger from '../utils/logger';
import { LoginSettingsService } from './login-settings';
import { clearAvatarSession } from './avatar-session-storage';
import { clearPersistentAvatarState } from './avatar-state-storage';
import { profileCache } from './profile-cache';

// ===== Storage Keys =====

export const STORAGE_KEYS = {
  // Login & Authentication
  LOGIN_SETTINGS: 'crossworld_login_settings',
  GUEST_ACCOUNT: 'crossworld_guest_account',

  // Network & Relays
  RELAYS: 'crossworld_relays',

  // Camera & World State
  CAMERA_STATE: 'cameraControllerState',
  WORLD_GRID_VISIBLE: 'worldPanel.worldGridVisible',
  WIREFRAME_ENABLED: 'worldPanel.wireframeEnabled',
  TEXTURES_ENABLED: 'worldPanel.texturesEnabled',
  AVATAR_TEXTURES_ENABLED: 'worldPanel.avatarTexturesEnabled',

  // Configuration
  LOG_CONFIG: 'crossworld:log-config',

  // Avatar
  AVATAR_SELECTION: 'avatarSelection',
  AVATAR_PERSISTENT_STATE: 'crossworld.avatar.persistent-state',
} as const;

export const SESSION_STORAGE_KEYS = {
  AVATAR_SESSION: 'crossworld.avatar.session',
} as const;

// ===== Storage Operations =====

/**
 * Clear all persistent data (complete reset)
 * Use this for "Reset All Data" functionality
 */
export function clearAllData(): void {
  try {
    logger.log('service', '[StorageManager] Clearing all persistent data...');

    // Clear login data
    LoginSettingsService.clear();
    LoginSettingsService.clearGuestAccount();

    // Clear all localStorage keys
    Object.values(STORAGE_KEYS).forEach(key => {
      localStorage.removeItem(key);
    });

    // Clear all sessionStorage keys
    Object.values(SESSION_STORAGE_KEYS).forEach(key => {
      sessionStorage.removeItem(key);
    });

    // Clear avatar persistent state
    clearPersistentAvatarState();

    // Clear in-memory caches
    profileCache.clearCache();

    logger.log('service', '[StorageManager] All data cleared successfully');
  } catch (error) {
    logger.error('service', '[StorageManager] Failed to clear all data:', error);
    throw error;
  }
}

/**
 * Clear all data except login information
 * Use this for "Restart" functionality in profile panel
 */
export function clearAllDataExceptLogin(): void {
  try {
    logger.log('service', '[StorageManager] Clearing all data except login...');

    // Clear relays
    localStorage.removeItem(STORAGE_KEYS.RELAYS);

    // Clear camera & world state
    localStorage.removeItem(STORAGE_KEYS.CAMERA_STATE);
    localStorage.removeItem(STORAGE_KEYS.WORLD_GRID_VISIBLE);
    localStorage.removeItem(STORAGE_KEYS.WIREFRAME_ENABLED);
    localStorage.removeItem(STORAGE_KEYS.TEXTURES_ENABLED);
    localStorage.removeItem(STORAGE_KEYS.AVATAR_TEXTURES_ENABLED);

    // Clear log config
    localStorage.removeItem(STORAGE_KEYS.LOG_CONFIG);

    // Clear avatar selection and persistent state
    localStorage.removeItem(STORAGE_KEYS.AVATAR_SELECTION);
    clearPersistentAvatarState();

    // Clear session storage (avatar config)
    clearAvatarSession();

    // Clear in-memory caches
    profileCache.clearCache();

    logger.log('service', '[StorageManager] All data cleared except login');
  } catch (error) {
    logger.error('service', '[StorageManager] Failed to clear data:', error);
    throw error;
  }
}

/**
 * Get a value from localStorage
 */
export function getItem<T = string>(key: string): T | null {
  try {
    const value = localStorage.getItem(key);
    if (value === null) return null;

    // Try to parse as JSON, fall back to string
    try {
      return JSON.parse(value) as T;
    } catch {
      return value as unknown as T;
    }
  } catch (error) {
    logger.error('service', `[StorageManager] Failed to get item '${key}':`, error);
    return null;
  }
}

/**
 * Set a value in localStorage
 */
export function setItem<T>(key: string, value: T): void {
  try {
    const stringValue = typeof value === 'string' ? value : JSON.stringify(value);
    localStorage.setItem(key, stringValue);
  } catch (error) {
    logger.error('service', `[StorageManager] Failed to set item '${key}':`, error);
    throw error;
  }
}

/**
 * Remove a value from localStorage
 */
export function removeItem(key: string): void {
  try {
    localStorage.removeItem(key);
  } catch (error) {
    logger.error('service', `[StorageManager] Failed to remove item '${key}':`, error);
    throw error;
  }
}

/**
 * Check if user has login data saved
 */
export function hasLoginData(): boolean {
  return LoginSettingsService.exists();
}

/**
 * Check if avatar data exists in session
 */
export function hasAvatarInSession(): boolean {
  const stored = sessionStorage.getItem(SESSION_STORAGE_KEYS.AVATAR_SESSION);
  return stored !== null;
}

/**
 * Get storage statistics (useful for debugging)
 */
export function getStorageStats(): {
  localStorageKeys: number;
  sessionStorageKeys: number;
  localStorageSize: number;
  sessionStorageSize: number;
} {
  let localStorageSize = 0;
  let sessionStorageSize = 0;

  for (let i = 0; i < localStorage.length; i++) {
    const key = localStorage.key(i);
    if (key) {
      const value = localStorage.getItem(key);
      if (value) {
        localStorageSize += key.length + value.length;
      }
    }
  }

  for (let i = 0; i < sessionStorage.length; i++) {
    const key = sessionStorage.key(i);
    if (key) {
      const value = sessionStorage.getItem(key);
      if (value) {
        sessionStorageSize += key.length + value.length;
      }
    }
  }

  return {
    localStorageKeys: localStorage.length,
    sessionStorageKeys: sessionStorage.length,
    localStorageSize,
    sessionStorageSize,
  };
}
