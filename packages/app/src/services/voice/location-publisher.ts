import * as logger from '../../utils/logger';
import * as Moq from '@kixelated/moq'
import { Effect, Signal } from '@kixelated/signals'
import { MoqConnectionManager } from './connection'
import { LIVE_CHAT_D_TAG } from '../../config'

export interface LocationData {
  x: number
  y: number
  z?: number
  timestamp: number
}

/**
 * Location publisher for broadcasting position data via MoQ
 * Similar to AudioPublisher but for location/position data
 */
export class LocationPublisher {
  private connection: MoqConnectionManager
  private signals = new Effect()

  // Input signals
  private enabledSignal: Signal<boolean> = new Signal(false)
  private npubSignal: Signal<string | undefined> = new Signal(undefined)
  private locationSignal: Signal<LocationData | undefined> = new Signal(undefined)

  // Broadcast instance
  private broadcast: Moq.Broadcast | null = null

  // Output signals
  public readonly enabled: Signal<boolean> = new Signal(false)
  public readonly error: Signal<string | null> = new Signal(null)

  constructor(connection: MoqConnectionManager) {
    this.connection = connection

    // Setup reactive broadcast creation
    this.signals.effect(this.#runBroadcast.bind(this))
  }

  /**
   * Enable location broadcasting
   */
  async enable(npub: string): Promise<void> {
    if (this.enabledSignal.peek()) {
      logger.log('voice', '[Location Publisher] Already enabled')
      return
    }

    logger.log('voice', '[Location Publisher] Enabling for:', npub)
    this.npubSignal.set(npub)
    this.enabledSignal.set(true)
    this.enabled.set(true)
  }

  /**
   * Disable location broadcasting
   */
  async disable(): Promise<void> {
    if (!this.enabledSignal.peek()) {
      return
    }

    logger.log('voice', '[Location Publisher] Disabling')
    this.enabledSignal.set(false)
    this.npubSignal.set(undefined)
    this.enabled.set(false)
  }

  /**
   * Update current location
   */
  updateLocation(x: number, y: number, z?: number): void {
    const location: LocationData = {
      x,
      y,
      z,
      timestamp: Date.now(),
    }
    this.locationSignal.set(location)

    // Send immediately if broadcast is active
    if (this.broadcast) {
      this.sendLocation(location)
    }
  }

  /**
   * Send location data via MoQ track
   */
  private sendLocation(location: LocationData): void {
    if (!this.broadcast) return

    try {
      const track = this.broadcast.subscribe('location', 0)
      const data = JSON.stringify(location)
      track.writeJson({ location: data })
      logger.log('voice', '[Location Publisher] Sent location:', location)
    } catch (err) {
      logger.error('voice', '[Location Publisher] Failed to send location:', err)
    }
  }

  /**
   * Create and manage broadcast
   */
  #runBroadcast(effect: Effect): void {
    const conn = effect.get(this.connection.established)
    if (!conn) return

    const npub = effect.get(this.npubSignal)
    if (!npub) return

    const enabled = effect.get(this.enabledSignal)
    if (!enabled) return

    // Create broadcast path
    const path = Moq.Path.from(`crossworld/location/${LIVE_CHAT_D_TAG}/${npub}`)
    logger.log('voice', '[Location Publisher] Creating broadcast:', {
      path: String(path),
      npub,
      dTag: LIVE_CHAT_D_TAG,
    })

    // Create broadcast
    this.broadcast = new Moq.Broadcast()
    conn.publish(path, this.broadcast)

    logger.log('voice', '[Location Publisher] Broadcast created and active')

    // Send current location if available
    const currentLocation = this.locationSignal.peek()
    if (currentLocation) {
      this.sendLocation(currentLocation)
    }

    // Cleanup: close broadcast when effect ends
    effect.cleanup(() => {
      logger.log('voice', '[Location Publisher] Closing broadcast:', String(path))
      this.broadcast?.close()
      this.broadcast = null
    })
  }

  /**
   * Check if enabled
   */
  isEnabled(): boolean {
    return this.enabled.peek()
  }

  /**
   * Clean up all resources
   */
  close(): void {
    this.signals.close()
    if (this.broadcast) {
      this.broadcast.close()
      this.broadcast = null
    }
  }
}
