/**
 * Relay Settings Service
 *
 * THIS IS THE SINGLE SOURCE OF TRUTH FOR ALL RELAY CONFIGURATION.
 * All code must use functions from this service instead of importing DEFAULT_RELAYS directly.
 *
 * Manages relay configuration from localStorage and provides
 * functions to get enabled relays for different purposes.
 */

import { DEFAULT_RELAYS, DEFAULT_RELAY_STATES } from '../config';

interface RelayConfig {
  url: string;
  enabledForProfile: boolean;
  enabledForWorld: boolean;
}

const STORAGE_KEY = 'crossworld_relays';

/**
 * Load relay settings from localStorage
 */
function loadRelaySettings(): RelayConfig[] {
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved) {
      return JSON.parse(saved) as RelayConfig[];
    }
  } catch (error) {
    console.error('[RelaySettings] Failed to load from localStorage:', error);
  }

  // Return defaults if nothing saved
  return DEFAULT_RELAYS.map(url => {
    const defaults = DEFAULT_RELAY_STATES[url as keyof typeof DEFAULT_RELAY_STATES];
    return {
      url,
      enabledForProfile: defaults?.enabledForProfile ?? true,
      enabledForWorld: defaults?.enabledForWorld ?? true,
    };
  });
}

/**
 * Get all enabled relays for world/chat activities
 * (avatar state, chat, world storage)
 */
export function getEnabledWorldRelays(): string[] {
  const settings = loadRelaySettings();
  const enabled = settings
    .filter(relay => relay.enabledForWorld)
    .map(relay => relay.url);

  // Return enabled relays, even if empty (user choice)
  // Services should handle empty array gracefully
  return enabled;
}

/**
 * Get all enabled relays for profile activities
 * (fetching user profiles, metadata, etc.)
 */
export function getEnabledProfileRelays(): string[] {
  const settings = loadRelaySettings();
  const enabled = settings
    .filter(relay => relay.enabledForProfile)
    .map(relay => relay.url);

  // Return enabled relays, even if empty (user choice)
  return enabled;
}

/**
 * Get all enabled relays (for any purpose)
 */
export function getAllEnabledRelays(): string[] {
  const settings = loadRelaySettings();
  const enabled = settings
    .filter(relay => relay.enabledForProfile || relay.enabledForWorld)
    .map(relay => relay.url);

  // Return enabled relays, even if empty (user choice)
  return enabled;
}

/**
 * Check if a specific relay is enabled for world activities
 */
export function isRelayEnabledForWorld(relayUrl: string): boolean {
  const settings = loadRelaySettings();
  const relay = settings.find(r => r.url === relayUrl);
  return relay?.enabledForWorld ?? false;
}

/**
 * Check if a specific relay is enabled for profile activities
 */
export function isRelayEnabledForProfile(relayUrl: string): boolean {
  const settings = loadRelaySettings();
  const relay = settings.find(r => r.url === relayUrl);
  return relay?.enabledForProfile ?? false;
}
