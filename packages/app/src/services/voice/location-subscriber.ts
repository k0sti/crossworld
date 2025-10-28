import * as logger from '../../utils/logger';
import * as Moq from '@kixelated/moq'
import { Effect, Signal } from '@kixelated/signals'
import { MoqConnectionManager } from './connection'
import { LIVE_CHAT_D_TAG } from '../../config'

export interface ParticipantLocation {
  npub: string
  x: number
  y: number
  z?: number
  timestamp: number
  discoverySource: 'nostr' | 'moq' | 'both'
}

/**
 * Watcher for a single participant's location stream
 */
class LocationWatcher {
  public readonly npub: string
  private track: Moq.Track | null = null
  private signals = new Effect()
  public discoverySource: 'nostr' | 'moq' | 'both' = 'moq'

  // Public signals for UI
  public readonly location: Signal<ParticipantLocation | null> = new Signal(null)

  constructor(connection: Moq.Connection.Established, npub: string, source: 'nostr' | 'moq' = 'moq') {
    this.npub = npub
    this.discoverySource = source

    const path = Moq.Path.from(`crossworld/location/${LIVE_CHAT_D_TAG}/${npub}`)
    logger.log('voice', '[Location Subscriber] Creating watcher for:', {
      npub,
      path: String(path),
      source,
    })

    try {
      // Subscribe to location broadcast
      const broadcast = connection.consume(path)
      this.track = broadcast.subscribe('location', 0)

      // Start reading location updates
      this.signals.effect((effect) => {
        effect.spawn(async () => {
          if (!this.track) return

          try {
            for (;;) {
              const data = await this.track.readJson() as { location?: string }
              if (!data) break

              // Parse location data
              if (data.location) {
                const location = JSON.parse(data.location)
                const participantLocation: ParticipantLocation = {
                  npub,
                  x: location.x,
                  y: location.y,
                  z: location.z,
                  timestamp: location.timestamp,
                  discoverySource: this.discoverySource,
                }
                this.location.set(participantLocation)
                logger.log('voice', '[Location Subscriber] Received location for', npub, ':', location)
              }
            }
          } catch (err) {
            logger.error('voice', '[Location Subscriber] Error reading location for', npub, ':', err)
          }
        })
      })

      logger.log('voice', '[Location Subscriber] Watcher created for:', npub)
    } catch (err) {
      logger.error('voice', '[Location Subscriber] Failed to create watcher for', npub, ':', err)
    }
  }

  /**
   * Mark as discovered via both sources
   */
  markDualDiscovery(): void {
    if (this.discoverySource !== 'both') {
      logger.log('voice', '[Location Subscriber] Participant now discovered via both sources:', this.npub)
      this.discoverySource = 'both'
    }
  }

  /**
   * Clean up resources
   */
  close(): void {
    logger.log('voice', '[Location Subscriber] Closing watcher for:', this.npub)
    this.signals.close()
    if (this.track) {
      this.track.close()
      this.track = null
    }
  }
}

/**
 * Location subscriber with dual discovery: Nostr + MoQ announcements
 * Subscribes to participant location broadcasts
 */
export class LocationSubscriber {
  private connection: MoqConnectionManager
  private watchers = new Map<string, LocationWatcher>()
  private ownNpub: string | null = null
  private announcementAbortController: AbortController | null = null
  private signals = new Effect()

  public locations: Signal<Map<string, ParticipantLocation>> = new Signal(new Map())

  constructor(connection: MoqConnectionManager) {
    this.connection = connection
  }

  /**
   * Set our own npub to avoid watching ourselves
   */
  setOwnNpub(npub: string): void {
    this.ownNpub = npub
    logger.log('voice', '[Location Subscriber] Own npub set:', npub)
  }

  /**
   * Start listening for location broadcasts via MoQ announcements
   */
  async startListening(): Promise<void> {
    const conn = this.connection.getConnection()
    if (!conn) {
      throw new Error('Not connected to MoQ relay')
    }

    logger.log('voice', '[Location Subscriber] Starting location discovery via MoQ announcements...')
    this.startAnnouncementListener(conn)
    logger.log('voice', '[Location Subscriber] Now listening for location broadcasts')
  }

