import { Signal } from '@kixelated/signals'
import { MoqConnectionManager, moqConnection } from './connection'
import { AudioPublisher } from './publisher'
import { AudioSubscriber, type Participant } from './subscriber'

export type VoiceStatus = 'disconnected' | 'connecting' | 'connected' | 'error'

export class VoiceManager {
  private connection: MoqConnectionManager
  private publisher: AudioPublisher
  private subscriber: AudioSubscriber
  private currentNpub: string | null = null

  public status: Signal<VoiceStatus> = new Signal('disconnected')
  public micEnabled: Signal<boolean> = new Signal(false)
  public speaking: Signal<boolean> = new Signal(false)
  public participants: Signal<Map<string, Participant>> = new Signal(new Map())
  public error: Signal<string | null> = new Signal(null)

  constructor() {
    this.connection = moqConnection
    this.publisher = new AudioPublisher(this.connection)
    this.subscriber = new AudioSubscriber(this.connection)

    // Subscribe to publisher state
    this.publisher.micEnabled.subscribe((enabled) => {
      this.micEnabled.set(enabled)
    })

    this.publisher.speaking.subscribe((speaking) => {
      this.speaking.set(speaking)
    })

    // Subscribe to participant updates
    this.subscriber.participants.subscribe((participants) => {
      this.participants.set(participants)
    })

    // Subscribe to connection status
    this.connection.status.subscribe((status) => {
      if (status === 'disconnected') {
        this.status.set('disconnected')
      } else if (status === 'connecting') {
        this.status.set('connecting')
      } else if (status === 'connected') {
        this.status.set('connected')
      } else if (status === 'error') {
        this.status.set('error')
      }
    })

    // Subscribe to errors
    this.publisher.error.subscribe((err) => {
      if (err) {
        this.error.set(err)
      }
    })

    this.subscriber.error.subscribe((err) => {
      if (err) {
        this.error.set(err)
      }
    })
  }

  async connect(streamingUrl: string, npub: string): Promise<void> {
    if (this.status.peek() === 'connected') {
      console.log('Already connected to voice')
      return
    }

    this.currentNpub = npub
    this.error.set(null)

    try {
      // Connect to MoQ relay
      await this.connection.connect(streamingUrl)

      // Set own npub so subscriber can filter it out
      this.subscriber.setOwnNpub(npub)

      // Start listening for participants
      await this.subscriber.startListening()

      this.status.set('connected')
      console.log('Voice chat connected')
    } catch (err) {
      console.error('Failed to connect to voice:', err)
      this.status.set('error')
      this.error.set(err instanceof Error ? err.message : 'Connection failed')
      throw err
    }
  }

  async disconnect(): Promise<void> {
    if (this.status.peek() === 'disconnected') {
      return
    }

    try {
      // Disable mic if enabled
      if (this.publisher.isMicEnabled()) {
        await this.publisher.disableMic()
      }

      // Stop listening
      this.subscriber.stopListening()

      // Disconnect from MoQ
      await this.connection.disconnect()

      this.status.set('disconnected')
      this.error.set(null)
      this.currentNpub = null
      console.log('Voice chat disconnected')
    } catch (err) {
      console.error('Error disconnecting from voice:', err)
    }
  }

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

  isConnected(): boolean {
    return this.status.peek() === 'connected'
  }

  isMicEnabled(): boolean {
    return this.micEnabled.peek()
  }

  isSpeaking(): boolean {
    return this.speaking.peek()
  }

  getParticipants(): Participant[] {
    return Array.from(this.participants.peek().values())
  }

  getParticipantCount(): number {
    return this.participants.peek().size
  }
}

// Global singleton instance
export const voiceManager = new VoiceManager()
