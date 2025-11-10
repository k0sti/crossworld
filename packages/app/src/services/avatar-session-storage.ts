/**
 * Avatar Session Storage Service
 *
 * Manages browser session storage for avatar configuration.
 * This provides a fallback when Nostr state restoration fails or times out.
 */

import type { AvatarConfig } from './avatar-state';
import * as logger from '../utils/logger';

const STORAGE_KEY = 'crossworld.avatar.session';

/**
 * Save avatar config to session storage
 */
export function saveAvatarToSession(pubkey: string, config: AvatarConfig): void {
  try {
    const data = {
      pubkey,
      config,
      timestamp: Date.now(),
    };
    sessionStorage.setItem(STORAGE_KEY, JSON.stringify(data));
    logger.log('ui', '[AvatarSession] Saved avatar config to session storage');
  } catch (error) {
    logger.error('ui', '[AvatarSession] Failed to save to session storage:', error);
  }
}

/**
 * Load avatar config from session storage
 * Returns null if not found or if for different pubkey
 */
export function loadAvatarFromSession(pubkey: string): AvatarConfig | null {
  try {
    const stored = sessionStorage.getItem(STORAGE_KEY);
    if (!stored) {
      return null;
    }

    const data = JSON.parse(stored);

    // Check if stored avatar is for current user
    if (data.pubkey !== pubkey) {
      logger.log('ui', '[AvatarSession] Session avatar is for different user, ignoring');
      return null;
    }

    logger.log('ui', '[AvatarSession] Loaded avatar config from session storage');
    return data.config;
  } catch (error) {
    logger.error('ui', '[AvatarSession] Failed to load from session storage:', error);
    return null;
  }
}

/**
 * Clear avatar config from session storage
 */
export function clearAvatarSession(): void {
  try {
    sessionStorage.removeItem(STORAGE_KEY);
    logger.log('ui', '[AvatarSession] Cleared session storage');
  } catch (error) {
    logger.error('ui', '[AvatarSession] Failed to clear session storage:', error);
  }
}
