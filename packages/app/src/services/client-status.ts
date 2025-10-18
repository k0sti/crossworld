import { SimplePool, type Event } from 'nostr-tools'
import { npubEncode } from 'nostr-tools/nip19'
import { WORLD_RELAYS, getLiveChatATag } from '../config'
import type { AccountManager } from 'applesauce-accounts'

export interface ClientStatus {
  /** User's npub (for display purposes) */
  npub: string
  /** User's hex pubkey (unique identifier) */
  pubkey: string
  /** Current status of the client */
  status: 'active' | 'idle' | 'away'
  /** Name of the client application */
  clientName: string
  /** Version of the client application */
  clientVersion?: string
  /** 3D position in the world */
  position?: { x: number; y: number; z: number }
  /** Whether user is connected to voice chat */
  voiceConnected?: boolean
  /** Whether user's microphone is enabled */
  micEnabled?: boolean
  /** Activity badges */
  isChatting?: boolean
  isExploring?: boolean
  isEditing?: boolean
  /** Custom status message */
  customMessage?: string
  /** Timestamp of last update (unix timestamp in seconds) */
  lastUpdate: number
}

export class ClientStatusService {
  private pool: SimplePool
  private accountManager: AccountManager | null = null
  private updateInterval: NodeJS.Timeout | null = null
  private publishDebounceTimeout: NodeJS.Timeout | null = null
  private currentStatus: Partial<ClientStatus> = {}
  private liveActivityATag: string
  private lastPublishTime: number = 0
  private clients: Map<string, ClientStatus> = new Map()
  private clientTimers: Map<string, NodeJS.Timeout> = new Map()
  private subscription: { close: () => void } | null = null
  private listeners: Set<(clients: Map<string, ClientStatus>) => void> = new Set()

  constructor(accountManager?: AccountManager) {
    this.pool = new SimplePool()
    this.accountManager = accountManager || null
    this.liveActivityATag = getLiveChatATag()
  }

  /**
   * Get current clients map
   */
  getClients(): Map<string, ClientStatus> {
    return new Map(this.clients)
  }

  /**
   * Subscribe to client changes
   */
  onChange(listener: (clients: Map<string, ClientStatus>) => void): () => void {
    this.listeners.add(listener)
    return () => {
      this.listeners.delete(listener)
    }
  }

  private notifyListeners(): void {
    const clientsCopy = new Map(this.clients)
    this.listeners.forEach(listener => listener(clientsCopy))
  }

  /**
   * Start subscribing to client statuses
   * Single subscription shared across the app
   */
  startSubscription(): void {
    if (this.subscription) {
      console.warn('Client status subscription already active')
      return
    }

    this.subscription = this.pool.subscribeMany(
      WORLD_RELAYS,
      {
        kinds: [30315], // NIP-38 User Statuses
        '#a': [this.liveActivityATag],
        since: Math.floor(Date.now() / 1000) - 300, // Only get events from last 5 minutes
      },
      {
        onevent: (event: Event) => {
          // Check if event is recent (within last 5 minutes)
          const now = Math.floor(Date.now() / 1000)
          const eventAge = now - event.created_at

          if (eventAge > 300) {
            // Event is older than 5 minutes, ignore it
            return
          }

          const client = this.parseClientStatus(event)
          if (client) {
            this.clients.set(client.pubkey, client)
            this.resetClientTimeout(client.pubkey)
            this.notifyListeners()
          }
        },
        oneose: () => {
          // Subscription established
        },
      }
    )
  }

  /**
   * Stop subscription
   */
  stopSubscription(): void {
    if (this.subscription) {
      this.subscription.close()
      this.subscription = null
    }
    // Clear all timeouts
    for (const timer of this.clientTimers.values()) {
      clearTimeout(timer)
    }
    this.clientTimers.clear()
    this.clients.clear()
    this.notifyListeners()
  }

  private resetClientTimeout(pubkey: string): void {
    const existing = this.clientTimers.get(pubkey)
    if (existing) {
      clearTimeout(existing)
    }

    // Remove client if no update after 5 minutes (300 seconds)
    const timer = setTimeout(() => {
      this.clients.delete(pubkey)
      this.clientTimers.delete(pubkey)
      this.notifyListeners()
    }, 300000)

    this.clientTimers.set(pubkey, timer)
  }

  /**
   * Publish own client status
   * @param status - Status update to publish
   * @param immediate - If true, bypasses debounce (use for user UI actions)
   */
  async publishStatus(status: Partial<ClientStatus>, immediate: boolean = false): Promise<void> {
    if (!this.accountManager) {
      console.warn('AccountManager not set, skipping status publish')
      return
    }

    const account = this.accountManager.active
    if (!account) {
      console.warn('No active account, skipping status publish')
      return
    }

    // Merge with current status
    this.currentStatus = { ...this.currentStatus, ...status }

    // Publish immediately if requested (for UI actions)
    if (immediate) {
      await this.publishStatusImmediate()
      return
    }

    // Debounce rapid updates (don't publish more than once every 5 seconds)
    const now = Date.now()
    if (now - this.lastPublishTime < 5000) {
      // Clear any pending debounce
      if (this.publishDebounceTimeout) {
        clearTimeout(this.publishDebounceTimeout)
      }

      // Schedule publish after debounce period
      this.publishDebounceTimeout = setTimeout(() => {
        this.publishStatusImmediate().catch(console.error)
      }, 5000 - (now - this.lastPublishTime))
      return
    }

    await this.publishStatusImmediate()
  }

