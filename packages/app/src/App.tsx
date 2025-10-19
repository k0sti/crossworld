import { useState, useMemo, useEffect, useRef } from 'react'
import { ChakraProvider, useToast } from '@chakra-ui/react'
import { AccountsProvider } from 'applesauce-react/providers'
import { AccountManager } from 'applesauce-accounts'
import { TopBar } from './components/TopBar'
import { WorldCanvas, type VoxelModelType } from './components/WorldCanvas'
import { LeftSidebarPanel } from './components/LeftSidebarPanel'
import { ConfigPanelType } from './components/ConfigPanel'
import { NetworkConfigPanel } from './components/NetworkConfigPanel'
import { ProfilePanel } from './components/ProfilePanel'
import { AvatarPanel } from './components/AvatarPanel'
import { ChatPanel } from './components/ChatPanel'
import { ClientListPanel } from './components/ClientListPanel'
import { RestoreStateModal } from './components/RestoreStateModal'
import { fetchLiveEvent } from './services/live-event'
import { AvatarStateService, type AvatarConfig, type AvatarState } from './services/avatar-state'
import { useVoice } from './hooks/useVoice'
import { npubEncode } from 'nostr-tools/nip19'
import type { TeleportAnimationType } from './renderer/teleport-animation'

function App() {
  const [pubkey, setPubkey] = useState<string | null>(null)
  const [useVoxelAvatar, setUseVoxelAvatar] = useState(true)
  const [isEditMode, setIsEditMode] = useState(false)
  const [activePanelType, setActivePanelType] = useState<ConfigPanelType>(null)
  const [isChatOpen, setIsChatOpen] = useState(false)
  const [isClientListOpen, setIsClientListOpen] = useState(false)
  const [viewedProfilePubkey, setViewedProfilePubkey] = useState<string | null>(null)
  const [streamingUrl, setStreamingUrl] = useState<string | null>(null)
  const accountManager = useMemo(() => new AccountManager(), [])
  const avatarStateService = useMemo(() => new AvatarStateService(accountManager), [accountManager])
  const toast = useToast()

  // Voice chat
  const voice = useVoice()

  // Activity tracking
  const [isExploring, setIsExploring] = useState(false)
  const exploringTimeoutRef = useRef<NodeJS.Timeout | null>(null)

  // Avatar state
  const [avatarUrl, setAvatarUrl] = useState<string | undefined>()
  const [voxelModel, setVoxelModel] = useState<VoxelModelType>('boy')
  const [useVoxFile, setUseVoxFile] = useState(false)
  const [useOriginalColors, setUseOriginalColors] = useState(false)
  const [colorChangeCounter, setColorChangeCounter] = useState(0)
  const [teleportAnimationType, setTeleportAnimationType] = useState<TeleportAnimationType>('fade')

  // State restoration
  const [showRestoreModal, setShowRestoreModal] = useState(false)
  const restoreTimeoutRef = useRef<NodeJS.Timeout | null>(null)
  const initialStatePublished = useRef(false)

  // Fetch live event on mount
  useEffect(() => {
    const loadLiveEvent = async () => {
      try {
        // Fetch live event for MoQ relay URL
        const liveEvent = await fetchLiveEvent()
        if (liveEvent?.streaming_url) {
          setStreamingUrl(liveEvent.streaming_url)
          console.log('[App] MoQ streaming URL from live event:', liveEvent.streaming_url)
        } else {
          console.warn('[App] No streaming URL found in live event')

          // Fallback options for testing:
          // - Local relay: http://localhost:4443/anon
          // - Public relays:
          //   * https://relay.moq.dev/anon
          //   * https://relay.cloudflare.mediaoverquic.com/anon
          //   (NOTE: Use /anon suffix, NOT /crossworld-dev or other custom paths)
        }
      } catch (err) {
        console.error('[App] Failed to fetch live event:', err)
      }
    }
    loadLiveEvent()
  }, [])

  // Start avatar state subscription on mount
  useEffect(() => {
    avatarStateService.startSubscription()

    return () => {
      avatarStateService.stopSubscription()
    }
  }, [avatarStateService])

  // Query last state when logging in
  useEffect(() => {
    if (!pubkey) return

    // Show loading modal
    setShowRestoreModal(true)

    // Set 10-second timeout to dismiss modal
    restoreTimeoutRef.current = setTimeout(() => {
      setShowRestoreModal(false)
    }, 10000)

    const queryAndRestore = async () => {
      try {
        const state = await avatarStateService.queryLastState(pubkey)

        if (state) {
          // Auto-restore state
          console.log('[App] Restoring previous state')

          // Apply restored avatar config to UI state
          if (state.avatarType) {
            setUseVoxelAvatar(state.avatarType === 'voxel')
          }
          if (state.avatarModel) {
            setVoxelModel(state.avatarModel as VoxelModelType)
            setUseVoxFile(state.avatarModel !== 'generated')
          }
          if (state.avatarColors) {
            setUseOriginalColors(state.avatarColors === 'original')
          }
          if (state.avatarUrl) {
            setAvatarUrl(state.avatarUrl)
          }

          // Publish state with restored data
          publishInitialState(state)

          toast({
            title: 'State restored',
            description: 'Your previous avatar and position have been restored',
            status: 'success',
            duration: 3000,
            isClosable: true,
          })
        } else {
          // No previous state, publish new state
          console.log('[App] No previous state found, starting fresh')
          publishInitialState()
        }

        // Dismiss modal
        if (restoreTimeoutRef.current) {
          clearTimeout(restoreTimeoutRef.current)
        }
        setShowRestoreModal(false)
      } catch (err) {
        console.error('[App] Failed to query/restore state:', err)
        // On error, start fresh
        publishInitialState()

        if (restoreTimeoutRef.current) {
          clearTimeout(restoreTimeoutRef.current)
        }
        setShowRestoreModal(false)
      }
    }

    queryAndRestore()

    return () => {
      if (restoreTimeoutRef.current) {
        clearTimeout(restoreTimeoutRef.current)
      }
    }
  }, [pubkey])

  // Helper to publish initial state
  const publishInitialState = (restoredState?: Partial<AvatarState>) => {
    // Set avatar state service on voice manager
    voice.setClientStatusService?.(avatarStateService)

    // Build avatar config (use restored state or defaults)
    const avatarConfig: AvatarConfig = {
      avatarType: restoredState?.avatarType ?? (useVoxelAvatar ? 'voxel' : 'glb'),
      avatarModel: restoredState?.avatarModel ?? (useVoxFile ? voxelModel : 'generated'),
      avatarUrl: restoredState?.avatarUrl ?? (!useVoxelAvatar ? avatarUrl : undefined),
      avatarColors: restoredState?.avatarColors ?? (useOriginalColors ? 'original' : 'random'),
    }

    // Use restored position or default
    const position = restoredState?.position ?? { x: 4, y: 0, z: 4 }

    // Publish initial state event
    avatarStateService.publishStateEvent(
      avatarConfig,
      position,
      'active',
      false,
      false,
      ''
    ).then(() => {
      initialStatePublished.current = true
    }).catch(console.error)

    // Start heartbeat
    avatarStateService.startHeartbeat()
  }

  // Update avatar state when voice or activity state changes
  useEffect(() => {
    if (!pubkey) return

    // Build activities array
    const activities: Array<'chatting' | 'exploring' | 'editing'> = []
    if (isChatOpen) activities.push('chatting')
    if (isExploring) activities.push('exploring')
    if (isEditMode) activities.push('editing')

    // Publish update event
    avatarStateService.publishUpdate({
      voiceConnected: voice.isConnected,
      micEnabled: voice.micEnabled,
      activities,
    }).catch(console.error)
  }, [pubkey, voice.isConnected, voice.micEnabled, isChatOpen, isExploring, isEditMode, avatarStateService])

  // Track exploring activity with keyboard events
  useEffect(() => {
    if (!pubkey || isEditMode) return

    const handleKeyDown = (e: KeyboardEvent) => {
      // WASD or Arrow keys indicate exploring
      if (['w', 'a', 's', 'd', 'ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight'].includes(e.key.toLowerCase())) {
        const wasExploring = isExploring
        setIsExploring(true)

        // Publish immediately when exploring starts
        if (!wasExploring) {
          const activities: Array<'chatting' | 'exploring' | 'editing'> = []
          if (isChatOpen) activities.push('chatting')
          activities.push('exploring')
          if (isEditMode) activities.push('editing')

          avatarStateService.publishUpdate({
            voiceConnected: voice.isConnected,
            micEnabled: voice.micEnabled,
            activities,
          }).catch(console.error)
        }

        // Clear any existing timeout
        if (exploringTimeoutRef.current) {
          clearTimeout(exploringTimeoutRef.current)
        }

        // Set timeout to clear exploring after 5 seconds of inactivity
        exploringTimeoutRef.current = setTimeout(() => {
          setIsExploring(false)

          // Publish immediately when exploring stops
          const activities: Array<'chatting' | 'exploring' | 'editing'> = []
          if (isChatOpen) activities.push('chatting')
          if (isEditMode) activities.push('editing')

          avatarStateService.publishUpdate({
            voiceConnected: voice.isConnected,
            micEnabled: voice.micEnabled,
            activities,
          }).catch(console.error)
        }, 5000)
      }
    }

    window.addEventListener('keydown', handleKeyDown)

    return () => {
      window.removeEventListener('keydown', handleKeyDown)
      if (exploringTimeoutRef.current) {
        clearTimeout(exploringTimeoutRef.current)
      }
    }
  }, [pubkey, isEditMode, isExploring, avatarStateService, voice.isConnected, voice.micEnabled, isChatOpen])

  // Publish new state event when avatar configuration changes
  useEffect(() => {
    // Skip if not logged in or initial state not yet published
    if (!pubkey || !initialStatePublished.current) return

    // Build avatar config from current settings
    const avatarConfig: AvatarConfig = {
      avatarType: useVoxelAvatar ? 'voxel' : 'glb',
      avatarModel: useVoxFile ? voxelModel : 'generated',
      avatarUrl: !useVoxelAvatar ? avatarUrl : undefined,
      avatarColors: useOriginalColors ? 'original' : 'random',
    }

    // Publish state event with updated config (preserves position)
    avatarStateService.updateAvatarConfig(avatarConfig).catch(console.error)
  }, [pubkey, useVoxelAvatar, voxelModel, useVoxFile, useOriginalColors, avatarUrl, avatarStateService])

  const handleLogin = (publicKey: string) => {
    setPubkey(publicKey)
  }

  const handleLogout = async () => {
    // Publish final state update with away status
    await avatarStateService.publishUpdate({
      status: 'away',
    }).catch(console.error)

    // Disconnect voice if connected
    if (voice.isConnected) {
      await voice.disconnect()
    }
    // Stop avatar state heartbeat
    avatarStateService.stopHeartbeat()

    // Reset state
    initialStatePublished.current = false
    setPubkey(null)
    setIsChatOpen(false)
    setIsClientListOpen(false)
  }

  const handleAvatarUrlChange = (url: string) => {
    setAvatarUrl(url)
    setUseVoxelAvatar(false)
  }

  const handleRandomizeColors = () => {
    setColorChangeCounter(c => c + 1)
  }

  const handleCustomColor = (_color: string) => {
    setColorChangeCounter(c => c + 1)
  }

  const handleViewProfile = (profilePubkey: string) => {
    setViewedProfilePubkey(profilePubkey)
    setActivePanelType('profile')
  }

  const handleToggleVoice = async () => {
    if (!pubkey) {
      toast({
        title: 'Login required',
        description: 'Please login to use voice chat',
        status: 'warning',
        duration: 3000,
      })
      return
    }

    if (!streamingUrl) {
      toast({
        title: 'Voice unavailable',
        description: 'MoQ streaming URL not configured',
        status: 'error',
        duration: 3000,
      })
      return
    }

    try {
      if (voice.isConnected) {
        await voice.disconnect()
        toast({
          title: 'Voice disconnected',
          status: 'info',
          duration: 2000,
        })
      } else {
        const npub = npubEncode(pubkey)
        await voice.connect(streamingUrl, npub)
        toast({
          title: 'Voice connected',
          description: 'You can now enable your microphone',
          status: 'success',
          duration: 2000,
        })
      }
    } catch (err) {
      console.error('Voice toggle error:', err)
      toast({
        title: 'Voice error',
        description: err instanceof Error ? err.message : 'Failed to toggle voice',
        status: 'error',
        duration: 4000,
      })
    }
  }

  const handleToggleMic = async () => {
    try {
      await voice.toggleMic()
    } catch (err) {
      console.error('Mic toggle error:', err)
      toast({
        title: 'Microphone error',
        description: err instanceof Error ? err.message : 'Failed to toggle microphone',
        status: 'error',
        duration: 4000,
      })
    }
  }

  return (
    <AccountsProvider manager={accountManager}>
      <ChakraProvider>
        <WorldCanvas
          isLoggedIn={pubkey !== null}
          useVoxelAvatar={useVoxelAvatar}
          onToggleAvatarType={setUseVoxelAvatar}
          isEditMode={isEditMode}
          voxelModel={voxelModel}
          onVoxelModelChange={setVoxelModel}
          useVoxFile={useVoxFile}
          onVoxFileChange={setUseVoxFile}
          useOriginalColors={useOriginalColors}
          onColorModeChange={setUseOriginalColors}
          onAvatarUrlChange={handleAvatarUrlChange}
          avatarUrl={avatarUrl}
          colorChangeCounter={colorChangeCounter}
          avatarStateService={avatarStateService}
          currentUserPubkey={pubkey}
          teleportAnimationType={teleportAnimationType}
        />
        <TopBar
          pubkey={pubkey}
          onLogin={handleLogin}
        />
        {pubkey && (
          <LeftSidebarPanel
            onOpenPanel={setActivePanelType}
            onLogout={handleLogout}
            activePanelType={activePanelType}
            isEditMode={isEditMode}
            onToggleEditMode={setIsEditMode}
            isChatOpen={isChatOpen}
            onToggleChat={() => setIsChatOpen(!isChatOpen)}
            isClientListOpen={isClientListOpen}
            onToggleClientList={() => setIsClientListOpen(!isClientListOpen)}
            voiceConnected={voice.isConnected}
            voiceConnecting={voice.status === 'connecting'}
            micEnabled={voice.micEnabled}
            speaking={voice.speaking}
            participantCount={voice.participantCount}
            onToggleVoice={handleToggleVoice}
            onToggleMic={handleToggleMic}
          />
        )}

        {/* Config Panels */}
        {activePanelType === 'network' && <NetworkConfigPanel />}
        {activePanelType === 'profile' && <ProfilePanel pubkey={viewedProfilePubkey || pubkey} />}
        {activePanelType === 'avatar' && (
          <AvatarPanel
            useVoxelAvatar={useVoxelAvatar}
            onToggleAvatarType={setUseVoxelAvatar}
            currentModel={voxelModel}
            onModelChange={setVoxelModel}
            useVoxFile={useVoxFile}
            onSourceChange={setUseVoxFile}
            useOriginalColors={useOriginalColors}
            onColorModeChange={setUseOriginalColors}
            onRandomizeColors={handleRandomizeColors}
            onCustomColor={handleCustomColor}
            onAvatarUrlChange={handleAvatarUrlChange}
            currentUrl={avatarUrl}
            teleportAnimationType={teleportAnimationType}
            onTeleportAnimationChange={setTeleportAnimationType}
          />
        )}

        {/* Chat Panel */}
        <ChatPanel isOpen={isChatOpen} currentPubkey={pubkey} onViewProfile={handleViewProfile} />

        {/* Client List Panel */}
        <ClientListPanel
          isOpen={isClientListOpen}
          statusService={avatarStateService}
        />

        {/* Loading State Modal */}
        <RestoreStateModal
          isOpen={showRestoreModal}
        />
      </ChakraProvider>
    </AccountsProvider>
  )
}

export default App
