import * as Hang from '@kixelated/hang'
import * as Moq from '@kixelated/moq'
import { Effect, Signal } from '@kixelated/signals'
import { MoqConnectionManager } from './connection'
import { LIVE_CHAT_D_TAG } from '../../config'
import { AvatarStateService, type AvatarState } from '../avatar-state'

export interface Participant {
  npub: string
  speaking: boolean
  volume: number
  muted: boolean
  discoverySource: 'nostr' | 'moq' | 'both'
}

/**
 * Watcher for a single participant's audio stream
 * Based on ref/moq/js/hang/src/watch/element.ts
 */
class ParticipantWatcher {
  public readonly npub: string
  private watcher: Hang.Watch.Broadcast
  private emitter: Hang.Watch.Audio.Emitter
  private signals = new Effect()

  // Public signals for UI
  public readonly speaking: Signal<boolean> = new Signal(false)
  public readonly volume: Signal<number> = new Signal(0.5)
  public readonly muted: Signal<boolean> = new Signal(false)
  public discoverySource: 'nostr' | 'moq' | 'both' = 'nostr'

  constructor(connection: Moq.Connection.Established, npub: string, source: 'nostr' | 'moq' = 'nostr') {
    this.npub = npub
    this.discoverySource = source

    const path = Moq.Path.from(`crossworld/voice/${LIVE_CHAT_D_TAG}/${npub}`)
    console.log('[MoQ Subscriber] Creating watcher for participant:', {
      npub,
      path: String(path),
      source,
    })

    // Create watcher (ref: ref/moq/js/hang/src/watch/element.ts:221-238)
    this.watcher = new Hang.Watch.Broadcast({
      connection,
      path,
      enabled: true,
      audio: {
        enabled: true,
        latency: 100 as any, // 100ms jitter buffer (Milli type)
        speaking: {
          enabled: true,
        },
      },
    })

    // Create emitter to connect audio to speakers (ref: ref/moq/js/hang/src/watch/element.ts:251-255)
    this.emitter = new Hang.Watch.Audio.Emitter(this.watcher.audio, {
      volume: this.volume,
      muted: this.muted,
      paused: new Signal(false),
    })

    // Subscribe to speaking state
    this.signals.effect((effect) => {
      const isSpeaking = effect.get(this.watcher.audio.speaking.active)
      if (isSpeaking !== undefined) {
        console.log('[MoQ Subscriber] Speaking state for', npub, ':', isSpeaking)
      }
      this.speaking.set(isSpeaking ?? false)
    })

    console.log('[MoQ Subscriber] Watcher created and active for:', npub)
  }

  /**
   * Mark this watcher as discovered via both sources
   */
  markDualDiscovery(): void {
    if (this.discoverySource !== 'both') {
      console.log('[MoQ Subscriber] Participant now discovered via both Nostr and MoQ:', this.npub)
      this.discoverySource = 'both'
    }
  }

  /**
   * Set volume for this participant (0.0 - 1.0)
   */
  setVolume(volume: number): void {
    this.volume.set(Math.max(0, Math.min(1, volume)))
  }

  /**
   * Mute/unmute this participant
   */
  setMuted(muted: boolean): void {
    this.muted.set(muted)
  }

  /**
   * Clean up resources
   */
  close(): void {
    console.log('[MoQ Subscriber] Closing watcher for:', this.npub)
    this.signals.close()
    this.emitter.close()
    this.watcher.close()
  }
}

/**
 * Audio subscriber with dual discovery: Nostr AvatarStateService + MoQ announcements
 * Discovery based on both sources for maximum reliability
 */
export class AudioSubscriber {
  private connection: MoqConnectionManager
  private watchers = new Map<string, ParticipantWatcher>()
  private ownNpub: string | null = null
  private avatarStateService: AvatarStateService | null = null
  private unsubscribeAvatarState: (() => void) | null = null
  private announcementAbortController: AbortController | null = null
  private signals = new Effect()

  public participants: Signal<Map<string, Participant>> = new Signal(new Map())

  // Debug state
  public announcementsReceived = 0
  public activeAnnouncementCount = 0

  constructor(connection: MoqConnectionManager) {
    this.connection = connection
  }

  /**
   * Set the avatar state service for Nostr-based discovery
   */
  setClientStatusService(service: AvatarStateService): void {
    this.avatarStateService = service
  }

  /**
   * Set our own npub to avoid watching ourselves
   */
  setOwnNpub(npub: string): void {
    this.ownNpub = npub
    console.log('[MoQ Subscriber] Own npub set:', npub)
  }

