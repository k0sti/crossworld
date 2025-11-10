// Re-export shared config from common package
export { DEFAULT_RELAYS, DEFAULT_RELAY_STATES, WORLD_RELAYS, APP_NPUB, APP_PUBKEY } from '@crossworld/common'
import { APP_PUBKEY } from '@crossworld/common'

// Live chat event (NIP-53)
export const LIVE_CHAT_D_TAG = 'crossworld-dev'

// Chat history configuration
export const CHAT_HISTORY_CONFIG = {
  // Maximum number of old messages to fetch
  MAX_MESSAGES: 100,
  // Maximum time range for old messages (in seconds)
  // Default: 24 hours = 24 * 60 * 60 = 86400 seconds
  MAX_TIME_RANGE_S: 24 * 60 * 60,
}

// Generate live chat a-tag dynamically from components
export function getLiveChatATag(): string {
  return `30311:${APP_PUBKEY}:${LIVE_CHAT_D_TAG}`
}

// Avatar voxel model constants
export const AVATAR_VOXEL_CONSTANTS = {
  // Voxel grid size (width, height, depth in voxels)
  GRID_SIZE_X: 16,
  GRID_SIZE_Y: 32,
  GRID_SIZE_Z: 16,

  // Voxel size in world units
  VOXEL_SIZE: 0.1,

  // Model dimensions in world units
  get MODEL_WIDTH() { return this.GRID_SIZE_X * this.VOXEL_SIZE; },
  get MODEL_HEIGHT() { return this.GRID_SIZE_Y * this.VOXEL_SIZE; },
  get MODEL_DEPTH() { return this.GRID_SIZE_Z * this.VOXEL_SIZE; },
}

// Avatar State Event System
export const AVATAR_STATE_CONFIG = {
  // Event kinds
  STATE_EVENT_KIND: 30317,  // Addressable: Full avatar state
  UPDATE_EVENT_KIND: 1317,  // Regular: Incremental updates

  // Time windows (in seconds)
  SUBSCRIPTION_WINDOW_S: 60 * 60,  // 1 hour - how far back to query events
  STATE_TTL_S: 30 * 60,            // 30 minutes - when to consider user offline
  EVENT_EXPIRY_S: 60 * 60,         // 1 hour - expiration tag on events

  // Update intervals (in milliseconds)
  POSITION_UPDATE_MS: 500,     // Position updates while moving
  HEARTBEAT_INTERVAL_MS: 60000, // Heartbeat to keep presence alive
}

// Generate avatar state d-tag for a world
export function getAvatarStateDTag(): string {
  return `crossworld-avatar-${getLiveChatATag()}`
}

// Voice/MoQ Configuration
export const VOICE_CONFIG = {
  // Debug: override relay URL (set to null to use event-based URL)
  // DEBUG_RELAY_URL: 'https://moq.justinmoon.com/anon' as string | null,
  DEBUG_RELAY_URL: null,  // Uncomment to use URL from live event
}
