import * as logger from '../utils/logger';
import { SimplePool, type Event } from 'nostr-tools'
import { WORLD_RELAYS } from '../config'
import { getMacroDepth, getMicroDepth } from '../config/depth-config'
import type { AccountManager } from 'applesauce-accounts'

// Singleton pool for world storage operations
let sharedPool: SimplePool | null = null

function getPool(): SimplePool {
  if (!sharedPool) {
    sharedPool = new SimplePool()
  }
  return sharedPool
}

export interface WorldMetadata {
  pubkey: string
  dTag: string
  title?: string
  description?: string
  macroDepth: number
  microDepth: number
  octantPath: string
  csmCode: string
  createdAt: number
  eventId: string
}

export interface PublishWorldOptions {
  octantPath?: string
  title?: string
  description?: string
}

/**
 * Parse d-tag into components
 */
export function parseDTag(dTag: string): {
  octantPath: string
  macroDepth: number
  microDepth: number
} {
  const parts = dTag.split(':')
  let octantPath = ''
  let macroDepth = 3
  let microDepth = 0

  if (parts.length === 2) {
    // Format: ":3" or "abc:3"
    octantPath = parts[0]
    macroDepth = parseInt(parts[1])
  } else if (parts.length === 3) {
    // Format: ":3:2" or "abc:3:2"
    octantPath = parts[0]
    macroDepth = parseInt(parts[1])
    microDepth = parseInt(parts[2])
  }

  return { octantPath, macroDepth, microDepth }
}

/**
 * Build d-tag from components
 */
export function buildDTag(
  octantPath: string = '',
  macroDepth: number,
  microDepth: number
): string {
  if (microDepth > 0) {
    return `${octantPath}:${macroDepth}:${microDepth}`
  }
  return `${octantPath}:${macroDepth}`
}

/**
 * Publish world to Nostr
 */
export async function publishWorld(
  accountManager: AccountManager,
  csmContent: string,
  options: PublishWorldOptions = {}
): Promise<Event> {
  const pool = getPool()

  // Get accounts array and use first account
  const accounts = Array.from(accountManager.accounts.values())
  if (accounts.length === 0) {
    throw new Error('No account available')
  }

  const account = accounts[0]
  const pubkey = await account.signer.getPublicKey()
  const macroDepth = getMacroDepth()
  const microDepth = getMicroDepth()
  const octantPath = options.octantPath || ''

  // Build d-tag
  const dTag = buildDTag(octantPath, macroDepth, microDepth)

  // Build event
  const unsignedEvent = {
    kind: 30078,
    created_at: Math.floor(Date.now() / 1000),
    tags: [
      ['d', dTag],
      ['macro', macroDepth.toString()],
      ['micro', microDepth.toString()],
      ...(options.title ? [['title', options.title]] : []),
      ...(options.description ? [['description', options.description]] : []),
    ],
    content: csmContent,
    pubkey,
  }

  // Sign event
  const signedEvent = await account.signer.signEvent(unsignedEvent)

  // Publish to WORLD_RELAYS using persistent pool
  if (!WORLD_RELAYS || WORLD_RELAYS.length === 0) {
    logger.warn('storage', 'No world relays configured, world not published')
    // Return signed event even if we can't publish (for local storage)
    return signedEvent
  }

  try {
    await pool.publish(WORLD_RELAYS, signedEvent)
  } catch (err) {
    logger.warn('storage', 'Failed to publish world (relay may be unavailable):', err)
    // Return signed event anyway - it's still valid for local use
    return signedEvent
  }

  return signedEvent
}

/**
 * Fetch all worlds for a user
 */
export async function fetchUserWorlds(pubkey: string): Promise<WorldMetadata[]> {
  const pool = getPool()

  // Check if world relays are configured
  if (!WORLD_RELAYS || WORLD_RELAYS.length === 0) {
    logger.warn('storage', 'No world relays configured, cannot fetch worlds')
    return []
  }

  try {
    const events = await pool.querySync(WORLD_RELAYS, {
      kinds: [30078],
      authors: [pubkey],
    })

    return events.map(event => {
      const dTag = event.tags.find(t => t[0] === 'd')?.[1] || ''
      const parsed = parseDTag(dTag)

      return {
        pubkey: event.pubkey,
        dTag,
        title: event.tags.find(t => t[0] === 'title')?.[1],
        description: event.tags.find(t => t[0] === 'description')?.[1],
        macroDepth: parsed.macroDepth,
        microDepth: parsed.microDepth,
        octantPath: parsed.octantPath,
        csmCode: event.content,
        createdAt: event.created_at,
        eventId: event.id,
      }
    })
  } catch (error) {
    logger.warn('storage', 'Failed to fetch user worlds (relay may be unavailable):', error)
    return []
  }
}

/**
 * Fetch specific world by d-tag
 */
export async function fetchWorldByDTag(
  pubkey: string,
  dTag: string
): Promise<WorldMetadata | null> {
  const pool = getPool()

  // Check if world relays are configured
  if (!WORLD_RELAYS || WORLD_RELAYS.length === 0) {
    logger.warn('storage', 'No world relays configured, cannot fetch world')
    return null
  }

  try {
    const event = await pool.get(WORLD_RELAYS, {
      kinds: [30078],
      authors: [pubkey],
      '#d': [dTag],
    })

    if (!event) return null

    const parsed = parseDTag(dTag)

    return {
      pubkey: event.pubkey,
      dTag,
      title: event.tags.find(t => t[0] === 'title')?.[1],
      description: event.tags.find(t => t[0] === 'description')?.[1],
      macroDepth: parsed.macroDepth,
      microDepth: parsed.microDepth,
      octantPath: parsed.octantPath,
      csmCode: event.content,
      createdAt: event.created_at,
      eventId: event.id,
    }
  } catch (error) {
    logger.warn('storage', 'Failed to fetch world (relay may be unavailable):', error)
    return null
  }
}

/**
 * Fetch world matching current configuration
 */
export async function fetchCurrentWorld(
  pubkey: string,
  octantPath: string = ''
): Promise<WorldMetadata | null> {
  const macroDepth = getMacroDepth()
  const microDepth = getMicroDepth()
  const dTag = buildDTag(octantPath, macroDepth, microDepth)

  return fetchWorldByDTag(pubkey, dTag)
}

/**
 * Validate if world matches current configuration
 */
export function validateWorldConfig(world: WorldMetadata): {
  valid: boolean
  error?: string
} {
  const currentMacro = getMacroDepth()
  const currentMicro = getMicroDepth()

  if (world.macroDepth !== currentMacro) {
    return {
      valid: false,
      error: `Macro depth mismatch: world=${world.macroDepth}, current=${currentMacro}`
    }
  }

  if (world.microDepth !== currentMicro) {
    return {
      valid: false,
      error: `Micro depth mismatch: world=${world.microDepth}, current=${currentMicro}`
    }
  }

  return { valid: true }
}
