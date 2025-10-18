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
  private discoveryActive = false
  private discoveryAbort: AbortController | null = null
  private ownNpub: string | null = null

  public participants: Signal<Map<string, Participant>> = new Signal(new Map())
  public error: Signal<string | null> = new Signal(null)

  constructor(connection: MoqConnectionManager) {
    this.connection = connection
  }

  setOwnNpub(npub: string): void {
    this.ownNpub = npub
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
      console.log('Setting up participant discovery for:', pathPrefix)
      const announced = connection.announced(Moq.Path.from(pathPrefix))

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

        // Extract npub from path (path is Moq.Path.Valid which is string-like)
        const pathStr = String(path)
        const npub = pathStr.slice(pathPrefix.length)

        if (!npub) {
          continue
        }

        // Skip our own broadcast
        if (this.ownNpub && npub === this.ownNpub) {
          continue
        }

        if (this.watchers.has(npub)) {
          continue
        }

        console.log('Discovered participant:', npub)
        this.subscribeToParticipant(connection, pathStr, npub)
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
    try {
      console.log('Subscribing to participant:', npub)

      // Create watcher for this participant
      const watcher = new Hang.Watch.Broadcast({
        connection,
        path: Moq.Path.from(broadcastName),
        enabled: true, // CRITICAL: Must enable watcher for it to consume the broadcast
        audio: { enabled: true },
      })

      // Connect audio to output when available
      watcher.audio.root.subscribe((audioNode) => {
        if (audioNode) {
          // Get the AudioContext from the watcher (created by Hang library)
          const audioContext = watcher.audio.context.peek()

          if (!audioContext) {
            console.error('No AudioContext available for participant:', npub)
            return
          }

          try {
            // Connect the audio node to the destination of its own AudioContext
            audioNode.connect(audioContext.destination)
            console.log('Audio connected for participant:', npub)
          } catch (err) {
            console.error('Failed to connect audio for participant:', npub, err)
          }
        }
      })

      // Subscribe to speaking state
      watcher.audio.speaking.active.subscribe((isSpeaking) => {
        const speaking = isSpeaking ?? false
        this.updateParticipant(npub, speaking)
      })

      this.watchers.set(npub, watcher)
      this.updateParticipant(npub, false)
    } catch (err) {
      console.error('Failed to subscribe to participant:', npub, err)
    }
  }

  private updateParticipant(npub: string, speaking: boolean): void {
    const participants = new Map(this.participants.peek())
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
        watcher.close()
      } catch (err) {
        console.error(`Error closing watcher for ${npub}:`, err)
      }
    }
    this.watchers.clear()

    // Clear participants
    this.participants.set(new Map())

    console.log('Stopped listening')
  }

  getParticipantCount(): number {
    return this.participants.peek().size
  }

  getParticipants(): Participant[] {
    return Array.from(this.participants.peek().values())
  }
}
