import * as Hang from '@kixelated/hang'
import * as Moq from '@kixelated/moq'
import { Signal } from '@kixelated/signals'
import { MoqConnectionManager } from './connection'
import { LIVE_CHAT_D_TAG } from '../../config'

export interface Participant {
  npub: string
  speaking: boolean
  lastUpdate: number
}

export class AudioSubscriber {
  private connection: MoqConnectionManager
  private watchers = new Map<string, Hang.Watch.Broadcast>()
  private audioContext: AudioContext | null = null
  private discoveryActive = false
  private discoveryAbort: AbortController | null = null

  public participants: Signal<Map<string, Participant>> = new Signal(new Map())
  public error: Signal<string | null> = new Signal(null)

  constructor(connection: MoqConnectionManager) {
    this.connection = connection
  }

  async startListening(): Promise<void> {
    if (this.discoveryActive) {
      console.log('Already listening for participants')
      return
    }

    const conn = this.connection.getConnection()
    if (!conn) {
      throw new Error('Not connected to MoQ relay')
    }

    // Create audio context for playback
    if (!this.audioContext) {
      this.audioContext = new AudioContext()
    }

    console.log('Starting participant discovery...')
    this.discoveryActive = true
    this.discoveryAbort = new AbortController()

    // Start discovery loop
    this.discoverParticipants(conn, this.discoveryAbort.signal)
  }

  private async discoverParticipants(
    connection: Moq.Connection.Established,
    signal: AbortSignal
  ): Promise<void> {
    const pathPrefix = `crossworld/voice/${LIVE_CHAT_D_TAG}/`

    try {
      // Get the announced stream with our path prefix
      const announced = connection.announced(pathPrefix)

      while (!signal.aborted) {
        const entry = await announced.next()

        if (!entry || signal.aborted) {
          break
        }

        const path = entry.path

        // Check if this broadcast is active
        if (!entry.active) {
          continue
        }

        // Extract npub from path
        const npub = path.slice(pathPrefix.length)

        if (!npub || this.watchers.has(npub)) {
          continue
        }

        console.log('Discovered participant:', npub)
        this.subscribeToParticipant(connection, path, npub)
      }
    } catch (err) {
      if (!signal.aborted) {
        console.error('Participant discovery error:', err)
        this.error.set(err instanceof Error ? err.message : 'Discovery failed')
      }
    }
  }

  private subscribeToParticipant(
    connection: Moq.Connection.Established,
    broadcastName: string,
    npub: string
  ): void {
    if (!this.audioContext) {
      console.error('Audio context not initialized')
      return
    }

    try {
      console.log('Subscribing to participant:', npub)

      // Create watcher for this participant
      const watcher = new Hang.Watch.Broadcast({
        connection,
        path: broadcastName,
        audio: { enabled: true },
      })

      // Create audio emitter for playback
      const emitter = new Hang.Watch.Audio.Emitter(this.audioContext)
      emitter.input.set(watcher.audio.output)

      // Subscribe to speaking state
      watcher.audio.speaking.active.subscribe((isSpeaking) => {
        this.updateParticipant(npub, isSpeaking)
      })

      this.watchers.set(npub, watcher)
      this.updateParticipant(npub, false)
    } catch (err) {
      console.error(`Failed to subscribe to participant ${npub}:`, err)
    }
  }

  private updateParticipant(npub: string, speaking: boolean): void {
    const participants = new Map(this.participants.value)
    participants.set(npub, {
      npub,
      speaking,
      lastUpdate: Date.now(),
    })
    this.participants.set(participants)
  }

  stopListening(): void {
    if (!this.discoveryActive) {
      return
    }

    console.log('Stopping participant discovery...')

    // Stop discovery
    if (this.discoveryAbort) {
      this.discoveryAbort.abort()
      this.discoveryAbort = null
    }
    this.discoveryActive = false

    // Close all watchers
    for (const [npub, watcher] of this.watchers) {
      try {
        watcher.audio.enabled.set(false)
      } catch (err) {
        console.error(`Error closing watcher for ${npub}:`, err)
      }
    }
    this.watchers.clear()

    // Clear participants
    this.participants.set(new Map())

    // Close audio context
    if (this.audioContext) {
      this.audioContext.close()
      this.audioContext = null
    }

    console.log('Stopped listening')
  }

  getParticipantCount(): number {
    return this.participants.value.size
  }

  getParticipants(): Participant[] {
    return Array.from(this.participants.value.values())
  }
}
