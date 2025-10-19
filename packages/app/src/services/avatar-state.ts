import { SimplePool, type Event } from 'nostr-tools'
import { npubEncode } from 'nostr-tools/nip19'
import { WORLD_RELAYS, getLiveChatATag, getAvatarStateDTag, AVATAR_STATE_CONFIG } from '../config'
import type { AccountManager } from 'applesauce-accounts'

export type AvatarType = 'voxel' | 'glb'
export type VoxelModelType = 'boy' | 'girl' | 'generated'
export type ColorMode = 'original' | 'random' | 'custom'
export type UserStatus = 'active' | 'idle' | 'away'
export type ActivityType = 'chatting' | 'exploring' | 'editing'

export interface Position {
  x: number
  y: number
  z: number
  // Rotation as quaternion [x, y, z, w]
  quaternion?: [number, number, number, number]
}

export interface AvatarConfig {
  avatarType: AvatarType
  avatarModel?: VoxelModelType  // voxel only
  avatarUrl?: string            // GLB only
  avatarColors?: ColorMode
  customColor?: string
}

export interface AvatarState {
  // Identity
  npub: string
  pubkey: string

  // Avatar configuration
  avatarType: AvatarType
  avatarModel?: VoxelModelType
  avatarUrl?: string
  avatarColors?: ColorMode
  customColor?: string

  // Client info
  clientName: string
  clientVersion?: string

  // Current state
  position: Position
  status: UserStatus
  voiceConnected: boolean
  micEnabled: boolean
  activities: ActivityType[]
  customMessage?: string

  // Metadata
  stateEventTimestamp: number
  lastUpdateTimestamp: number
}

interface StateEventData {
  event: Event
  parsed: Partial<AvatarState>
}

interface UpdateEventData {
  event: Event
  stateEventRef: string  // The 'a' tag pointing to state event
}

export class AvatarStateService {
  private pool: SimplePool
  private accountManager: AccountManager | null = null
  private liveActivityATag: string
  private avatarStateDTag: string

  // Current user's state
  private currentState: Partial<AvatarState> = {}
  private lastPositionPublishTime: number = 0
  private heartbeatInterval: NodeJS.Timeout | null = null

  // Other users' states
  private userStates: Map<string, AvatarState> = new Map()
  private stateEvents: Map<string, StateEventData> = new Map()
  private updateEvents: Map<string, UpdateEventData[]> = new Map()

  // Subscriptions
  private subscription: { close: () => void } | null = null
  private listeners: Set<(states: Map<string, AvatarState>) => void> = new Set()

  // Cleanup timers
  private userTimers: Map<string, NodeJS.Timeout> = new Map()

  constructor(accountManager?: AccountManager) {
    this.pool = new SimplePool()
    this.accountManager = accountManager || null
    this.liveActivityATag = getLiveChatATag()
    this.avatarStateDTag = getAvatarStateDTag()
  }

  /**
   * Get current states of all users
   */
  getUserStates(): Map<string, AvatarState> {
    return new Map(this.userStates)
  }

  /**
   * Subscribe to state changes
   */
  onChange(listener: (states: Map<string, AvatarState>) => void): () => void {
    this.listeners.add(listener)
    return () => {
      this.listeners.delete(listener)
    }
  }

  private notifyListeners(): void {
    const statesCopy = new Map(this.userStates)
    this.listeners.forEach(listener => listener(statesCopy))
  }

