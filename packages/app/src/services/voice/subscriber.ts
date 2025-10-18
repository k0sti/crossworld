import * as Hang from '@kixelated/hang'
import * as Moq from '@kixelated/moq'
import { Effect, Signal } from '@kixelated/signals'
import { MoqConnectionManager } from './connection'
import { LIVE_CHAT_D_TAG } from '../../config'
import { ClientStatusService, type ClientStatus } from '../client-status'

export interface Participant {
  npub: string
  speaking: boolean
  volume: number
  muted: boolean
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

  constructor(connection: Moq.Connection.Established, npub: string) {
    this.npub = npub

    const path = Moq.Path.from(`crossworld/voice/${LIVE_CHAT_D_TAG}/${npub}`)
    console.log('Creating watcher for participant:', npub, 'path:', String(path))

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
      this.speaking.set(isSpeaking ?? false)
    })

    console.log('Watcher created for:', npub)
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
    console.log('Closing watcher for:', this.npub)
    this.signals.close()
    this.emitter.close()
    this.watcher.close()
  }
}

/**
 * Audio subscriber that watches participants from client list
 * Discovery based on ClientStatusService, playback based on hang library
 */
export class AudioSubscriber {
  private connection: MoqConnectionManager
  private watchers = new Map<string, ParticipantWatcher>()
  private ownNpub: string | null = null
  private clientStatusService: ClientStatusService | null = null
  private unsubscribeClientStatus: (() => void) | null = null

  public participants: Signal<Map<string, Participant>> = new Signal(new Map())

  constructor(connection: MoqConnectionManager) {
    this.connection = connection
  }

  /**
   * Set the client status service for participant discovery
   */
  setClientStatusService(service: ClientStatusService): void {
    this.clientStatusService = service
  }

  /**
   * Set our own npub to avoid watching ourselves
   */
  setOwnNpub(npub: string): void {
    this.ownNpub = npub
  }

  /**
   * Start listening for participants via client list
   */
  async startListening(): Promise<void> {
    const conn = this.connection.getConnection()
    if (!conn) {
      throw new Error('Not connected to MoQ relay')
    }

    if (!this.clientStatusService) {
      throw new Error('ClientStatusService not set - required for participant discovery')
    }

    console.log('Starting participant discovery via client status...')

    // Subscribe to client status changes
    this.unsubscribeClientStatus = this.clientStatusService.onChange((clients) => {
      this.handleClientListUpdate(conn, clients)
    })

    console.log('Listening for participants')
  }

  /**
   * Handle client list update from ClientStatusService
   */
  private handleClientListUpdate(
    connection: Moq.Connection.Established,
    clients: Map<string, ClientStatus>
  ): void {
    // Track which npubs should be active
    const activeNpubs = new Set<string>()

    // Process each client
    clients.forEach((client) => {
      // Skip clients not in voice chat
      if (!client.voiceConnected) {
        return
      }

      // Skip ourselves
      if (this.ownNpub && client.npub === this.ownNpub) {
        return
      }

      activeNpubs.add(client.npub)

      // Create watcher if not already watching
      if (!this.watchers.has(client.npub)) {
        console.log('Client joined voice:', client.npub)
        this.createWatcher(connection, client.npub)
      }
    })

    // Remove watchers for clients no longer in voice
    for (const [npub] of this.watchers) {
      if (!activeNpubs.has(npub)) {
        console.log('Client left voice:', npub)
        this.removeWatcher(npub)
      }
    }

    // Update participants signal
    this.updateParticipantsList()
  }

  /**
   * Create watcher for a participant
   */
  private createWatcher(connection: Moq.Connection.Established, npub: string): void {
    try {
      const watcher = new ParticipantWatcher(connection, npub)

      // Subscribe to speaking changes
      watcher.speaking.subscribe(() => {
        this.updateParticipantsList()
      })

      this.watchers.set(npub, watcher)
      this.updateParticipantsList()
    } catch (err) {
      console.error('Failed to create watcher for', npub, err)
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
    console.log('Stopping participant listening...')

    // Unsubscribe from client status
    if (this.unsubscribeClientStatus) {
      this.unsubscribeClientStatus()
      this.unsubscribeClientStatus = null
    }

    // Close all watchers
    for (const [, watcher] of this.watchers) {
      watcher.close()
    }
    this.watchers.clear()

    // Clear participants
    this.participants.set(new Map())

    console.log('Stopped listening')
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
  }
}
