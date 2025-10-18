import * as Moq from '@kixelated/moq'
import { Effect, Signal, type Getter } from '@kixelated/signals'

export type ConnectionStatus = 'disconnected' | 'connecting' | 'connected'

/**
 * MoQ connection manager with automatic reconnection
 * Based on ref/moq/js/hang/src/publish/element.ts:175-178
 */
export class MoqConnectionManager {
  // Input signals
  private urlSignal: Signal<URL | undefined> = new Signal(undefined)
  private enabledSignal: Signal<boolean> = new Signal(false)

  // Connection with auto-reconnect
  private connection: Moq.Connection.Reload

  // Expose status directly from Connection.Reload
  public readonly status: Getter<ConnectionStatus>
  public readonly established: Getter<Moq.Connection.Established | undefined>

  private signals = new Effect()

  constructor() {
    // Create auto-reconnecting connection (ref: ref/moq/js/hang/src/publish/element.ts:175-178)
    this.connection = new Moq.Connection.Reload({
      enabled: this.enabledSignal,
      url: this.urlSignal,
    })

    // Expose connection status
    this.status = this.connection.status
    this.established = this.connection.established
  }

  /**
   * Connect to MoQ relay (or change URL)
   * Connection.Reload handles reconnection automatically
   */
  connect(url: string): void {
    console.log('Connecting to MoQ relay:', url)
    this.urlSignal.set(new URL(url))
    this.enabledSignal.set(true)
  }

  /**
   * Disconnect from MoQ relay
   */
  disconnect(): void {
    console.log('Disconnecting from MoQ relay')
    this.enabledSignal.set(false)
    this.urlSignal.set(undefined)
  }

  /**
   * Get the established connection (if connected)
   */
  getConnection(): Moq.Connection.Established | undefined {
    return this.connection.established.peek()
  }

  /**
   * Check if currently connected
   */
  isConnected(): boolean {
    return this.connection.status.peek() === 'connected'
  }

  /**
   * Clean up resources
   */
  close(): void {
    this.signals.close()
    this.connection.close()
  }
}

// Global singleton instance
export const moqConnection = new MoqConnectionManager()