  /**
   * Start subscribing to avatar state and update events
   */
  startSubscription(): void {
    if (this.subscription) {
      console.warn('Avatar state subscription already active')
      return
    }

    const now = Math.floor(Date.now() / 1000)
    const since = now - AVATAR_STATE_CONFIG.SUBSCRIPTION_WINDOW_S

    // Subscribe to both state events (30317) and update events (1317)
    this.subscription = this.pool.subscribeMany(
      WORLD_RELAYS,
      {
        kinds: [AVATAR_STATE_CONFIG.STATE_EVENT_KIND, AVATAR_STATE_CONFIG.UPDATE_EVENT_KIND],
        '#a': [this.liveActivityATag],
        since,
      },
      {
        onevent: (event: Event) => {
          if (event.kind === AVATAR_STATE_CONFIG.STATE_EVENT_KIND) {
            this.handleStateEvent(event)
          } else if (event.kind === AVATAR_STATE_CONFIG.UPDATE_EVENT_KIND) {
            this.handleUpdateEvent(event)
          }
        },
        oneose: () => {
          console.log('Avatar state subscription established')
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

    // Clear all timers
    for (const timer of this.userTimers.values()) {
      clearTimeout(timer)
    }
    this.userTimers.clear()

    this.stateEvents.clear()
    this.updateEvents.clear()
    this.userStates.clear()
    this.notifyListeners()
  }

  /**
   * Handle incoming state event (kind 30317)
   */
  private handleStateEvent(event: Event): void {
    const pubkey = event.pubkey

    // Check if this is newer than existing state event
    const existing = this.stateEvents.get(pubkey)
    if (existing && existing.event.created_at >= event.created_at) {
      return // Ignore older event
    }

    // Parse state event
    const parsed = this.parseStateEvent(event)
    if (!parsed) return

    // Store state event
    this.stateEvents.set(pubkey, { event, parsed })

    // Clear old update events for this user
    this.updateEvents.set(pubkey, [])

    // Reconstruct and update user state
    this.reconstructUserState(pubkey)
    this.resetUserTimeout(pubkey)
  }

  /**
   * Handle incoming update event (kind 1317)
   */
  private handleUpdateEvent(event: Event): void {
    const pubkey = event.pubkey

    // Get the state event reference from 'a' tag
    const stateEventRef = this.getStateEventRef(event)
    if (!stateEventRef) {
      console.warn('Update event missing state event reference')
      return
    }

    // Check if we have the state event
    const stateEvent = this.stateEvents.get(pubkey)
    if (!stateEvent) {
      console.warn('Received update before state event for', pubkey)
      return
    }

    // Verify update links to current state event
    const currentStateRef = this.getStateEventRefFromStateEvent(stateEvent.event)
    if (stateEventRef !== currentStateRef) {
      console.log('Update event links to old state, ignoring')
      return
    }

    // Store update event
    const updates = this.updateEvents.get(pubkey) || []

    // Check if we already have this event
    if (updates.some(u => u.event.id === event.id)) {
      return
    }

    updates.push({ event, stateEventRef })
    this.updateEvents.set(pubkey, updates)

    // Reconstruct user state
    this.reconstructUserState(pubkey)
    this.resetUserTimeout(pubkey)
  }

  /**
   * Reconstruct user's current state from state event + updates
   */
  private reconstructUserState(pubkey: string): void {
    const stateEventData = this.stateEvents.get(pubkey)
    if (!stateEventData) return

    // Start with state event
    const state: AvatarState = {
      npub: npubEncode(pubkey),
      pubkey,
      avatarType: stateEventData.parsed.avatarType || 'voxel',
      avatarModel: stateEventData.parsed.avatarModel,
      avatarUrl: stateEventData.parsed.avatarUrl,
      avatarColors: stateEventData.parsed.avatarColors,
      customColor: stateEventData.parsed.customColor,
      clientName: stateEventData.parsed.clientName || 'Unknown',
      clientVersion: stateEventData.parsed.clientVersion,
      position: stateEventData.parsed.position || { x: 0, y: 0, z: 0 },
      status: stateEventData.parsed.status || 'active',
      voiceConnected: stateEventData.parsed.voiceConnected || false,
      micEnabled: stateEventData.parsed.micEnabled || false,
      activities: stateEventData.parsed.activities || [],
      customMessage: stateEventData.parsed.customMessage,
      stateEventTimestamp: stateEventData.event.created_at,
      lastUpdateTimestamp: stateEventData.event.created_at,
    }

    // Apply update events in chronological order
    const updates = this.updateEvents.get(pubkey) || []
    updates
      .sort((a, b) => a.event.created_at - b.event.created_at)
      .forEach(update => {
        this.applyUpdate(state, update.event)
      })

    this.userStates.set(pubkey, state)
    this.notifyListeners()
  }

  /**
   * Apply an update event to state
   */
  private applyUpdate(state: AvatarState, event: Event): void {
    const getTag = (name: string): string | undefined => {
      const tag = event.tags.find((t) => t[0] === name)
      return tag?.[1]
    }

    const getAllTags = (name: string): string[] => {
      return event.tags.filter((t) => t[0] === name).map((t) => t[1])
    }

    // Update position
    const positionStr = getTag('position')
    if (positionStr) {
      try {
        state.position = JSON.parse(positionStr)
      } catch (e) {
        console.error('Failed to parse position:', e)
      }
    }

    // Update status
    const status = getTag('status')
    if (status) {
      state.status = status as UserStatus
    }

    // Update activities
    const activities = getAllTags('activity')
    if (activities.length > 0) {
      state.activities = activities as ActivityType[]
    }

    // Update voice/mic
    const voice = getTag('voice')
    if (voice !== undefined) {
      state.voiceConnected = voice === 'connected'
    }

    const mic = getTag('mic')
    if (mic !== undefined) {
      state.micEnabled = mic === 'enabled'
    }

    // Update custom message
    if (event.content) {
      state.customMessage = event.content
    }

    state.lastUpdateTimestamp = event.created_at
  }

  /**
   * Parse state event into partial state
   */
  private parseStateEvent(event: Event): Partial<AvatarState> | null {
    try {
      const getTag = (name: string): string | undefined => {
        const tag = event.tags.find((t) => t[0] === name)
        return tag?.[1]
      }

      const getAllTags = (name: string): string[] => {
        return event.tags.filter((t) => t[0] === name).map((t) => t[1])
      }

      const avatarType = (getTag('avatar_type') || 'voxel') as AvatarType
      const avatarModel = getTag('avatar_model') as VoxelModelType | undefined
      const avatarUrl = getTag('avatar_url')
      const avatarColors = getTag('avatar_colors') as ColorMode | undefined
      const customColor = getTag('avatar_custom_color')

      const clientName = getTag('client') || 'Unknown'
      const clientVersion = getTag('client_version')

      const positionStr = getTag('position')
      let position: Position | undefined
      if (positionStr) {
        try {
          position = JSON.parse(positionStr)
        } catch (e) {
          console.error('Failed to parse position:', e)
        }
      }

      const status = (getTag('status') || 'active') as UserStatus
      const voice = getTag('voice')
      const mic = getTag('mic')
      const activities = getAllTags('activity') as ActivityType[]

      return {
        avatarType,
        avatarModel,
        avatarUrl,
        avatarColors,
        customColor,
        clientName,
        clientVersion,
        position,
        status,
        voiceConnected: voice === 'connected',
        micEnabled: mic === 'enabled',
        activities,
        customMessage: event.content || undefined,
      }
    } catch (err) {
      console.error('Failed to parse state event:', err)
      return null
    }
  }

  /**
   * Get state event reference from update event
   */
  private getStateEventRef(event: Event): string | null {
    // Look for 'a' tag pointing to state event (30317:pubkey:d-tag)
    const aTag = event.tags.find(t =>
      t[0] === 'a' &&
      t[1]?.startsWith(`${AVATAR_STATE_CONFIG.STATE_EVENT_KIND}:`)
    )
    return aTag?.[1] || null
  }

  /**
   * Generate state event reference from state event
   */
  private getStateEventRefFromStateEvent(event: Event): string {
    const dTag = event.tags.find(t => t[0] === 'd')?.[1] || this.avatarStateDTag
    return `${AVATAR_STATE_CONFIG.STATE_EVENT_KIND}:${event.pubkey}:${dTag}`
  }

  /**
   * Reset user timeout (mark as offline after TTL)
   */
  private resetUserTimeout(pubkey: string): void {
    const existing = this.userTimers.get(pubkey)
    if (existing) {
      clearTimeout(existing)
    }

    const timer = setTimeout(() => {
      this.userStates.delete(pubkey)
      this.stateEvents.delete(pubkey)
      this.updateEvents.delete(pubkey)
      this.userTimers.delete(pubkey)
      this.notifyListeners()
    }, AVATAR_STATE_CONFIG.STATE_TTL_S * 1000)

    this.userTimers.set(pubkey, timer)
  }

  /**
   * Publish avatar state event (kind 30317)
   */
  async publishStateEvent(
    avatarConfig: AvatarConfig,
    position: Position,
    status: UserStatus = 'active',
    voiceConnected: boolean = false,
    micEnabled: boolean = false,
    customMessage: string = ''
  ): Promise<void> {
    if (!this.accountManager?.active) {
      console.warn('No active account, skipping state publish')
      return
    }

    const account = this.accountManager.active
    const now = Math.floor(Date.now() / 1000)
    const expiry = now + AVATAR_STATE_CONFIG.EVENT_EXPIRY_S

    const tags: string[][] = [
      ['d', this.avatarStateDTag],
      ['a', this.liveActivityATag],
      ['expiration', expiry.toString()],

      // Avatar configuration
      ['avatar_type', avatarConfig.avatarType],

      // Client info
      ['client', 'Crossworld Web'],

      // Initial state
      ['position', JSON.stringify(position)],
      ['status', status],
      ['voice', voiceConnected ? 'connected' : 'disconnected'],
      ['mic', micEnabled ? 'enabled' : 'disabled'],
    ]

    // Add optional avatar fields
    if (avatarConfig.avatarModel) {
      tags.push(['avatar_model', avatarConfig.avatarModel])
    }
    if (avatarConfig.avatarUrl) {
      tags.push(['avatar_url', avatarConfig.avatarUrl])
    }
    if (avatarConfig.avatarColors) {
      tags.push(['avatar_colors', avatarConfig.avatarColors])
    }
    if (avatarConfig.customColor) {
      tags.push(['avatar_custom_color', avatarConfig.customColor])
    }

    const unsignedEvent = {
      kind: AVATAR_STATE_CONFIG.STATE_EVENT_KIND,
      created_at: now,
      tags,
      content: customMessage,
    }

    try {
      const signedEvent = await account.signEvent(unsignedEvent)
      await this.pool.publish(WORLD_RELAYS, signedEvent)

      // Update current state
      this.currentState = {
        ...avatarConfig,
        position,
        status,
        voiceConnected,
        micEnabled,
        customMessage,
      }
    } catch (err) {
      console.error('Failed to publish state event:', err)
      throw err
    }
  }

  /**
   * Publish state update event (kind 1317)
   */
  async publishUpdate(update: {
    position?: Position
    status?: UserStatus
    activities?: ActivityType[]
    voiceConnected?: boolean
    micEnabled?: boolean
    customMessage?: string
  }): Promise<void> {
    if (!this.accountManager?.active) {
      console.warn('No active account, skipping update publish')
      return
    }

    const account = this.accountManager.active
    const now = Math.floor(Date.now() / 1000)
    const expiry = now + AVATAR_STATE_CONFIG.EVENT_EXPIRY_S

    // Determine update type
    let updateType = 'status'
    if (update.position) updateType = 'position'
    else if (update.activities) updateType = 'activity'
    else if (update.voiceConnected !== undefined || update.micEnabled !== undefined) {
      updateType = 'voice'
    }

    const stateEventRef = `${AVATAR_STATE_CONFIG.STATE_EVENT_KIND}:${account.pubkey}:${this.avatarStateDTag}`

    const tags: string[][] = [
      ['a', stateEventRef],  // Link to state event
      ['a', this.liveActivityATag],  // Link to world
      ['update_type', updateType],
      ['expiration', expiry.toString()],
    ]

    // Add only provided fields
    if (update.position) {
      tags.push(['position', JSON.stringify(update.position)])
    }
    if (update.status) {
      tags.push(['status', update.status])
    }
    if (update.activities) {
      update.activities.forEach(activity => {
        tags.push(['activity', activity])
      })
    }
    if (update.voiceConnected !== undefined) {
      tags.push(['voice', update.voiceConnected ? 'connected' : 'disconnected'])
    }
    if (update.micEnabled !== undefined) {
      tags.push(['mic', update.micEnabled ? 'enabled' : 'disabled'])
    }

    const unsignedEvent = {
      kind: AVATAR_STATE_CONFIG.UPDATE_EVENT_KIND,
      created_at: now,
      tags,
      content: update.customMessage || '',
    }

    try {
      const signedEvent = await account.signEvent(unsignedEvent)
      await this.pool.publish(WORLD_RELAYS, signedEvent)

      // Update current state
      Object.assign(this.currentState, update)
    } catch (err) {
      console.error('Failed to publish update event:', err)
      throw err
    }
  }

  /**
   * Publish position update (with rate limiting)
   */
  async publishPosition(position: Position): Promise<void> {
    const now = Date.now()

    // Rate limit position updates
    if (now - this.lastPositionPublishTime < AVATAR_STATE_CONFIG.POSITION_UPDATE_MS) {
      return // Too soon
    }

    this.lastPositionPublishTime = now
    await this.publishUpdate({ position })
  }

  /**
   * Update avatar configuration and publish new state event
   * Preserves current position and other state
   */
  async updateAvatarConfig(avatarConfig: AvatarConfig): Promise<void> {
    // Use current position or default if not set
    const position = this.currentState.position ?? { x: 4, y: 0, z: 4 }
    const status = this.currentState.status ?? 'active'
    const voiceConnected = this.currentState.voiceConnected ?? false
    const micEnabled = this.currentState.micEnabled ?? false
    const customMessage = this.currentState.customMessage ?? ''

    await this.publishStateEvent(
      avatarConfig,
      position,
      status,
      voiceConnected,
      micEnabled,
      customMessage
    )
  }

  /**
   * Start periodic heartbeat
   */
  startHeartbeat(): void {
    if (this.heartbeatInterval) return

    this.heartbeatInterval = setInterval(() => {
      // Publish empty update to keep presence alive
      this.publishUpdate({}).catch(console.error)
    }, AVATAR_STATE_CONFIG.HEARTBEAT_INTERVAL_MS)
  }

  /**
   * Stop periodic heartbeat
   */
  stopHeartbeat(): void {
    if (this.heartbeatInterval) {
      clearInterval(this.heartbeatInterval)
      this.heartbeatInterval = null
    }
  }

  /**
   * Query user's last avatar state from relays
   * Used when logging in to restore previous state
   */
  async queryLastState(pubkey: string): Promise<Partial<AvatarState> | null> {
    try {
      const now = Math.floor(Date.now() / 1000)
      const since = now - AVATAR_STATE_CONFIG.SUBSCRIPTION_WINDOW_S

      // Query for state event
      const stateEvent = await this.pool.get(
        WORLD_RELAYS,
        {
          kinds: [AVATAR_STATE_CONFIG.STATE_EVENT_KIND],
          authors: [pubkey],
          '#a': [this.liveActivityATag],
          since,
        }
      )

      if (!stateEvent) {
        console.log('No previous state found for user')
        return null
      }

      // Check if state is too old (expired)
      const eventAge = now - stateEvent.created_at
      if (eventAge > AVATAR_STATE_CONFIG.EVENT_EXPIRY_S) {
        console.log('State event expired, too old to restore')
        return null
      }

      const parsed = this.parseStateEvent(stateEvent)
      if (!parsed) {
        return null
      }

      console.log('Found previous state:', parsed)
      return parsed
    } catch (err) {
      console.error('Failed to query last state:', err)
      return null
    }
  }

  /**
   * Clean up resources
   */
  destroy(): void {
    this.stopHeartbeat()
    this.stopSubscription()
    this.pool.close(WORLD_RELAYS)
  }
}
