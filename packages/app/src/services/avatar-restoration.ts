/**
 * Avatar Restoration Service
 *
 * Handles the waterfall approach for avatar restoration:
 * 1. Try to fetch from Nostr (with timeout)
 * 2. Fall back to localStorage persistent state (includes position)
 * 3. Fall back to browser session storage (config only)
 * 4. Show avatar selector as last resort
 */

import type { AvatarConfig, AvatarState, Position } from './avatar-state';
import { loadAvatarFromSession, saveAvatarToSession } from './avatar-session-storage';
import { loadPersistentAvatarState, savePersistentFromNostrState } from './avatar-state-storage';
import * as logger from '../utils/logger';

export type RestoreStatus =
  | 'fetching-nostr'
  | 'using-persistent'
  | 'using-session'
  | 'need-selection'
  | 'restored'
  | 'error';

export interface RestoreResult {
  status: RestoreStatus;
  config?: AvatarConfig;
  position?: Position;
  state?: Partial<AvatarState>;
  source?: 'nostr' | 'persistent' | 'session' | 'none';
  message: string;
}

interface RestoreOptions {
  /** Timeout for Nostr query in milliseconds (default: 5000) */
  nostrTimeout?: number;

  /** Callback for status updates */
  onStatusChange?: (status: RestoreStatus, message: string) => void;
}

/**
 * Restore avatar config with waterfall approach:
 * 1. Try Nostr (with timeout)
 * 2. Try localStorage persistent state (includes position)
 * 3. Try session storage (config only)
 * 4. Return null (triggers avatar selector)
 */
export async function restoreAvatarConfig(
  pubkey: string,
  queryLastState: (pubkey: string) => Promise<Partial<AvatarState> | null>,
  options: RestoreOptions = {}
): Promise<RestoreResult> {
  const {
    nostrTimeout = 5000,
    onStatusChange
  } = options;

  logger.log('ui', '[AvatarRestore] Starting restoration for pubkey:', pubkey);

  // Step 1: Try to fetch from Nostr with timeout
  try {
    onStatusChange?.('fetching-nostr', 'Fetching avatar from network...');
    logger.log('ui', '[AvatarRestore] Querying Nostr...');

    const state = await withTimeout(
      queryLastState(pubkey),
      nostrTimeout,
      'Nostr query timeout'
    );

    if (state) {
      logger.log('ui', '[AvatarRestore] Successfully restored from Nostr');

      const config: AvatarConfig = {
        avatarType: state.avatarType || 'vox',
        avatarId: state.avatarId,
        avatarUrl: state.avatarUrl,
        avatarData: state.avatarData,
        avatarMod: state.avatarMod,
        avatarTexture: state.avatarTexture,
      };

      const position: Position = state.position || { x: 4, y: 0, z: 4 };

      // Save to both persistent storage and session for future fallback
      savePersistentFromNostrState(state);
      saveAvatarToSession(pubkey, config);

      onStatusChange?.('restored', 'Avatar restored from network');

      return {
        status: 'restored',
        config,
        position,
        state,
        source: 'nostr',
        message: 'Avatar restored from network',
      };
    }

    logger.log('ui', '[AvatarRestore] No Nostr state found, trying persistent storage...');
  } catch (error) {
    logger.warn('ui', '[AvatarRestore] Nostr query failed or timed out:', error);
    // Continue to persistent storage fallback
  }

  // Step 2: Try persistent storage (localStorage with position)
  onStatusChange?.('using-persistent', 'Loading avatar from storage...');
  logger.log('ui', '[AvatarRestore] Checking persistent storage...');

  const persistentState = loadPersistentAvatarState(pubkey);
  if (persistentState) {
    logger.log('ui', '[AvatarRestore] Successfully restored from persistent storage');

    onStatusChange?.('restored', 'Avatar loaded from storage');

    return {
      status: 'restored',
      config: persistentState.avatarConfig,
      position: persistentState.position,
      source: 'persistent',
      message: 'Avatar loaded from persistent storage',
    };
  }

  // Step 3: Try session storage (config only, no position)
  onStatusChange?.('using-session', 'Loading avatar from session...');
  logger.log('ui', '[AvatarRestore] Checking session storage...');

  const sessionConfig = loadAvatarFromSession(pubkey);
  if (sessionConfig) {
    logger.log('ui', '[AvatarRestore] Successfully restored from session storage');

    onStatusChange?.('restored', 'Avatar loaded from session');

    return {
      status: 'restored',
      config: sessionConfig,
      source: 'session',
      message: 'Avatar loaded from previous session',
    };
  }

  // Step 4: No avatar found - need selection
  logger.log('ui', '[AvatarRestore] No avatar found, need selection');
  onStatusChange?.('need-selection', 'Please select an avatar');

  return {
    status: 'need-selection',
    source: 'none',
    message: 'No saved avatar found',
  };
}

/**
 * Helper: Execute promise with timeout
 */
function withTimeout<T>(
  promise: Promise<T>,
  timeoutMs: number,
  timeoutMessage: string
): Promise<T> {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => {
      reject(new Error(timeoutMessage));
    }, timeoutMs);

    promise
      .then((value) => {
        clearTimeout(timer);
        resolve(value);
      })
      .catch((error) => {
        clearTimeout(timer);
        reject(error);
      });
  });
}

/**
 * Get default avatar config when no restoration is possible
 */
export function getDefaultAvatarConfig(): AvatarConfig {
  return {
    avatarType: 'vox',
    avatarId: 'chr_base',
    avatarTexture: 'grass',
  };
}
