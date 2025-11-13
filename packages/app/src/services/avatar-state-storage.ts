/**
 * Avatar State Storage Service
 *
 * Manages persistent storage of avatar STATE_EVENT data (kind 30317).
 * This stores the complete avatar configuration AND position in localStorage
 * as a fallback when Nostr relay queries fail or timeout.
 *
 * Unlike avatar-session-storage.ts which only stores avatar config in sessionStorage,
 * this service stores the full state including position in localStorage for persistence
 * across browser sessions.
 */

import type { AvatarConfig, AvatarState, Position } from './avatar-state';
import * as logger from '../utils/logger';

const STORAGE_KEY = 'crossworld.avatar.persistent-state';

/**
 * Persistent avatar state data structure
 * This mirrors the STATE_EVENT (kind 30317) structure
 */
export interface PersistentAvatarState {
  pubkey: string;

  // Avatar configuration
  avatarConfig: AvatarConfig;

  // Position and state
  position: Position;
  status: 'active' | 'idle' | 'away';

  // Optional metadata
  customMessage?: string;

  // Timestamp when this was saved
  savedAt: number;

  // Original event timestamp (if from Nostr)
  eventTimestamp?: number;
}

/**
 * Save avatar state to localStorage
 * This should be called whenever a STATE_EVENT is published
 */
export function savePersistentAvatarState(
  pubkey: string,
  avatarConfig: AvatarConfig,
  position: Position,
  status: 'active' | 'idle' | 'away' = 'active',
  customMessage?: string,
  eventTimestamp?: number
): void {
  try {
    const data: PersistentAvatarState = {
      pubkey,
      avatarConfig,
      position,
      status,
      customMessage,
      savedAt: Date.now(),
      eventTimestamp,
    };

    localStorage.setItem(STORAGE_KEY, JSON.stringify(data));
    logger.log('service', '[AvatarStateStorage] Saved persistent avatar state:', {
      pubkey: pubkey.slice(0, 8),
      avatarType: avatarConfig.avatarType,
      position,
    });
  } catch (error) {
    logger.error('service', '[AvatarStateStorage] Failed to save persistent state:', error);
  }
}

/**
 * Load avatar state from localStorage
 * Returns null if not found or if for different pubkey
 */
export function loadPersistentAvatarState(pubkey: string): PersistentAvatarState | null {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (!stored) {
      logger.log('service', '[AvatarStateStorage] No persistent state found');
      return null;
    }

    const data = JSON.parse(stored) as PersistentAvatarState;

    // Check if stored state is for current user
    if (data.pubkey !== pubkey) {
      logger.log('service', '[AvatarStateStorage] Persistent state is for different user, ignoring');
      return null;
    }

    // Check if data is too old (older than 7 days)
    const MAX_AGE_MS = 7 * 24 * 60 * 60 * 1000; // 7 days
    const age = Date.now() - data.savedAt;
    if (age > MAX_AGE_MS) {
      logger.log('service', '[AvatarStateStorage] Persistent state too old, ignoring');
      clearPersistentAvatarState();
      return null;
    }

    logger.log('service', '[AvatarStateStorage] Loaded persistent avatar state:', {
      pubkey: pubkey.slice(0, 8),
      avatarType: data.avatarConfig.avatarType,
      position: data.position,
      age: Math.round(age / 1000 / 60) + ' minutes',
    });

    return data;
  } catch (error) {
    logger.error('service', '[AvatarStateStorage] Failed to load persistent state:', error);
    return null;
  }
}

/**
 * Save state from Nostr event
 * This extracts data from a parsed AvatarState and saves it
 */
export function savePersistentFromNostrState(state: Partial<AvatarState>): void {
  if (!state.pubkey) {
    logger.warn('service', '[AvatarStateStorage] Cannot save state without pubkey');
    return;
  }

  const avatarConfig: AvatarConfig = {
    avatarType: state.avatarType || 'vox',
    avatarId: state.avatarId,
    avatarUrl: state.avatarUrl,
    avatarData: state.avatarData,
    avatarMod: state.avatarMod,
    avatarTexture: state.avatarTexture,
  };

  const position: Position = state.position || { x: 4, y: 0, z: 4 };

  savePersistentAvatarState(
    state.pubkey,
    avatarConfig,
    position,
    state.status || 'active',
    state.customMessage,
    state.stateEventTimestamp
  );
}

/**
 * Clear avatar state from localStorage
 */
export function clearPersistentAvatarState(): void {
  try {
    localStorage.removeItem(STORAGE_KEY);
    logger.log('service', '[AvatarStateStorage] Cleared persistent avatar state');
  } catch (error) {
    logger.error('service', '[AvatarStateStorage] Failed to clear persistent state:', error);
  }
}

/**
 * Check if persistent state exists
 */
export function hasPersistentAvatarState(): boolean {
  return localStorage.getItem(STORAGE_KEY) !== null;
}

/**
 * Get the age of stored state in milliseconds
 */
export function getPersistentStateAge(pubkey: string): number | null {
  const state = loadPersistentAvatarState(pubkey);
  if (!state) return null;
  return Date.now() - state.savedAt;
}