  /**
   * Start listening for participants via BOTH Nostr and MoQ announcements
   */
  async startListening(): Promise<void> {
    const conn = this.connection.getConnection()
    if (!conn) {
      throw new Error('Not connected to MoQ relay')
    }

    console.log('[MoQ Subscriber] Starting DUAL discovery (Nostr + MoQ announcements)...')

    // 1. Start Nostr-based discovery (AvatarStateService)
    if (this.avatarStateService) {
      console.log('[MoQ Subscriber] Starting Nostr-based discovery...')
      this.unsubscribeAvatarState = this.avatarStateService.onChange((states) => {
        console.log('[MoQ Subscriber] Avatar states updated (Nostr), processing', states.size, 'users')
        this.handleClientListUpdate(conn, states)
      })
      console.log('[MoQ Subscriber] Nostr discovery active')
    } else {
      console.warn('[MoQ Subscriber] AvatarStateService not set - Nostr discovery disabled')
    }

    // 2. Start MoQ announcement-based discovery
    console.log('[MoQ Subscriber] Starting MoQ announcement-based discovery...')
    this.startAnnouncementListener(conn)

    console.log('[MoQ Subscriber] Now listening via BOTH discovery methods')
  }

  /**
   * Start listening for MoQ announcements (like ref/innpub)
   */
  private async startAnnouncementListener(connection: Moq.Connection.Established): Promise<void> {
    // Cancel any existing listener
    if (this.announcementAbortController) {
      this.announcementAbortController.abort()
    }

    this.announcementAbortController = new AbortController()
    const signal = this.announcementAbortController.signal

    const prefix = Moq.Path.from(`crossworld/voice/${LIVE_CHAT_D_TAG}`)
    console.log('[MoQ Subscriber] Listening for announcements with prefix:', String(prefix))

    const announced = connection.announced(prefix)

    // Start announcement loop (ref: ref/innpub/src/multiplayer/stream.ts:1573-1602)
    const loop = (async () => {
      try {
        let count = 0
        for (;;) {
          if (signal.aborted) {
            console.log('[MoQ Subscriber] Announcement listener aborted')
            break
          }

          const entry = await announced.next()
          if (!entry) {
            console.log('[MoQ Subscriber] Announcement stream ended')
            break
          }

          count++
          this.announcementsReceived = count

          console.log('[MoQ Subscriber] Announcement received:', {
            path: String(entry.path),
            active: entry.active,
            totalReceived: count,
          })

          if (entry.active) {
            this.activeAnnouncementCount++
            this.handleAnnouncementAdded(connection, entry.path)
          } else {
            this.activeAnnouncementCount = Math.max(0, this.activeAnnouncementCount - 1)
            this.handleAnnouncementRemoved(entry.path)
          }
        }
      } catch (err) {
        if (!signal.aborted) {
          console.error('[MoQ Subscriber] Announcement loop failed:', err)
        }
      }
    })()

    // Don't await - let it run in background
    loop.catch((err) => {
      if (!signal.aborted) {
        console.error('[MoQ Subscriber] Announcement loop error:', err)
      }
    })
  }