  /**
   * Immediately publish status (internal method)
   */
  private async publishStatusImmediate(): Promise<void> {
    if (!this.accountManager) {
      return
    }

    const account = this.accountManager.active
    if (!account) {
      return
    }

    this.lastPublishTime = Date.now()

    const tags: string[][] = [
      ['d', `crossworld-status-${this.liveActivityATag}`],
      ['a', this.liveActivityATag],
      ['status', this.currentStatus.status || 'active'],
      ['client', this.currentStatus.clientName || 'Crossworld Web'],
    ]

    if (this.currentStatus.clientVersion) {
      tags.push(['client_version', this.currentStatus.clientVersion])
    }

    if (this.currentStatus.position) {
      tags.push(['position', JSON.stringify(this.currentStatus.position)])
    }

    if (this.currentStatus.voiceConnected !== undefined) {
      tags.push(['voice', this.currentStatus.voiceConnected ? 'connected' : 'disconnected'])
    }

    if (this.currentStatus.micEnabled !== undefined) {
      tags.push(['mic', this.currentStatus.micEnabled ? 'enabled' : 'disabled'])
    }

    // Activity badges
    if (this.currentStatus.isChatting) {
      tags.push(['activity', 'chatting'])
    }
    if (this.currentStatus.isExploring) {
      tags.push(['activity', 'exploring'])
    }
    if (this.currentStatus.isEditing) {
      tags.push(['activity', 'editing'])
    }

    const unsignedEvent = {
      kind: 30315,
      created_at: Math.floor(Date.now() / 1000),
      tags,
      content: this.currentStatus.customMessage || '',
    }

    try {
      const signedEvent = await account.signEvent(unsignedEvent)
      await this.pool.publish(WORLD_RELAYS, signedEvent)
    } catch (err) {
      console.error('Failed to publish client status:', err)
      throw err
    }
  }

  /**
   * Start periodic status updates (every 60 seconds)
   */
  startStatusUpdates(status: Partial<ClientStatus>): void {
    // Publish immediately
    this.publishStatus(status).catch(console.error)

    // Then publish every 60 seconds
    this.updateInterval = setInterval(() => {
      this.publishStatus({}).catch(console.error)
    }, 60000)
  }

  /**
   * Stop periodic status updates
   */
  stopStatusUpdates(): void {
    if (this.updateInterval) {
      clearInterval(this.updateInterval)
      this.updateInterval = null
    }
    if (this.publishDebounceTimeout) {
      clearTimeout(this.publishDebounceTimeout)
      this.publishDebounceTimeout = null
    }
    this.currentStatus = {}
  }

  /**
   * Parse a Nostr event into a ClientStatus object
   */
  private parseClientStatus(event: Event): ClientStatus | null {
    try {
      const getTag = (name: string): string | undefined => {
        const tag = event.tags.find((t) => t[0] === name)
        return tag?.[1]
      }

      const getAllTags = (name: string): string[] => {
        return event.tags.filter((t) => t[0] === name).map((t) => t[1])
      }

      const dTag = getTag('d')
      if (!dTag?.startsWith('crossworld-status-')) {
        return null
      }

      const status = (getTag('status') || 'active') as ClientStatus['status']
      const clientName = getTag('client') || 'Unknown Client'
      const clientVersion = getTag('client_version')
      const positionStr = getTag('position')
      const voiceStr = getTag('voice')
      const micStr = getTag('mic')
      const activities = getAllTags('activity')

      let position: { x: number; y: number; z: number } | undefined
      if (positionStr) {
        try {
          position = JSON.parse(positionStr)
        } catch {
          // Invalid position JSON
        }
      }

      return {
        npub: npubEncode(event.pubkey),
        pubkey: event.pubkey,
        status,
        clientName,
        clientVersion,
        position,
        voiceConnected: voiceStr === 'connected',
        micEnabled: micStr === 'enabled',
        isChatting: activities.includes('chatting'),
        isExploring: activities.includes('exploring'),
        isEditing: activities.includes('editing'),
        customMessage: event.content || undefined,
        lastUpdate: event.created_at,
      }
    } catch (err) {
      console.error('Failed to parse client status:', err)
      return null
    }
  }

  /**
   * Clean up resources
   */
  destroy(): void {
    this.stopStatusUpdates()
    this.stopSubscription()
    this.pool.close(WORLD_RELAYS)
  }
}
