import * as Hang from '@kixelated/hang'
import * as Moq from '@kixelated/moq'
import { Signal } from '@kixelated/signals'
import { MoqConnectionManager } from './connection'
import { LIVE_CHAT_D_TAG } from '../../config'

export class AudioPublisher {
  private broadcast: Hang.Publish.Broadcast | null = null
  private connection: MoqConnectionManager

  public micEnabled: Signal<boolean> = new Signal(false)
  public speaking: Signal<boolean> = new Signal(false)
  public error: Signal<string | null> = new Signal(null)

  constructor(connection: MoqConnectionManager) {
    this.connection = connection
  }

  async enableMic(npub: string): Promise<void> {
    if (this.broadcast) {
      console.log('Microphone already enabled')
      return
    }

    const conn = this.connection.getConnection()
    if (!conn) {
      throw new Error('Not connected to MoQ relay')
    }

    try {
      console.log('Requesting microphone permission...')

      // Request microphone access
      const stream = await navigator.mediaDevices.getUserMedia({
        audio: {
          echoCancellation: true,
          noiseSuppression: true,
          autoGainControl: true,
        }
      })

      // Get audio track from stream
      const audioTrack = stream.getAudioTracks()[0]
      if (!audioTrack) {
        throw new Error('No audio track found in media stream')
      }

      // Ensure it's an audio track
      if (audioTrack.kind !== 'audio') {
        throw new Error('Track is not an audio track')
      }

      // Create broadcast path: crossworld/voice/<d-tag>/<npub>
      const broadcastName = `crossworld/voice/${LIVE_CHAT_D_TAG}/${npub}`

      console.log('Creating audio broadcast:', broadcastName)

      // Create Hang broadcast
      // Note: TypeScript can't properly narrow MediaStreamTrack.kind type
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      this.broadcast = new Hang.Publish.Broadcast({
        connection: conn,
        path: Moq.Path.from(broadcastName),
        audio: {
          enabled: true,
          source: audioTrack as any,
        },
      })

      // Subscribe to speaking detection
      this.broadcast.audio.speaking.active.subscribe((isSpeaking) => {
        this.speaking.set(isSpeaking)
      })

      this.micEnabled.set(true)
      this.error.set(null)
      console.log('Microphone enabled and broadcasting')
    } catch (err) {
      console.error('Failed to enable microphone:', err)
      this.error.set(err instanceof Error ? err.message : 'Microphone access failed')
      throw err
    }
  }

  async disableMic(): Promise<void> {
    if (!this.broadcast) {
      return
    }

    try {
      console.log('Disabling microphone...')

      // Stop the broadcast
      this.broadcast.audio.enabled.set(false)

      // Clean up
      this.broadcast = null
      this.micEnabled.set(false)
      this.speaking.set(false)
      this.error.set(null)

      console.log('Microphone disabled')
    } catch (err) {
      console.error('Error disabling microphone:', err)
    }
  }

  isMicEnabled(): boolean {
    return this.micEnabled.peek()
  }

  isSpeaking(): boolean {
    return this.speaking.peek()
  }
}