  /**
   * Start listening for MoQ announcements
   */
  private async startAnnouncementListener(connection: Moq.Connection.Established): Promise<void> {
    if (this.announcementAbortController) {
      this.announcementAbortController.abort()
    }

    this.announcementAbortController = new AbortController()
    const signal = this.announcementAbortController.signal

    const prefix = Moq.Path.from(`crossworld/location/${LIVE_CHAT_D_TAG}`)
    logger.log('voice', '[Location Subscriber] Listening for announcements with prefix:', String(prefix))

    const announced = connection.announced(prefix)

    const loop = (async () => {
      try {
        for (;;) {
          if (signal.aborted) break

          const entry = await announced.next()
          if (!entry) break

          logger.log('voice', '[Location Subscriber] Announcement received:', {
            path: String(entry.path),
            active: entry.active,
          })

          if (entry.active) {
            this.handleAnnouncementAdded(connection, entry.path)
          } else {
            this.handleAnnouncementRemoved(entry.path)
          }
        }
      } catch (err) {
        if (!signal.aborted) {
          logger.error('voice', '[Location Subscriber] Announcement loop failed:', err)
        }
      }
    })()

    loop.catch((err) => {
      if (!signal.aborted) {
        logger.error('voice', '[Location Subscriber] Announcement loop error:', err)
      }
    })
  }

  /**
   * Handle MoQ announcement added
   */
  private handleAnnouncementAdded(connection: Moq.Connection.Established, path: Moq.Path.Valid): void {
    const pathStr = String(path)
    const segments = pathStr.split('/')
    const npub = segments[segments.length - 1]

    if (!npub || npub === this.ownNpub) {
      logger.log('voice', '[Location Subscriber] Skipping own broadcast or invalid npub')
      return
    }

    logger.log('voice', '[Location Subscriber] Participant location announced:', npub)

    const existing = this.watchers.get(npub)
    if (existing) {
      existing.markDualDiscovery()
      this.updateLocationsList()
    } else {
      this.createWatcher(connection, npub)
    }
  }

  /**
   * Handle MoQ announcement removed
   */
  private handleAnnouncementRemoved(path: Moq.Path.Valid): void {
    const pathStr = String(path)
    const segments = pathStr.split('/')
    const npub = segments[segments.length - 1]

    if (!npub) return

    logger.log('voice', '[Location Subscriber] Location announcement ended:', npub)
    this.removeWatcher(npub)
  }

  /**
   * Create watcher for a participant
   */
  private createWatcher(connection: Moq.Connection.Established, npub: string, source: 'nostr' | 'moq' = 'moq'): void {
    try {
      logger.log('voice', '[Location Subscriber] Creating watcher for:', npub)
      const watcher = new LocationWatcher(connection, npub, source)

      // Subscribe to location changes
      watcher.location.subscribe(() => {
        this.updateLocationsList()
      })

      this.watchers.set(npub, watcher)
      logger.log('voice', '[Location Subscriber] Watcher created. Total watchers:', this.watchers.size)
      this.updateLocationsList()
    } catch (err) {
      logger.error('voice', '[Location Subscriber] Failed to create watcher for', npub, ':', err)
    }
  }

  /**
   * Remove watcher for a participant
   */
  private removeWatcher(npub: string): void {
    const watcher = this.watchers.get(npub)
    if (watcher) {
      watcher.close()
      this.watchers.delete(npub)
      logger.log('voice', '[Location Subscriber] Watcher removed. Remaining:', this.watchers.size)
      this.updateLocationsList()
    }
  }

  /**
   * Update the locations signal with current state
   */
  private updateLocationsList(): void {
    const locations = new Map<string, ParticipantLocation>()

    for (const [npub, watcher] of this.watchers) {
      const location = watcher.location.peek()
      if (location) {
        locations.set(npub, location)
      }
    }

    this.locations.set(locations)
  }

  /**
   * Get location for a specific participant
   */
  getLocation(npub: string): ParticipantLocation | null {
    const watcher = this.watchers.get(npub)
    return watcher?.location.peek() ?? null
  }

  /**
   * Stop listening and clean up all watchers
   */
  stopListening(): void {
    logger.log('voice', '[Location Subscriber] Stopping location listening...')

    if (this.announcementAbortController) {
      this.announcementAbortController.abort()
      this.announcementAbortController = null
    }

    const count = this.watchers.size
    for (const [, watcher] of this.watchers) {
      watcher.close()
    }
    this.watchers.clear()

    this.locations.set(new Map())

    logger.log('voice', '[Location Subscriber] Stopped listening. Closed', count, 'watchers')
  }

  /**
   * Clean up all resources
   */
  close(): void {
    this.stopListening()
    this.signals.close()
  }
}