  /**
   * Handle MoQ announcement added
   */
  private handleAnnouncementAdded(connection: Moq.Connection.Established, path: Moq.Path.Valid): void {
    // Extract npub from path: crossworld/voice/crossworld-dev/npub1...
    const pathStr = String(path)
    const segments = pathStr.split('/')
    const npub = segments[segments.length - 1]

    if (!npub || npub === this.ownNpub) {
      console.log('[MoQ Subscriber] Skipping own broadcast or invalid npub')
      return
    }

    console.log('[MoQ Subscriber] Participant announced via MoQ:', npub)

    // Check if we already have a watcher from Nostr discovery
    const existing = this.watchers.get(npub)
    if (existing) {
      console.log('[MoQ Subscriber] Participant already being watched (Nostr), marking as dual discovery')
      existing.markDualDiscovery()
      this.updateParticipantsList()
    } else {
      // Create new watcher discovered via MoQ
      console.log('[MoQ Subscriber] Creating new watcher from MoQ announcement')
      this.createWatcher(connection, npub, 'moq')
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

    console.log('[MoQ Subscriber] Participant announcement ended (MoQ):', npub)

    // Don't immediately remove if also discovered via Nostr
    const watcher = this.watchers.get(npub)
    if (watcher && watcher.discoverySource === 'both') {
      console.log('[MoQ Subscriber] Keeping watcher (still active via Nostr)')
      watcher.discoverySource = 'nostr'
      this.updateParticipantsList()
    } else if (watcher && watcher.discoverySource === 'moq') {
      // Only discovered via MoQ, remove it
      console.log('[MoQ Subscriber] Removing watcher (only discovered via MoQ)')
      this.removeWatcher(npub)
    }
  }

  /**
   * Handle avatar state update from AvatarStateService (Nostr discovery)
   */
  private handleClientListUpdate(
    connection: Moq.Connection.Established,
    states: Map<string, AvatarState>
  ): void {
    // Track which npubs should be active via Nostr
    const activeNpubs = new Set<string>()

    // Process each state
    states.forEach((state) => {
      // Skip users not in voice chat
      if (!state.voiceConnected) {
        return
      }

      // Skip ourselves
      if (this.ownNpub && state.npub === this.ownNpub) {
        return
      }

      activeNpubs.add(state.npub)

      // Create or update watcher
      const existing = this.watchers.get(state.npub)
      if (existing) {
        // Already watching, check if we should mark as dual discovery
        if (existing.discoverySource === 'moq') {
          console.log('[MoQ Subscriber] Participant now discovered via both sources:', state.npub)
          existing.markDualDiscovery()
          this.updateParticipantsList()
        }
      } else {
        // Create new watcher
        console.log('[MoQ Subscriber] User joined voice (Nostr):', state.npub)
        this.createWatcher(connection, state.npub, 'nostr')
      }
    })

    console.log('[MoQ Subscriber] Active participants (Nostr):', activeNpubs.size, 'Total watchers:', this.watchers.size)

    // Remove watchers that are no longer active via Nostr
    for (const [npub, watcher] of this.watchers) {
      if (!activeNpubs.has(npub)) {
        if (watcher.discoverySource === 'both') {
          // Still announced via MoQ
          console.log('[MoQ Subscriber] Client left Nostr but still on MoQ:', npub)
          watcher.discoverySource = 'moq'
          this.updateParticipantsList()
        } else if (watcher.discoverySource === 'nostr') {
          // Only on Nostr, remove it
          console.log('[MoQ Subscriber] Client left voice (Nostr only):', npub)
          this.removeWatcher(npub)
        }
      }
    }
  }

  /**
   * Create watcher for a participant
   */
  private createWatcher(connection: Moq.Connection.Established, npub: string, source: 'nostr' | 'moq' = 'nostr'): void {
    try {
      console.log('[MoQ Subscriber] Creating watcher for:', npub, 'via', source)
      const watcher = new ParticipantWatcher(connection, npub, source)

      // Subscribe to speaking changes
      watcher.speaking.subscribe(() => {
        this.updateParticipantsList()
      })

      this.watchers.set(npub, watcher)
      console.log('[MoQ Subscriber] Watcher created successfully. Total watchers:', this.watchers.size)
      this.updateParticipantsList()
    } catch (err) {
      console.error('[MoQ Subscriber] Failed to create watcher for', npub, ':', err)
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
      console.log('[MoQ Subscriber] Watcher removed. Remaining watchers:', this.watchers.size)
      this.updateParticipantsList()
    }
  }

  /**
   * Update the participants signal with current state
   */
  private updateParticipantsList(): void {
    const participants = new Map<string, Participant>()

    for (const [npub, watcher] of this.watchers) {
      participants.set(npub, {
        npub,
        speaking: watcher.speaking.peek(),
        volume: watcher.volume.peek(),
        muted: watcher.muted.peek(),
        discoverySource: watcher.discoverySource,
      })
    }

    this.participants.set(participants)
  }

  /**
   * Set volume for a specific participant
   */
  setParticipantVolume(npub: string, volume: number): void {
    const watcher = this.watchers.get(npub)
    if (watcher) {
      watcher.setVolume(volume)
      this.updateParticipantsList()
    }
  }

  /**
   * Mute/unmute a specific participant
   */
  setParticipantMuted(npub: string, muted: boolean): void {
    const watcher = this.watchers.get(npub)
    if (watcher) {
      watcher.setMuted(muted)
      this.updateParticipantsList()
    }
  }

  /**
   * Stop listening and clean up all watchers
   */
  stopListening(): void {
    console.log('[MoQ Subscriber] Stopping participant listening...')

    // Stop announcement listener
    if (this.announcementAbortController) {
      this.announcementAbortController.abort()
      this.announcementAbortController = null
    }

    // Unsubscribe from avatar state
    if (this.unsubscribeAvatarState) {
      this.unsubscribeAvatarState()
      this.unsubscribeAvatarState = null
    }

    // Close all watchers
    const count = this.watchers.size
    for (const [, watcher] of this.watchers) {
      watcher.close()
    }
    this.watchers.clear()

    // Clear participants
    this.participants.set(new Map())

    console.log('[MoQ Subscriber] Stopped listening. Closed', count, 'watchers')
  }

  /**
   * Get participant count
   */
  getParticipantCount(): number {
    return this.watchers.size
  }

  /**
   * Get participants as array
   */
  getParticipants(): Participant[] {
    return Array.from(this.participants.peek().values())
  }

  /**
   * Clean up all resources
   */
  close(): void {
    this.stopListening()
    this.signals.close()
  }
}
