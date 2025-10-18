export const DEFAULT_RELAYS = [
  'wss://strfry.atlantislabs.space/',
  'wss://relay.damus.io',
  'wss://nos.lol',
  'wss://relay.primal.net',
]

export const DEFAULT_RELAY_STATES = {
  'wss://strfry.atlantislabs.space/': { enabledForProfile: false, enabledForWorld: true },
  'wss://relay.damus.io': { enabledForProfile: true, enabledForWorld: false },
  'wss://nos.lol': { enabledForProfile: true, enabledForWorld: false },
  'wss://relay.primal.net': { enabledForProfile: true, enabledForWorld: false },
}

// World relays for client status and chat
export const WORLD_RELAYS = ['wss://strfry.atlantislabs.space/']

// Crossworld app identity
export const APP_NPUB = 'npub1ga6mzn7ygwuxpytr264uw09huwef9ypzfda767088gv83ypgtjtsxf25vh'

// Derive APP_PUBKEY from APP_NPUB
import { nip19 } from 'nostr-tools'
export const APP_PUBKEY = nip19.decode(APP_NPUB).data as string

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
