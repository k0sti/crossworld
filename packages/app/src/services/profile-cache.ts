import * as logger from '../utils/logger';
import { Relay } from 'applesauce-relay'

export interface ProfileMetadata {
  name?: string
  picture?: string
  display_name?: string
  about?: string
}

interface CachedProfile {
  data: ProfileMetadata
  timestamp: number
}

/**
 * Centralized profile cache service
 *
 * Caches Nostr profile metadata (kind 0 events) with a 5-minute TTL.
 * Prevents duplicate fetches and reduces relay load.
 */
class ProfileCacheService {
  private cache: Map<string, CachedProfile> = new Map()
  private fetchQueue: Set<string> = new Set()
  private readonly CACHE_TTL_MS = 5 * 60 * 1000 // 5 minutes

  /**
   * Get profile from cache or fetch from relays
   */
  async getProfile(pubkey: string, relays: string[]): Promise<ProfileMetadata | null> {
    // Check cache first
    const cached = this.cache.get(pubkey)
    if (cached && Date.now() - cached.timestamp < this.CACHE_TTL_MS) {
      return cached.data
    }

    // Check if already fetching
    if (this.fetchQueue.has(pubkey)) {
      // Wait for existing fetch to complete
      return new Promise((resolve) => {
        const checkInterval = setInterval(() => {
          const updated = this.cache.get(pubkey)
          if (updated || !this.fetchQueue.has(pubkey)) {
            clearInterval(checkInterval)
            resolve(updated?.data || null)
          }
        }, 100)

        // Timeout after 5 seconds
        setTimeout(() => {
          clearInterval(checkInterval)
          resolve(null)
        }, 5000)
      })
    }

    // Fetch from relays
    if (relays.length === 0) {
      return null
    }

    this.fetchQueue.add(pubkey)

    try {
      const profile = await this.fetchFromRelays(pubkey, relays)
      if (profile) {
        this.cache.set(pubkey, {
          data: profile,
          timestamp: Date.now()
        })
      }
      return profile
    } finally {
      this.fetchQueue.delete(pubkey)
    }
  }

  /**
   * Fetch profile from relays
   */
  private async fetchFromRelays(pubkey: string, relays: string[]): Promise<ProfileMetadata | null> {
    for (const relayUrl of relays) {
      try {
        const relay = new Relay(relayUrl)
        const events = await new Promise<any[]>((resolve) => {
          const collectedEvents: any[] = []
          let isResolved = false

          const cleanup = () => {
            if (!isResolved) {
              isResolved = true
              try { relay.close() } catch (e) {}
              resolve(collectedEvents)
            }
          }

          relay.request({
            kinds: [0],
            authors: [pubkey],
            limit: 1
          }).subscribe({
            next: (event: any) => {
              if (event === 'EOSE') {
                cleanup()
              } else if (event && event.kind === 0) {
                collectedEvents.push(event)
              }
            },
            error: () => cleanup(),
            complete: () => cleanup()
          })

          setTimeout(cleanup, 3000)
        })

        if (events.length > 0) {
          const latestEvent = events.sort((a, b) => b.created_at - a.created_at)[0]
          try {
            const metadata = JSON.parse(latestEvent.content) as ProfileMetadata
            return metadata
          } catch (e) {
            logger.error('profile', '[ProfileCache] Failed to parse profile metadata:', e)
          }
        }
      } catch (error) {
        logger.error('profile', `[ProfileCache] Failed to fetch from ${relayUrl}:`, error)
      }
    }

    return null
  }

  /**
   * Check if profile is cached and still valid
   */
  isCached(pubkey: string): boolean {
    const cached = this.cache.get(pubkey)
    return cached !== undefined && Date.now() - cached.timestamp < this.CACHE_TTL_MS
  }

  /**
   * Get profile from cache only (synchronous)
   */
  getCached(pubkey: string): ProfileMetadata | null {
    const cached = this.cache.get(pubkey)
    if (cached && Date.now() - cached.timestamp < this.CACHE_TTL_MS) {
      return cached.data
    }
    return null
  }

  /**
   * Manually set profile in cache (useful for optimistic updates)
   */
  setProfile(pubkey: string, profile: ProfileMetadata): void {
    this.cache.set(pubkey, {
      data: profile,
      timestamp: Date.now()
    })
  }

  /**
   * Clear entire cache
   */
  clearCache(): void {
    this.cache.clear()
    this.fetchQueue.clear()
  }

  /**
   * Clear cache for specific pubkey
   */
  clearProfile(pubkey: string): void {
    this.cache.delete(pubkey)
  }

  /**
   * Prune expired entries from cache
   */
  pruneCache(): void {
    const now = Date.now()
    for (const [pubkey, cached] of this.cache.entries()) {
      if (now - cached.timestamp >= this.CACHE_TTL_MS) {
        this.cache.delete(pubkey)
      }
    }
  }
}

// Export singleton instance
export const profileCache = new ProfileCacheService()

// Prune cache every minute
setInterval(() => {
  profileCache.pruneCache()
}, 60 * 1000)
