import { Effect, Signal } from '@kixelated/signals'
import { MoqConnectionManager, moqConnection } from './connection'
import { AudioPublisher } from './publisher'
import { AudioSubscriber, type Participant } from './subscriber'
import type { ClientStatusService } from '../client-status'

export type VoiceStatus = 'disconnected' | 'connecting' | 'connected'

/**
 * Voice manager coordinating connection, publishing, and subscribing
 * Uses Effect system for reactive state management
 */
export class VoiceManager {
  private connection: MoqConnectionManager
  private publisher: AudioPublisher
  private subscriber: AudioSubscriber
  private signals = new Effect()

  // State
  private currentNpub: string | null = null

  // Output signals
  public readonly status: Signal<VoiceStatus> = new Signal('disconnected')
  public readonly micEnabled: Signal<boolean> = new Signal(false)
  public readonly speaking: Signal<boolean> = new Signal(false)
  public readonly participants: Signal<Map<string, Participant>> = new Signal(new Map())
  public readonly error: Signal<string | null> = new Signal(null)

  constructor() {
    this.connection = moqConnection
    this.publisher = new AudioPublisher(this.connection)
    this.subscriber = new AudioSubscriber(this.connection)

    // Reactive status forwarding using Effect system
    this.signals.effect((effect) => {
      const connStatus = effect.get(this.connection.status)
      this.status.set(connStatus)
    })

    // Forward publisher state
    this.signals.effect((effect) => {
      const enabled = effect.get(this.publisher.micEnabled)
      this.micEnabled.set(enabled)
    })

    this.signals.effect((effect) => {
      const isSpeaking = effect.get(this.publisher.speaking)
      this.speaking.set(isSpeaking)
    })

    this.signals.effect((effect) => {
      const err = effect.get(this.publisher.error)
      if (err) {
        this.error.set(err)
      }
    })

    // Forward subscriber state
    this.signals.effect((effect) => {
      const parts = effect.get(this.subscriber.participants)
      this.participants.set(parts)
    })
  }

  /**
   * Set the client status service for participant discovery
   */
  setClientStatusService(service: ClientStatusService): void {
    this.subscriber.setClientStatusService(service)
  }

  /**
   * Connect to voice chat
   */
  async connect(streamingUrl: string, npub: string): Promise<void> {
    if (this.status.peek() === 'connected') {
      console.log('Already connected to voice')
      return
    }

    console.log('Connecting to voice chat...')
    this.currentNpub = npub
    this.error.set(null)

    try {
      // Connect to MoQ relay (Connection.Reload handles reconnection)
      this.connection.connect(streamingUrl)

      // Wait for connection to establish
      // Poll until connected or error
      const maxWaitTime = 10000 // 10 seconds
      const startTime = Date.now()

      while (Date.now() - startTime < maxWaitTime) {
        const status = this.connection.status.peek()
        if (status === 'connected') {
          break
        }
        // Wait a bit before checking again
        await new Promise(resolve => setTimeout(resolve, 100))
      }

      const finalStatus = this.connection.status.peek()
      if (finalStatus !== 'connected') {
        throw new Error('Connection timeout')
      }

      // Set own npub for subscriber
      this.subscriber.setOwnNpub(npub)

      // Start listening for participants
      await this.subscriber.startListening()

      console.log('Voice chat connected')
    } catch (err) {
      console.error('Failed to connect to voice:', err)
      this.error.set(err instanceof Error ? err.message : 'Connection failed')
      throw err
    }
  }

  /**
   * Disconnect from voice chat
   */
  async disconnect(): Promise<void> {
    if (this.status.peek() === 'disconnected') {
      return
    }

    console.log('Disconnecting from voice chat...')

    try {
      // Disable mic if enabled
      if (this.publisher.isMicEnabled()) {
        await this.publisher.disableMic()
      }

      // Stop subscriber
      this.subscriber.stopListening()

      // Disconnect from MoQ
      this.connection.disconnect()

      this.error.set(null)
      this.currentNpub = null

      console.log('Voice chat disconnected')
    } catch (err) {
      console.error('Error disconnecting from voice:', err)
    }
  }

  /**
   * Toggle microphone on/off
   */
  async toggleMic(): Promise<void> {
    if (!this.currentNpub) {
      throw new Error('Not connected to voice')
    }

    if (this.publisher.isMicEnabled()) {
      await this.publisher.disableMic()
    } else {
      await this.publisher.enableMic(this.currentNpub)
    }
  }

  /**
   * Set volume for a participant (0.0 - 1.0)
   */
  setParticipantVolume(npub: string, volume: number): void {
    this.subscriber.setParticipantVolume(npub, volume)
  }

  /**
   * Mute/unmute a participant
   */
  setParticipantMuted(npub: string, muted: boolean): void {
    this.subscriber.setParticipantMuted(npub, muted)
  }

  /**
   * Check if connected to voice
   */
  isConnected(): boolean {
    return this.status.peek() === 'connected'
  }

  /**
   * Check if microphone is enabled
   */
  isMicEnabled(): boolean {
    return this.micEnabled.peek()
  }

  /**
   * Check if currently speaking
   */
  isSpeaking(): boolean {
    return this.speaking.peek()
  }

  /**
   * Get participants as array
   */
  getParticipants(): Participant[] {
    return Array.from(this.participants.peek().values())
  }

  /**
   * Get participant count
   */
  getParticipantCount(): number {
    return this.participants.peek().size
  }

  /**
   * Clean up all resources
   */
  close(): void {
    this.signals.close()
    this.publisher.close()
    this.subscriber.close()
    this.connection.close()
  }
}

// Global singleton instance
export const voiceManager = new VoiceManager()
