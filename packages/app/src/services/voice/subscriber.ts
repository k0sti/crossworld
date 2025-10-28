import * as logger from '../../utils/logger';
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
 * Uses Watch.Broadcast to directly watch the user's path
 */
class ParticipantWatcher {
  public readonly npub: string
  public readonly sessionId: string
  private watcher: Hang.Watch.Broadcast
  private emitter: Hang.Watch.Audio.Emitter
  private signals = new Effect()

  // Public signals for UI
  public readonly speaking: Signal<boolean> = new Signal(false)
  public readonly volume: Signal<number> = new Signal(0.5)
  public readonly muted: Signal<boolean> = new Signal(false)
  public discoverySource: 'nostr' | 'moq' | 'both' = 'nostr'

  constructor(
    connection: Signal<Moq.Connection.Established | undefined>,
    npub: string,
    sessionId: string,
    source: 'nostr' | 'moq' = 'nostr'
  ) {
    this.npub = npub
    this.sessionId = sessionId
    this.discoverySource = source

    // Watch specific broadcast: crossworld/voice/{d-tag}/{npub}/{session}
    const broadcastPath = Moq.Path.from('crossworld', 'voice', LIVE_CHAT_D_TAG, npub, sessionId)
    logger.log('voice', '[MoQ Subscriber] Creating watcher for participant:', {
      npub,
      sessionId,
      broadcastPath: String(broadcastPath),
      source,
    })

    // Create broadcast watcher
    this.watcher = new Hang.Watch.Broadcast({
      connection,
      path: broadcastPath,
      enabled: true,
      audio: {
        enabled: true,
        latency: 100 as any, // 100ms jitter buffer
        speaking: {
          enabled: true,
        },
      },
    })

    // Create emitter to connect audio to speakers
    this.emitter = new Hang.Watch.Audio.Emitter(this.watcher.audio, {
      volume: this.volume,
      muted: this.muted,
      paused: new Signal(false),
    })

    // Subscribe to speaking state
    this.signals.effect((effect) => {
      const isSpeaking = effect.get(this.watcher.audio.speaking.active)
      if (isSpeaking !== undefined) {
        logger.log('voice', '[MoQ Subscriber] Speaking state for', this.npub, ':', isSpeaking)
      }
      this.speaking.set(isSpeaking ?? false)
    })

    logger.log('voice', '[MoQ Subscriber] Watcher created for:', npub)
  }

  /**
   * Mark this watcher as discovered via both sources
   */
  markDualDiscovery(): void {
    if (this.discoverySource !== 'both') {
      logger.log('voice', '[MoQ Subscriber] Participant now discovered via both Nostr and MoQ:', this.npub)
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
    logger.log('voice', '[MoQ Subscriber] Closing watcher for:', this.npub)
    this.signals.close()
    this.emitter.close()
    this.watcher.close()
  }
}

/**
 * Audio subscriber with dual discovery: Nostr AvatarStateService + MoQ announcements
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
    logger.log('voice', '[MoQ Subscriber] Own npub set:', npub)
  }

  /**
   * Start listening for participants via BOTH Nostr and MoQ announcements
   */
  async startListening(): Promise<void> {
    const conn = this.connection.getConnection()
    if (!conn) {
      throw new Error('Not connected to MoQ relay')
    }

    logger.log('voice', '[MoQ Subscriber] Starting DUAL discovery (Nostr + MoQ announcements)...')

    // 1. Start Nostr-based discovery (AvatarStateService)
    if (this.avatarStateService) {
      logger.log('voice', '[MoQ Subscriber] Starting Nostr-based discovery...')
      this.unsubscribeAvatarState = this.avatarStateService.onChange((states) => {
        logger.log('voice', '[MoQ Subscriber] Avatar states updated (Nostr), processing', states.size, 'users')
        this.handleClientListUpdate(conn, states)
      })
      logger.log('voice', '[MoQ Subscriber] Nostr discovery active')
    } else {
      logger.warn('voice', '[MoQ Subscriber] AvatarStateService not set - Nostr discovery disabled')
    }

    // 2. Start MoQ announcement-based discovery
    logger.log('voice', '[MoQ Subscriber] Starting MoQ announcement-based discovery...')
    this.startAnnouncementListener(conn)

    logger.log('voice', '[MoQ Subscriber] Now listening via BOTH discovery methods')
  }

