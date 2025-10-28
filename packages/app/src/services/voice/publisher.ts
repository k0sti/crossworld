import * as logger from '../../utils/logger';
import * as Hang from '@kixelated/hang'
import * as Moq from '@kixelated/moq'
import { Effect, Signal } from '@kixelated/signals'
import { MoqConnectionManager } from './connection'
import { LIVE_CHAT_D_TAG } from '../../config'

/**
 * Audio publisher using hang library patterns
 * Based on ref/moq/js/hang/src/publish/element.ts
 */
export class AudioPublisher {
  private connection: MoqConnectionManager
  private signals = new Effect()

  // Input signals
  private enabledSignal: Signal<boolean> = new Signal(false)
  private npubSignal: Signal<string | undefined> = new Signal(undefined)

  // Microphone source (similar to ref/moq/js/hang/src/publish/element.ts:240-244)
  private microphoneSource: Signal<MediaStreamTrack | undefined> = new Signal(undefined)

  // Output signals
  public readonly micEnabled: Signal<boolean> = new Signal(false)
  public readonly speaking: Signal<boolean> = new Signal(false)
  public readonly error: Signal<string | null> = new Signal(null)

  constructor(connection: MoqConnectionManager) {
    this.connection = connection

    // Setup reactive broadcast creation (ref: ref/moq/js/hang/src/publish/element.ts:180-199)
    this.signals.effect(this.#runBroadcast.bind(this))

    // Setup microphone acquisition (ref: ref/moq/js/hang/src/publish/element.ts:230-252)
    this.signals.effect(this.#runMicrophone.bind(this))
  }

  /**
   * Enable microphone and start broadcasting
   */
  async enableMic(npub: string): Promise<void> {
    if (this.enabledSignal.peek()) {
      logger.log('voice', '[MoQ Publisher] Microphone already enabled')
      return
    }

    logger.log('voice', '[MoQ Publisher] Enabling microphone for:', npub)
    this.npubSignal.set(npub)
    this.enabledSignal.set(true)
  }

  /**
   * Disable microphone and stop broadcasting
   */
  async disableMic(): Promise<void> {
    if (!this.enabledSignal.peek()) {
      return
    }

    logger.log('voice', '[MoQ Publisher] Disabling microphone')
    this.enabledSignal.set(false)
  }

  /**
   * Acquire microphone and set up audio source
   * Ref: ref/moq/js/hang/src/publish/source/microphone.ts
   */
  #runMicrophone(effect: Effect): void {
    const enabled = effect.get(this.enabledSignal)

    if (!enabled) {
      this.microphoneSource.set(undefined)
      this.micEnabled.set(false)
      return
    }

    // Spawn async microphone acquisition
    effect.spawn(async () => {
      try {
        logger.log('voice', '[MoQ Publisher] Requesting microphone permission...')

        // Request microphone access (ref: ref/moq/js/hang/src/publish/source/microphone.ts:43-56)
        const stream = await navigator.mediaDevices.getUserMedia({
          audio: {
            echoCancellation: true,
            noiseSuppression: true,
            autoGainControl: true,
          },
        })

        // Check if effect was cancelled while waiting
        const cancelled = await Promise.race([
          Promise.resolve(false),
          effect.cancel.then(() => true),
        ])
        if (cancelled) {
          logger.log('voice', '[MoQ Publisher] Effect cancelled, stopping stream')
          stream.getTracks().forEach(track => track.stop())
          return
        }

        const track = stream.getAudioTracks()[0]
        if (!track) {
          throw new Error('No audio track found')
        }

        logger.log('voice', '[MoQ Publisher] Microphone acquired:', {
          label: track.label,
          enabled: track.enabled,
          readyState: track.readyState,
        })

        // Set the track - this will trigger broadcast creation
        this.microphoneSource.set(track)
        this.micEnabled.set(true)
        this.error.set(null)

        // Cleanup: stop track when effect is cancelled
        effect.cleanup(() => {
          logger.log('voice', '[MoQ Publisher] Effect cleanup - stopping microphone track')
          track.stop()
        })
      } catch (err) {
        logger.error('voice', '[MoQ Publisher] Failed to acquire microphone:', err)
        this.error.set(err instanceof Error ? err.message : 'Microphone access failed')
        this.enabledSignal.set(false)
      }
    })
  }

  /**
   * Create and manage broadcast
   * Ref: ref/moq/js/hang/src/publish/broadcast.ts
   * Path structure: crossworld/voice/{d-tag}/{npub}/{session}
   */
  #runBroadcast(effect: Effect): void {
    const enabled = effect.get(this.enabledSignal)
    if (!enabled) return

    const conn = effect.get(this.connection.established)
    if (!conn) return

    const npub = effect.get(this.npubSignal)
    if (!npub) return

    const audioSource = effect.get(this.microphoneSource)
    if (!audioSource) return

    // Generate random session ID (allows multiple tabs/connections)
    const sessionId = Math.random().toString(36).slice(2, 8)

    // Create broadcast path: crossworld/voice/{d-tag}/{npub}/{session}
    const path = Moq.Path.from('crossworld', 'voice', LIVE_CHAT_D_TAG, npub, sessionId)
    logger.log('voice', '[MoQ Publisher] Creating broadcast:', {
      path: String(path),
      npub,
      dTag: LIVE_CHAT_D_TAG,
      sessionId,
    })

    // Create broadcast (ref: ref/moq/js/hang/src/publish/element.ts:180-199)
    const broadcast = new Hang.Publish.Broadcast({
      connection: conn,
      path,
      enabled: true, // Must be enabled for announcement
      audio: {
        enabled: true,
        source: audioSource as Hang.Publish.Audio.AudioStreamTrack,
        speaking: {
          enabled: true, // Enable speaking detection
        },
      },
    })

    logger.log('voice', '[MoQ Publisher] Broadcast created, waiting for announcement...')

    // Subscribe to speaking state
    effect.effect((innerEffect) => {
      const isSpeaking = innerEffect.get(broadcast.audio.speaking.active)
      logger.log('voice', '[MoQ Publisher] Speaking state:', isSpeaking)
      this.speaking.set(isSpeaking)
    })

    logger.log('voice', '[MoQ Publisher] Now publishing audio to relay')

    // Cleanup: close broadcast when effect ends
    effect.cleanup(() => {
      logger.log('voice', '[MoQ Publisher] Closing broadcast:', String(path))
      broadcast.close()
    })
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
   * Clean up all resources
   */
  close(): void {
    this.signals.close()
  }
}
