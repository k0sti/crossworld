import { useState, useMemo, useEffect, useRef } from 'react'
import { ChakraProvider, useToast } from '@chakra-ui/react'
import { AccountsProvider } from 'applesauce-react/providers'
import { AccountManager } from 'applesauce-accounts'
import { TopBar } from './components/TopBar'
import { WorldCanvas } from './components/WorldCanvas'
import { LeftSidebarPanel } from './components/LeftSidebarPanel'
import { ConfigPanelType } from './components/ConfigPanel'
import { NetworkConfigPanel } from './components/NetworkConfigPanel'
import { InfoPanel } from './components/InfoPanel'
import { ProfilePanel } from './components/ProfilePanel'
import { SelectAvatar, type AvatarSelection } from './components/SelectAvatar'
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
  const [isEditMode, setIsEditMode] = useState(false)
  const [isCameraMode, setIsCameraMode] = useState(false)
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

  // Avatar state - unified configuration
  const [avatarConfig, setAvatarConfig] = useState<AvatarConfig>({
    avatarType: 'vox',
    avatarId: 'boy',
  })
  const [teleportAnimationType, setTeleportAnimationType] = useState<TeleportAnimationType>('fade')

  // Avatar selection modal
  const [showSelectAvatar, setShowSelectAvatar] = useState(false)

  // State restoration
  const [showRestoreModal, setShowRestoreModal] = useState(false)
  const restoreTimeoutRef = useRef<NodeJS.Timeout | null>(null)
  const initialStatePublished = useRef(false)

  // Ground render mode
  const [useCubeGround, setUseCubeGround] = useState(false)
  const geometryControllerRef = useRef<any>(null)
  const sceneManagerRef = useRef<any>(null)

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

          // Build avatar config from restored state
          const restoredConfig: AvatarConfig = {
            avatarType: state.avatarType || 'vox',
            avatarId: state.avatarId,
            avatarUrl: state.avatarUrl,
            avatarData: state.avatarData,
            avatarMod: state.avatarMod,
          }
          console.log('[App] Restoring avatar config:', restoredConfig)
          setAvatarConfig(restoredConfig)

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
          // No previous state, show avatar selection
          console.log('[App] No previous state found, showing avatar selection')
          setShowSelectAvatar(true)
          // Still dismiss the restore modal
        }

        // Dismiss modal
        if (restoreTimeoutRef.current) {
          clearTimeout(restoreTimeoutRef.current)
        }
        setShowRestoreModal(false)
      } catch (err) {
        console.error('[App] Failed to query/restore state:', err)
        // On error, show avatar selection
        setShowSelectAvatar(true)

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
  }, [pubkey, avatarStateService, toast])

  // Helper to publish initial state
  const publishInitialState = (restoredState?: Partial<AvatarState>) => {
    // Set avatar state service on voice manager
    voice.setClientStatusService?.(avatarStateService)

    // Build avatar config (use restored state or defaults)
    const config: AvatarConfig = {
      avatarType: restoredState?.avatarType ?? avatarConfig.avatarType,
      avatarId: restoredState?.avatarId ?? avatarConfig.avatarId,
      avatarUrl: restoredState?.avatarUrl ?? avatarConfig.avatarUrl,
      avatarData: restoredState?.avatarData ?? avatarConfig.avatarData,
      avatarMod: restoredState?.avatarMod ?? avatarConfig.avatarMod,
    }
    console.log('[App] Publishing initial state with config:', config)

    // Use restored position or default
    const position = restoredState?.position ?? { x: 4, y: 0, z: 4 }

    // Publish initial state event
    avatarStateService.publishStateEvent(
      config,
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

  // Set up camera mode exit callback (triggered when pointer lock is released)
  useEffect(() => {
    if (sceneManagerRef.current) {
      sceneManagerRef.current.setOnCameraModeExit(() => {
        setIsCameraMode(false)
      })
    }
  }, [])

  // Publish new state event when avatar configuration changes
  useEffect(() => {
    // Skip if not logged in or initial state not yet published
    if (!pubkey || !initialStatePublished.current) return

    // Publish state event with updated config (preserves position)
    avatarStateService.updateAvatarConfig(avatarConfig).catch(console.error)
  }, [pubkey, avatarConfig, avatarStateService])

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

    // Remove own state to prevent showing as remote avatar
    if (pubkey) {
      avatarStateService.removeUserState(pubkey)
    }

    // Reset state
    initialStatePublished.current = false
    setPubkey(null)
    setIsChatOpen(false)
    setIsClientListOpen(false)
  }

  const handleAvatarSelection = (selection: AvatarSelection) => {
    const config: AvatarConfig = {
      avatarType: selection.avatarType,
      avatarId: selection.avatarId,
      avatarUrl: selection.avatarUrl,
      avatarData: selection.avatarData,
    }

    console.log('[App] Avatar selection received:', selection)
    console.log('[App] Setting avatar config:', config)
    setAvatarConfig(config)
    setTeleportAnimationType(selection.teleportAnimationType)
    setShowSelectAvatar(false)

    // If this is first login, publish initial state
    if (pubkey && !initialStatePublished.current) {
      publishInitialState()
    }
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
          isEditMode={isEditMode}
          isCameraMode={isCameraMode}
          avatarConfig={avatarConfig}
          teleportAnimationType={teleportAnimationType}
          avatarStateService={avatarStateService}
          currentUserPubkey={pubkey}
          geometryControllerRef={geometryControllerRef}
          sceneManagerRef={sceneManagerRef}
        />
        <TopBar
          pubkey={pubkey}
          onLogin={handleLogin}
          onOpenPanel={setActivePanelType}
          onOpenProfile={() => setActivePanelType('profile')}
          activePanelType={activePanelType}
        />
        {pubkey && (
          <LeftSidebarPanel
            onOpenPanel={setActivePanelType}
            activePanelType={activePanelType}
            isEditMode={isEditMode}
            onToggleEditMode={setIsEditMode}
            isCameraMode={isCameraMode}
            onToggleCameraMode={() => setIsCameraMode(!isCameraMode)}
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
            useCubeGround={useCubeGround}
            onToggleGroundRenderMode={() => {
              const newMode = !useCubeGround
              setUseCubeGround(newMode)
              if (geometryControllerRef.current) {
                geometryControllerRef.current.setGroundRenderMode(newMode)
              }
            }}
          />
        )}

        {/* Config Panels */}
        {activePanelType === 'network' && <NetworkConfigPanel />}
        {activePanelType === 'info' && <InfoPanel />}
        {activePanelType === 'profile' && (
          <ProfilePanel
            pubkey={viewedProfilePubkey || pubkey}
            onClose={() => setActivePanelType(null)}
            local_user={!viewedProfilePubkey || viewedProfilePubkey === pubkey}
            onLogout={handleLogout}
          />
        )}
        {activePanelType === 'avatar' && (
          <SelectAvatar
            isOpen={true}
            onClose={() => setActivePanelType(null)}
            onSave={handleAvatarSelection}
            currentSelection={{
              avatarType: avatarConfig.avatarType,
              avatarId: avatarConfig.avatarId,
              avatarUrl: avatarConfig.avatarUrl,
              avatarData: avatarConfig.avatarData,
              teleportAnimationType,
            }}
          />
        )}

        {/* Chat Panel */}
        <ChatPanel isOpen={isChatOpen} currentPubkey={pubkey} onViewProfile={handleViewProfile} />

        {/* Client List Panel */}
        <ClientListPanel
          isOpen={isClientListOpen}
          statusService={avatarStateService}
          onOpenProfile={handleViewProfile}
        />

        {/* Loading State Modal */}
        <RestoreStateModal
          isOpen={showRestoreModal}
        />

        {/* Avatar Selection Modal (first login) */}
        <SelectAvatar
          isOpen={showSelectAvatar}
          onClose={() => {
            // Don't allow closing without selecting
            // Could publish a default state here if needed
          }}
          onSave={handleAvatarSelection}
          currentSelection={{
            avatarType: avatarConfig.avatarType,
            avatarId: avatarConfig.avatarId,
            avatarUrl: avatarConfig.avatarUrl,
            teleportAnimationType,
          }}
        />
      </ChakraProvider>
    </AccountsProvider>
  )
}

export default App
