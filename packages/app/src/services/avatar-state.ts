import * as logger from '../utils/logger';
import { SimplePool, type Event } from 'nostr-tools'
import { npubEncode } from 'nostr-tools/nip19'
import { WORLD_RELAYS, getLiveChatATag, getAvatarStateDTag, AVATAR_STATE_CONFIG } from '../config'
import type { AccountManager } from 'applesauce-accounts'

export type AvatarType = 'vox' | 'glb' | 'csm'
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
  avatarId?: string             // ID that can be used to load existing model from disk (e.g., 'boy', 'girl', 'man')
  avatarUrl?: string            // Load avatar from URL
  avatarData?: string           // Preferred way, generate model based on data (not yet implemented)
  avatarMod?: string            // Custom modification applied to avatar after load (not yet implemented)
  avatarTexture?: string        // Texture name to apply to avatar (0 = only colors, or texture name like 'grass', 'stone')
  csmCode?: string              // CSM (Cube Script Model) code for procedurally generated voxel models
}

export interface AvatarState {
  // Identity
  npub: string
  pubkey: string

  // Avatar configuration
  avatarType: AvatarType
  avatarId?: string
  avatarUrl?: string
  avatarData?: string
  avatarMod?: string
  avatarTexture?: string

  // Client info
  clientName: string
  clientVersion?: string

  // Current state
  position: Position
  moveStyle?: string  // walk, run, teleport:fade, teleport:spin, etc.
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

  // Batch notification flag to prevent excessive updates during initial load
  private batchingNotifications: boolean = false

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
    // Skip notification if we're batching (during initial load)
    if (this.batchingNotifications) return

