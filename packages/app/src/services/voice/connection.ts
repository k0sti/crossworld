import * as Moq from '@kixelated/moq'
import { Signal } from '@kixelated/signals'

export type ConnectionStatus = 'disconnected' | 'connecting' | 'connected' | 'error'

export class MoqConnectionManager {
  private connection: Moq.Connection.Established | null = null
  private relayUrl: string | null = null

  public status: Signal<ConnectionStatus> = new Signal('disconnected')
  public error: Signal<string | null> = new Signal(null)

  async connect(url: string): Promise<void> {
    if (this.connection && this.relayUrl === url) {
      console.log('Already connected to', url)
      return
    }

    if (this.connection) {
      await this.disconnect()
    }

    this.status.set('connecting')
    this.error.set(null)
    this.relayUrl = url

    try {
      console.log('Connecting to MoQ relay:', url)
      this.connection = await Moq.Connection.connect(new URL(url))
      this.status.set('connected')
      console.log('Connected to MoQ relay')
    } catch (err) {
      console.error('Failed to connect to MoQ relay:', err)
      this.status.set('error')
      this.error.set(err instanceof Error ? err.message : 'Connection failed')
      this.connection = null
      this.relayUrl = null
      throw err
    }
  }

  async disconnect(): Promise<void> {
    if (this.connection) {
      try {
        await this.connection.close()
      } catch (err) {
        console.error('Error closing MoQ connection:', err)
      }
      this.connection = null
      this.relayUrl = null
    }
    this.status.set('disconnected')
    this.error.set(null)
  }

  getConnection(): Moq.Connection.Established | null {
    return this.connection
  }

  isConnected(): boolean {
    return this.status.peek() === 'connected' && this.connection !== null
  }
}

// Global singleton instance
export const moqConnection = new MoqConnectionManager()