  /**
   * Start listening for MoQ announcements
   * Listens for: crossworld/voice/{d-tag}/*
   */
  private async startAnnouncementListener(connection: Moq.Connection.Established): Promise<void> {
    // Cancel any existing listener
    if (this.announcementAbortController) {
      this.announcementAbortController.abort()
    }

    this.announcementAbortController = new AbortController()
    const signal = this.announcementAbortController.signal

    // Listen for all voice broadcasts: crossworld/voice/{d-tag}/*
    const prefix = Moq.Path.from('crossworld', 'voice', LIVE_CHAT_D_TAG)
    logger.log('voice', '[MoQ Subscriber] Listening for announcements with prefix:', String(prefix))

    const announced = connection.announced(prefix)

    // Start announcement loop
    const loop = (async () => {
      try {
        let count = 0
        for (;;) {
          if (signal.aborted) {
            logger.log('voice', '[MoQ Subscriber] Announcement listener aborted')
            break
          }

          const entry = await announced.next()
          if (!entry) {
            logger.log('voice', '[MoQ Subscriber] Announcement stream ended')
            break
          }

          count++
          this.announcementsReceived = count

          logger.log('voice', '[MoQ Subscriber] Announcement received:', {
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
          logger.error('voice', '[MoQ Subscriber] Announcement loop failed:', err)
        }
      }
    })()

    // Don't await - let it run in background
    loop.catch((err) => {
      if (!signal.aborted) {
        logger.error('voice', '[MoQ Subscriber] Announcement loop error:', err)
      }
    })
  }

  /**
   * Handle MoQ announcement added
   */
  private handleAnnouncementAdded(connection: Moq.Connection.Established, path: Moq.Path.Valid): void {
    // Extract npub and session from path: crossworld/voice/{d-tag}/{npub}/{session}
    const pathStr = String(path)
    const segments = pathStr.split('/')
    // Expecting: ['crossworld', 'voice', d-tag, npub, session]
    const npub = segments.length >= 4 ? segments[3] : null
    const sessionId = segments.length >= 5 ? segments[4] : null

    if (!npub || !sessionId || npub === this.ownNpub) {
      logger.log('voice', '[MoQ Subscriber] Skipping own broadcast or invalid path')
      return
    }

    logger.log('voice', '[MoQ Subscriber] Participant announced via MoQ:', npub, 'session:', sessionId)

    // Check if we already have a watcher
    const existing = this.watchers.get(npub)
    if (existing) {
      // Check if this is a new session (user toggled mic)
      if (existing.sessionId !== sessionId) {
        logger.log('voice', '[MoQ Subscriber] New session detected for', npub, '- replacing watcher')
        logger.log('voice', '[MoQ Subscriber] Old session:', existing.sessionId, '-> New session:', sessionId)

        // Remember the discovery source
        const source = existing.discoverySource === 'both' ? 'both' : 'moq'

        // Remove old watcher
        existing.close()
        this.watchers.delete(npub)

        // Create new watcher with new session
        this.createWatcher(connection, npub, sessionId, source === 'both' ? 'moq' : source)
      } else {
        logger.log('voice', '[MoQ Subscriber] Same session, marking as dual discovery')
        existing.markDualDiscovery()
        this.updateParticipantsList()
      }
    } else {
      // Create new watcher discovered via MoQ
      logger.log('voice', '[MoQ Subscriber] Creating new watcher from MoQ announcement')
      this.createWatcher(connection, npub, sessionId, 'moq')
    }
  }

  /**
   * Handle MoQ announcement removed
   */
  private handleAnnouncementRemoved(path: Moq.Path.Valid): void {
    const pathStr = String(path)
    const segments = pathStr.split('/')
    const npub = segments.length >= 4 ? segments[3] : null

    if (!npub) return

    logger.log('voice', '[MoQ Subscriber] Participant announcement ended (MoQ):', npub)

    // Don't immediately remove if also discovered via Nostr
    const watcher = this.watchers.get(npub)
    if (watcher && watcher.discoverySource === 'both') {
      logger.log('voice', '[MoQ Subscriber] Keeping watcher (still active via Nostr)')
      watcher.discoverySource = 'nostr'
      this.updateParticipantsList()
    } else if (watcher && watcher.discoverySource === 'moq') {
      // Only discovered via MoQ, remove it
      logger.log('voice', '[MoQ Subscriber] Removing watcher (only discovered via MoQ)')
      this.removeWatcher(npub)
    }
  }

  /**
   * Handle avatar state update from AvatarStateService (Nostr discovery)
   * Note: We don't have session ID from Nostr, so we can't create watchers here
   * Watchers will be created when MoQ announcements arrive
   */
  private handleClientListUpdate(
    _connection: Moq.Connection.Established,
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

      // Mark existing watchers as discovered via Nostr
      const existing = this.watchers.get(state.npub)
      if (existing && existing.discoverySource === 'moq') {
        logger.log('voice', '[MoQ Subscriber] Participant now discovered via both sources:', state.npub)
        existing.markDualDiscovery()
        this.updateParticipantsList()
      }
      // Note: We can't create watchers here because we don't have session ID
      // Watchers are created from MoQ announcements which include the session ID
    })

    logger.log('voice', '[MoQ Subscriber] Active participants (Nostr):', activeNpubs.size, 'Total watchers:', this.watchers.size)

    // Remove watchers that are no longer active via Nostr
    for (const [npub, watcher] of this.watchers) {
      if (!activeNpubs.has(npub)) {
        if (watcher.discoverySource === 'both') {
          // Still announced via MoQ
          logger.log('voice', '[MoQ Subscriber] Client left Nostr but still on MoQ:', npub)
          watcher.discoverySource = 'moq'
          this.updateParticipantsList()
        } else if (watcher.discoverySource === 'nostr') {
          // Only on Nostr, remove it
          logger.log('voice', '[MoQ Subscriber] Client left voice (Nostr only):', npub)
          this.removeWatcher(npub)
        }
      }
    }
  }

  /**
   * Create watcher for a participant
   */
  private createWatcher(_connection: Moq.Connection.Established, npub: string, sessionId: string, source: 'nostr' | 'moq' = 'nostr'): void {
    try {
      logger.log('voice', '[MoQ Subscriber] Creating watcher for:', npub, 'session:', sessionId, 'via', source)
      const watcher = new ParticipantWatcher(this.connection.established as Signal<Moq.Connection.Established | undefined>, npub, sessionId, source)

      // Subscribe to speaking changes
      watcher.speaking.subscribe(() => {
        this.updateParticipantsList()
      })

      this.watchers.set(npub, watcher)
      logger.log('voice', '[MoQ Subscriber] Watcher created successfully. Total watchers:', this.watchers.size)
      this.updateParticipantsList()
    } catch (err) {
      logger.error('voice', '[MoQ Subscriber] Failed to create watcher for', npub, ':', err)
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
      logger.log('voice', '[MoQ Subscriber] Watcher removed. Remaining watchers:', this.watchers.size)
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
    logger.log('voice', '[MoQ Subscriber] Stopping participant listening...')

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

    logger.log('voice', '[MoQ Subscriber] Stopped listening. Closed', count, 'watchers')
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