    const statesCopy = new Map(this.userStates)
    this.listeners.forEach(listener => listener(statesCopy))
  }

  /**
   * Start subscribing to avatar state and update events
   *
   * Strategy:
   * 1. First fetch all existing state events (30317) without 'since' to get current users
   * 2. Then subscribe to live updates (both 30317 and 1317) with since=now
   *
   * This ensures we see all active users regardless of when they last published
   */
  async startSubscription(): Promise<void> {
    if (this.subscription) {
      logger.warn('service', 'Avatar state subscription already active')
      return
    }

    // Check if world relays are configured
    if (!WORLD_RELAYS || WORLD_RELAYS.length === 0) {
      logger.warn('service', '[AvatarState] No world relays configured, skipping subscription')
      return
    }

    const now = Math.floor(Date.now() / 1000)
    const recentSince = now - AVATAR_STATE_CONFIG.SUBSCRIPTION_WINDOW_S

    // Enable batching to prevent multiple listener notifications during initial load
    this.batchingNotifications = true

    try {
      // Step 1: Query existing state events (30317) without 'since'
      // This gets all current avatar states regardless of age
      logger.log('service', '[AvatarState] Fetching existing state events...')
      const existingStates = await this.pool.querySync(
        WORLD_RELAYS,
        {
          kinds: [AVATAR_STATE_CONFIG.STATE_EVENT_KIND],
          '#a': [this.liveActivityATag],
          // No 'since' - we want all current state events
        }
      )

      // Process existing state events
      existingStates.forEach(event => {
        this.handleStateEvent(event)
      })

      logger.log('service', `[AvatarState] Loaded ${existingStates.length} existing state events`)
    } catch (error) {
      logger.warn('service', '[AvatarState] Failed to fetch existing states (relay may be unavailable):', error)
      // Continue - relay might come online later
    }

    try {
      // Step 2: Query recent update events (1317) from the last hour
      logger.log('service', '[AvatarState] Fetching recent update events...')
      const recentUpdates = await this.pool.querySync(
        WORLD_RELAYS,
        {
          kinds: [AVATAR_STATE_CONFIG.UPDATE_EVENT_KIND],
          '#a': [this.liveActivityATag],
          since: recentSince,
        }
      )

      // Process recent update events
      recentUpdates.forEach(event => {
        this.handleUpdateEvent(event)
      })

      logger.log('service', `[AvatarState] Loaded ${recentUpdates.length} recent update events`)
    } catch (error) {
      logger.warn('service', '[AvatarState] Failed to fetch recent updates (relay may be unavailable):', error)
      // Continue - relay might come online later
    }

    // Disable batching and send a single notification with all loaded states
    this.batchingNotifications = false
    this.notifyListeners()

    try {
      // Step 3: Subscribe to live updates (both state and update events) from now onwards
      this.subscription = this.pool.subscribeMany(
        WORLD_RELAYS,
        {
          kinds: [AVATAR_STATE_CONFIG.STATE_EVENT_KIND, AVATAR_STATE_CONFIG.UPDATE_EVENT_KIND],
          '#a': [this.liveActivityATag],
          since: now, // Only new events from now on
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
            logger.log('service', '[AvatarState] Live subscription established')
          },
        }
      )
    } catch (error) {
      logger.warn('service', '[AvatarState] Failed to subscribe to live updates (relay may be unavailable):', error)
      // App will work in offline mode without multi-user features
    }
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
    if (!parsed) {
      logger.warn('service', `[AvatarState] Skipping invalid state event from ${pubkey.slice(0, 8)}...`)
      return
    }

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
      logger.warn('service', 'Update event missing state event reference')
      return
    }

    // Check if we have the state event
    const stateEvent = this.stateEvents.get(pubkey)
    if (!stateEvent) {
      // Update received before state event - silently ignore
      return
    }

    // Discard update events older than state event
    // State event is canonical at its timestamp, older updates are obsolete
    if (event.created_at < stateEvent.event.created_at) {
      return // Update is obsolete, state event supersedes it
    }

    // Verify update links to current state event
    const currentStateRef = this.getStateEventRefFromStateEvent(stateEvent.event)
    if (stateEventRef !== currentStateRef) {
      return // Update links to old state event
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
      avatarType: stateEventData.parsed.avatarType || 'vox',
      avatarId: stateEventData.parsed.avatarId,
      avatarUrl: stateEventData.parsed.avatarUrl,
      avatarData: stateEventData.parsed.avatarData,
      avatarMod: stateEventData.parsed.avatarMod,
      avatarTexture: stateEventData.parsed.avatarTexture,
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
    // Filter out updates older than state event (state is canonical at its timestamp)
    const updates = this.updateEvents.get(pubkey) || []
    const stateTimestamp = stateEventData.event.created_at

    let shouldRemove = false
    updates
      .filter(update => update.event.created_at >= stateTimestamp)
      .sort((a, b) => a.event.created_at - b.event.created_at)
      .forEach(update => {
        const removed = this.applyUpdate(state, update.event)
        if (removed) shouldRemove = true
      })

    // If user was removed (status=away), don't re-add them
    if (shouldRemove) {
      this.notifyListeners()
      return
    }

    this.userStates.set(pubkey, state)
    this.notifyListeners()
  }

  /**
   * Apply an update event to state
   * Returns true if user should be removed (status=away)
   */
  private applyUpdate(state: AvatarState, event: Event): boolean {
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
        logger.warn('service', '[AvatarState] Failed to parse position in update event, keeping previous value')
        // Don't update position if parsing fails
      }
    }

    // Update move style
    const moveStyle = getTag('move_style')
    if (moveStyle) {
      state.moveStyle = moveStyle
    }

    // Update status
    const status = getTag('status')
    if (status) {
      state.status = status as UserStatus

      // If status is 'away', user has logged out - remove them completely
      if (status === 'away') {
        const pubkey = state.pubkey
        logger.log('service', `[AvatarState] User ${state.npub} went away, removing from state`)
        this.userStates.delete(pubkey)
        this.stateEvents.delete(pubkey)
        this.updateEvents.delete(pubkey)
        const timer = this.userTimers.get(pubkey)
        if (timer) {
          clearTimeout(timer)
          this.userTimers.delete(pubkey)
        }
        return true // Signal that user was removed
      }
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
    return false // User not removed
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

      const avatarType = (getTag('avatar_type') || 'vox') as AvatarType
      const avatarId = getTag('avatar_id')
      const avatarUrl = getTag('avatar_url')
      const avatarData = getTag('avatar_data')
      const avatarMod = getTag('avatar_mod')
      const avatarTexture = getTag('avatar_texture')

      logger.log('service', '[AvatarState] Parsed state event:', { avatarType, avatarId, avatarUrl, avatarDataLength: avatarData?.length, avatarTexture, pubkey: event.pubkey.slice(0, 8) })

      const clientName = getTag('client') || 'Unknown'
      const clientVersion = getTag('client_version')

      const positionStr = getTag('position')
      let position: Position | undefined
      if (positionStr) {
        try {
          position = JSON.parse(positionStr)
        } catch (e) {
          logger.warn('service', '[AvatarState] Failed to parse position JSON, skipping event:', positionStr.slice(0, 50))
          return null
        }
      }

      const status = (getTag('status') || 'active') as UserStatus
      const voice = getTag('voice')
      const mic = getTag('mic')
      const activities = getAllTags('activity') as ActivityType[]

      return {
        avatarType,
        avatarId,
        avatarUrl,
        avatarData,
        avatarMod,
        avatarTexture,
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
      logger.error('service', 'Failed to parse state event:', err)
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
      logger.warn('service', 'No active account, skipping state publish')
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
    if (avatarConfig.avatarId) {
      tags.push(['avatar_id', avatarConfig.avatarId])
    }
    if (avatarConfig.avatarUrl) {
      tags.push(['avatar_url', avatarConfig.avatarUrl])
    }
    if (avatarConfig.avatarData) {
      tags.push(['avatar_data', avatarConfig.avatarData])
    }
    if (avatarConfig.avatarMod) {
      tags.push(['avatar_mod', avatarConfig.avatarMod])
    }
    if (avatarConfig.avatarTexture) {
      tags.push(['avatar_texture', avatarConfig.avatarTexture])
    }

    const unsignedEvent = {
      kind: AVATAR_STATE_CONFIG.STATE_EVENT_KIND,
      created_at: now,
      tags,
      content: customMessage,
    }

    // Check if world relays are configured
    if (!WORLD_RELAYS || WORLD_RELAYS.length === 0) {
      logger.warn('service', '[AvatarState] No world relays configured, state event not published')
      // Update local state even if we can't publish
      this.currentState = {
        ...avatarConfig,
        position,
        status,
        voiceConnected,
        micEnabled,
        customMessage,
      }
      return
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
      logger.warn('service', '[AvatarState] Failed to publish state event (relay may be unavailable):', err)
      // Update local state even if we can't publish
      this.currentState = {
        ...avatarConfig,
        position,
        status,
        voiceConnected,
        micEnabled,
        customMessage,
      }
      // Don't throw - allow app to continue working offline
    }
  }

  /**
   * Publish state update event (kind 1317)
   */
  async publishUpdate(update: {
    position?: Position
    moveStyle?: string
    status?: UserStatus
    activities?: ActivityType[]
    voiceConnected?: boolean
    micEnabled?: boolean
    customMessage?: string
  }): Promise<void> {
    if (!this.accountManager?.active) {
      logger.warn('service', 'No active account, skipping update publish')
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
    if (update.moveStyle) {
      tags.push(['move_style', update.moveStyle])
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

    // Check if world relays are configured
    if (!WORLD_RELAYS || WORLD_RELAYS.length === 0) {
      // Update local state even if we can't publish
      Object.assign(this.currentState, update)
      return
    }

    try {
      const signedEvent = await account.signEvent(unsignedEvent)
      await this.pool.publish(WORLD_RELAYS, signedEvent)

      // Update current state
      Object.assign(this.currentState, update)
    } catch (err) {
      logger.warn('service', '[AvatarState] Failed to publish update event (relay may be unavailable):', err)
      // Update local state even if we can't publish
      Object.assign(this.currentState, update)
      // Don't throw - allow app to continue working offline
    }
  }

  /**
   * Publish position update (with rate limiting)
   */
  async publishPosition(position: Position, moveStyle?: string): Promise<void> {
    const now = Date.now()

    // Rate limit position updates
    if (now - this.lastPositionPublishTime < AVATAR_STATE_CONFIG.POSITION_UPDATE_MS) {
      return // Too soon
    }

    this.lastPositionPublishTime = now
    await this.publishUpdate({ position, moveStyle })
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
        logger.log('service', 'No previous state found for user')
        return null
      }

      // Check if state is too old (expired)
      const eventAge = now - stateEvent.created_at
      if (eventAge > AVATAR_STATE_CONFIG.EVENT_EXPIRY_S) {
        logger.log('service', 'State event expired, too old to restore')
        return null
      }

      const parsed = this.parseStateEvent(stateEvent)
      if (!parsed) {
        return null
      }

      logger.log('service', 'Found previous state:', parsed)
      return parsed
    } catch (err) {
      logger.error('service', 'Failed to query last state:', err)
      return null
    }
  }

  /**
   * Remove a specific user's state (used when logging out to prevent showing own avatar as remote)
   */
  removeUserState(pubkey: string): void {
    const existing = this.userTimers.get(pubkey)
    if (existing) {
      clearTimeout(existing)
      this.userTimers.delete(pubkey)
    }

    this.userStates.delete(pubkey)
    this.stateEvents.delete(pubkey)
    this.updateEvents.delete(pubkey)
    this.notifyListeners()
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
