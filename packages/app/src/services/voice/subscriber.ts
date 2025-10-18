import * as Hang from '@kixelated/hang'
import * as Moq from '@kixelated/moq'
import { Signal } from '@kixelated/signals'
import { MoqConnectionManager } from './connection'
import { LIVE_CHAT_D_TAG } from '../../config'
import { ClientStatusService, type ClientStatus } from '../client-status'

export interface Participant {
  npub: string
  speaking: boolean
  lastUpdate: number
}

export type ConnectionStatus = 'disconnected' | 'connecting' | 'connected' | 'error'

export class AudioSubscriber {
  private connection: MoqConnectionManager
  private watchers = new Map<string, Hang.Watch.Broadcast>()
  private connectionStatuses = new Map<string, ConnectionStatus>()
  private discoveryActive = false
  private discoveryAbort: AbortController | null = null
  private ownNpub: string | null = null
  private clientStatusService: ClientStatusService | null = null
  private unsubscribeClientStatus: (() => void) | null = null

  public participants: Signal<Map<string, Participant>> = new Signal(new Map())
  public error: Signal<string | null> = new Signal(null)

  constructor(connection: MoqConnectionManager) {
    this.connection = connection
  }

  setClientStatusService(service: ClientStatusService): void {
    this.clientStatusService = service
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

    console.log('Starting participant discovery via client status...')
    this.discoveryActive = true

    // Use client status service for discovery
    if (this.clientStatusService) {
      this.unsubscribeClientStatus = this.clientStatusService.onChange((clients) => {
        // Handle each client
        clients.forEach((client) => {
          this.handleClientStatusUpdate(conn, client)
        })

        // Remove clients that are no longer in the map
        const currentParticipants = this.participants.peek()
        currentParticipants.forEach((_, npub: string) => {
          // Find if this npub is still voice connected
          let stillConnected = false
          clients.forEach((client) => {
            if (client.npub === npub && client.voiceConnected) {
              stillConnected = true
            }
          })
          if (!stillConnected) {
            this.handleClientRemoved(npub)
          }
        })
      })
    } else {
      console.warn('No ClientStatusService set, falling back to MoQ discovery')
      this.discoveryAbort = new AbortController()
      this.discoverParticipants(conn, this.discoveryAbort.signal)
    }
  }

  private handleClientStatusUpdate(
    connection: Moq.Connection.Established,
    client: ClientStatus
  ): void {
    // Only subscribe to clients who are in voice chat
    if (!client.voiceConnected) {
      // If they're not in voice but we're watching them, stop
      if (this.watchers.has(client.npub)) {
        console.log('Client left voice chat:', client.npub)
        this.unsubscribeFromParticipant(client.npub)
      }
      return
    }

    // Skip our own broadcast
    if (this.ownNpub && client.npub === this.ownNpub) {
      return
    }

    // Subscribe if not already watching
    if (!this.watchers.has(client.npub)) {
      console.log('Client joined voice chat:', client.npub)
      const pathPrefix = `crossworld/voice/${LIVE_CHAT_D_TAG}/`
      const broadcastName = pathPrefix + client.npub
      this.subscribeToParticipant(connection, broadcastName, client.npub)
    }
  }

  private handleClientRemoved(pubkey: string): void {
    // Convert pubkey to npub to match our watchers map
    // For now, we need to find the npub in our participants
    const participants = this.participants.peek()
    for (const [npub] of participants) {
      // Note: We're storing npub in participants, so we need to match it
      // This is a simplification - in production you'd want proper pubkey/npub conversion
      if (npub.includes(pubkey.slice(0, 8))) {
        console.log('Client removed from world:', npub)
        this.unsubscribeFromParticipant(npub)
        break
      }
    }
  }

  private unsubscribeFromParticipant(npub: string): void {
    const watcher = this.watchers.get(npub)
    if (watcher) {
      try {
        watcher.close()
      } catch (err) {
        console.error(`Error closing watcher for ${npub}:`, err)
      }
      this.watchers.delete(npub)
    }

    // Remove connection status
    this.connectionStatuses.delete(npub)

    // Remove from participants
    const participants = new Map(this.participants.peek())
    participants.delete(npub)
    this.participants.set(participants)
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
      console.log('Announcement iterator created, waiting for broadcasts...')

      while (!signal.aborted) {
        console.log('Waiting for next announcement...')
        const entry = await announced.next()
        console.log('Received announcement entry:', entry)

        if (!entry || signal.aborted) {
          console.log('Discovery loop ended')
          break
        }

        const path = entry.path

        // Check if this broadcast is active
        if (!entry.active) {
          console.log('Skipping inactive broadcast:', String(path))
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
          console.log('Skipping own broadcast:', npub)
          continue
        }

        if (this.watchers.has(npub)) {
          console.log('Already watching participant:', npub)
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

      // Set status to connecting
      this.connectionStatuses.set(npub, 'connecting')

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
            this.connectionStatuses.set(npub, 'error')
            return
          }

          try {
            // Connect the audio node to the destination of its own AudioContext
            audioNode.connect(audioContext.destination)
            console.log('Audio connected for participant:', npub)
            this.connectionStatuses.set(npub, 'connected')
          } catch (err) {
            console.error('Failed to connect audio for participant:', npub, err)
            this.connectionStatuses.set(npub, 'error')
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
      this.connectionStatuses.set(npub, 'error')
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

    // Stop client status subscription
    if (this.unsubscribeClientStatus) {
      this.unsubscribeClientStatus()
      this.unsubscribeClientStatus = null
    }

    // Stop MoQ discovery if active
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

  getConnectionStatus(npub: string): ConnectionStatus {
    return this.connectionStatuses.get(npub) || 'disconnected'
  }
}
