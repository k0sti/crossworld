import * as logger from './utils/logger';
import { useState, useMemo, useEffect, useRef } from 'react'
import { useToast, Box, Text } from '@chakra-ui/react'
import { useAccountManager } from 'applesauce-react/hooks'
import { TopBar, ConfigPanelType, ProfilePanel } from '@crossworld/common'
import { WorldCanvas } from './components/WorldCanvas'
import { ConfigPanel } from './components/ConfigPanel'
import { NetworkConfigPanel } from './components/NetworkConfigPanel'
import { InfoPanel } from './components/InfoPanel'
import { SelectAvatar, type AvatarSelection } from './components/SelectAvatar'
import { ChatPanel } from './components/ChatPanel'
import { ClientListPanel } from './components/ClientListPanel'
import { RestoreStateModal } from './components/RestoreStateModal'
import { PublishWorldModal } from './components/PublishWorldModal'
import { ColorPalette } from './components/ColorPalette'
import { ScriptPanel } from './components/ScriptPanel'
import { AvatarStateService, type AvatarConfig, type AvatarState } from './services/avatar-state'
import { useVoice } from './hooks/useVoice'
import type { TeleportAnimationType } from './renderer/teleport-animation'
import { LoginSettingsService } from '@crossworld/common'
import { ExtensionAccount, SimpleAccount } from 'applesauce-accounts/accounts'
import { ExtensionSigner } from 'applesauce-signers'
import { fetchCurrentWorld, validateWorldConfig } from './services/world-storage'
import { loadModelFromCSM } from './utils/csmUtils'
import { getMacroDepth, getMicroDepth } from './config/depth-config'

function App() {
  const [pubkey, setPubkey] = useState<string | null>(null)
  const [isEditMode, setIsEditMode] = useState(false)
  const [isCameraMode, setIsCameraMode] = useState(false)
  const [activePanelType, setActivePanelType] = useState<ConfigPanelType>(null)
  const [isChatOpen, setIsChatOpen] = useState(false)
  const [isClientListOpen, setIsClientListOpen] = useState(false)
  const [viewedProfilePubkey, setViewedProfilePubkey] = useState<string | null>(null)
  const accountManager = useAccountManager()
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
  })
  const [teleportAnimationType, setTeleportAnimationType] = useState<TeleportAnimationType>('fade')

  // Avatar selection modal
  const [showSelectAvatar, setShowSelectAvatar] = useState(false)

  // State restoration
  const [showRestoreModal, setShowRestoreModal] = useState(false)
  const restoreTimeoutRef = useRef<NodeJS.Timeout | null>(null)
  const initialStatePublished = useRef(false)
  const voiceAutoConnected = useRef(false)

  // Speech/Voice enabled state
  const [speechEnabled, setSpeechEnabled] = useState(false)

  // Time of day state
  const [timeOfDay, setTimeOfDay] = useState(0.35); // Start slightly after sunrise
  const [sunAutoMove, setSunAutoMove] = useState(false); // Start with sun fixed
  const [sunSpeed, setSunSpeed] = useState(0.01);

  const geometryControllerRef = useRef<any>(null)
  const sceneManagerRef = useRef<any>(null)

  // World CSM state
  const [worldCSM, setWorldCSM] = useState<string>('')
  const [isScriptPanelOpen, setIsScriptPanelOpen] = useState(false)

  // Publish world modal
  const [isPublishWorldOpen, setIsPublishWorldOpen] = useState(false)

  // Voice auto-connect disabled - user must manually connect
  // useEffect(() => {
  //   if (!pubkey || !streamingUrl || voiceAutoConnected.current) return

  //   const autoConnect = async () => {
  //     try {
  //       const npub = npubEncode(pubkey)
  //       await voice.connect(streamingUrl, npub)
  //       voiceAutoConnected.current = true
  //       logger.log('ui', '[App] Auto-connected to voice chat')
  //     } catch (err) {
  //       logger.error('ui', '[App] Failed to auto-connect to voice:', err)
  //     }
  //   }

  //   // Use a small delay to ensure this only runs on initial login
  //   const timer = setTimeout(autoConnect, 100)
  //   return () => clearTimeout(timer)
  // }, [pubkey, streamingUrl, voice])

  // Start avatar state subscription on mount
  useEffect(() => {
    avatarStateService.startSubscription()

    return () => {
      avatarStateService.stopSubscription()
    }
  }, [avatarStateService])

  // Auto-login on mount if login settings exist
  useEffect(() => {
    const autoLogin = async () => {
      const loginSettings = LoginSettingsService.load()

      if (!loginSettings) {
        return
      }

      try {
        if (loginSettings.method === 'guest') {
          // Restore guest account
          const guestData = LoginSettingsService.loadGuestAccount()
          if (!guestData) {
            logger.error('ui', '[App] Guest account data missing')
            LoginSettingsService.clear()
            return
          }

          const account = SimpleAccount.fromJSON(guestData.account)
          accountManager.addAccount(account)
          accountManager.setActive(account)

          toast({
            title: 'Welcome back',
            description: `Logged in as ${guestData.name}`,
            status: 'success',
            duration: 3000,
            isClosable: true,
          })

          setPubkey(account.pubkey)
        } else if (loginSettings.method === 'extension') {
          // Try to reconnect with extension
          if (!window.nostr) {
            LoginSettingsService.clear()
            return
          }

          const signer = new ExtensionSigner()
          const publicKey = await signer.getPublicKey()

          if (publicKey !== loginSettings.pubkey) {
            logger.warn('ui', '[App] Extension pubkey mismatch, clearing settings')
            LoginSettingsService.clear()
            return
          }

          const existingAccount = accountManager.accounts.find(
            (a) => a.type === ExtensionAccount.type && a.pubkey === publicKey
          )

          if (!existingAccount) {
            const account = new ExtensionAccount(publicKey, signer)
            accountManager.addAccount(account)
            accountManager.setActive(account)
          } else {
            accountManager.setActive(existingAccount)
          }

          toast({
            title: 'Welcome back',
            description: 'Reconnected to extension',
            status: 'success',
            duration: 3000,
            isClosable: true,
          })

          setPubkey(publicKey)
        } else if (loginSettings.method === 'amber') {
          // Amber auto-login not supported (requires user interaction)
          LoginSettingsService.clear()
        }
      } catch (error) {
        logger.error('ui', '[App] Auto-login failed:', error)
        LoginSettingsService.clear()
        toast({
          title: 'Auto-login failed',
          description: 'Please log in again',
          status: 'warning',
          duration: 3000,
          isClosable: true,
        })
      }
    }

    autoLogin()
  }, [accountManager, toast])

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
          // Build avatar config from restored state
          const restoredConfig: AvatarConfig = {
            avatarType: state.avatarType || 'vox',
            avatarId: state.avatarId,
            avatarUrl: state.avatarUrl,
            avatarData: state.avatarData,
            avatarMod: state.avatarMod,
          }
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
          setShowSelectAvatar(true)
          // Still dismiss the restore modal
        }

        // Dismiss modal
        if (restoreTimeoutRef.current) {
          clearTimeout(restoreTimeoutRef.current)
        }
        setShowRestoreModal(false)
      } catch (err) {
        logger.error('ui', '[App] Failed to query/restore state:', err)
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

  // Auto-load world from Nostr when user logs in
  useEffect(() => {
    if (!pubkey || !geometryControllerRef.current) return

    const autoLoadWorld = async () => {
      try {
        const world = await fetchCurrentWorld(pubkey)

        if (world) {
          // Validate world matches current configuration
          const validation = validateWorldConfig(world)

          if (validation.valid) {
            const totalDepth = getMacroDepth() + getMicroDepth()

            await loadModelFromCSM(world.csmCode, 'world', totalDepth)

            // Trigger mesh update
            if (geometryControllerRef.current) {
              geometryControllerRef.current.forceUpdate()
            }

            toast({
              title: 'World loaded',
              description: world.title || 'Your saved world has been loaded',
              status: 'success',
              duration: 3000,
            })
          } else {
            logger.warn('ui', '[App] World config mismatch:', validation.error)
          }
        }
      } catch (error) {
        logger.error('ui', '[App] Failed to auto-load world:', error)
        // Don't show toast for errors, just log them
      }
    }

    autoLoadWorld()
  }, [pubkey, toast])

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

  // Reset script panel state when exiting edit mode
  useEffect(() => {
    if (!isEditMode) {
      setIsScriptPanelOpen(false)
    }
  }, [isEditMode])

  // Toggle script panel with 'l' key in edit mode
  useEffect(() => {
    if (!isEditMode) return

    const handleKeyDown = (e: KeyboardEvent) => {
      // Ignore if user is typing in an input/textarea
      const target = e.target as HTMLElement
      if (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA') return

      if (e.key === 'l' || e.key === 'L') {
        setIsScriptPanelOpen(prev => !prev)
      }
    }

    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [isEditMode])

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

    // Clear login settings
    LoginSettingsService.clear()

    // Reset state
    initialStatePublished.current = false
    voiceAutoConnected.current = false
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

    setAvatarConfig(config)
    setTeleportAnimationType(selection.teleportAnimationType)
    setShowSelectAvatar(false)

    // If this is first login, publish initial state with the new config
    if (pubkey && !initialStatePublished.current) {
      publishInitialState({
        avatarType: config.avatarType,
        avatarId: config.avatarId,
        avatarUrl: config.avatarUrl,
        avatarData: config.avatarData,
      })
    }
  }

  const handleRestart = () => {
    // Reset avatar config to empty state
    setAvatarConfig({
      avatarType: 'vox',
      avatarId: undefined,
      avatarUrl: undefined,
      avatarData: undefined,
    })
    setTeleportAnimationType('fade')
  }

  const handleViewProfile = (profilePubkey: string) => {
    setViewedProfilePubkey(profilePubkey)
    setActivePanelType('profile')
  }

  const handleColorSelect = (_color: string, index: number) => {
    if (sceneManagerRef.current) {
      sceneManagerRef.current.setSelectedColorIndex(index)
    }
  }

  // Initialize ground render mode when geometry controller is ready
  useEffect(() => {
    if (geometryControllerRef.current) {
      // Always use combined ground mode (cube + flat)
      geometryControllerRef.current.setGroundRenderMode(true)
    }
  }, [])

  const handlePublishWorld = () => {
    // Ensure geometry controller is initialized before opening modal
    if (!geometryControllerRef.current) {
      toast({
        title: 'World not ready',
        description: 'Please wait for the world to initialize',
        status: 'warning',
        duration: 3000,
      })
      return
    }
    setIsPublishWorldOpen(true)
  }

  return (
      <>
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
          speechEnabled={speechEnabled}
          onSpeechEnabledChange={setSpeechEnabled}
          onWorldCSMUpdate={setWorldCSM}
          timeOfDay={timeOfDay}
          sunAutoMove={sunAutoMove}
          sunSpeed={sunSpeed}
          onPublishWorld={handlePublishWorld}
        />
        <TopBar
          pubkey={pubkey}
          onLogin={handleLogin}
          onOpenPanel={setActivePanelType}
          onOpenProfile={() => setActivePanelType('profile')}
          activePanelType={activePanelType}
          isEditMode={isEditMode}
          onToggleEditMode={() => setIsEditMode(!isEditMode)}
        />

        {/* Config Panels */}
        {activePanelType === 'config' && (
          <ConfigPanel
            onClose={() => setActivePanelType(null)}
            onOpenPanel={setActivePanelType}
            timeOfDay={timeOfDay}
            onTimeOfDayChange={setTimeOfDay}
            sunAutoMove={sunAutoMove}
            onSunAutoMoveChange={setSunAutoMove}
            sunSpeed={sunSpeed}
            onSunSpeedChange={setSunSpeed}
          />
        )}
        <NetworkConfigPanel
          isOpen={activePanelType === 'network'}
          onClose={() => setActivePanelType(null)}
        />
        <InfoPanel
          isOpen={activePanelType === 'info'}
          onClose={() => setActivePanelType(null)}
        />
        <ProfilePanel
          pubkey={viewedProfilePubkey || pubkey}
          isOpen={activePanelType === 'profile'}
          onClose={() => setActivePanelType(null)}
          local_user={!viewedProfilePubkey || viewedProfilePubkey === pubkey}
          onLogout={handleLogout}
          onOpenAvatarSelection={() => setActivePanelType('avatar')}
          onRestart={handleRestart}
        />
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

        {/* Client List Panel - Hide when menu is open */}
        <ClientListPanel
          isOpen={isClientListOpen && activePanelType !== 'config'}
          statusService={avatarStateService}
          onOpenProfile={handleViewProfile}
          isEditMode={isEditMode}
        />

        {/* Chat Button (bottom left) */}
        {pubkey && (
          <Box
            as="button"
            onClick={() => setIsChatOpen(!isChatOpen)}
            position="fixed"
            bottom={4}
            left={4}
            w="48px"
            h="48px"
            bg={isChatOpen ? "rgba(120, 120, 120, 0.2)" : "rgba(80, 80, 80, 0.1)"}
            border="1px solid rgba(255, 255, 255, 0.1)"
            _hover={{
              bg: 'rgba(120, 120, 120, 0.2)',
              borderColor: 'rgba(255, 255, 255, 0.2)'
            }}
            _active={{
              bg: 'rgba(60, 60, 60, 0.3)',
            }}
            transition="all 0.1s"
            cursor="pointer"
            display="flex"
            alignItems="center"
            justifyContent="center"
            zIndex={1500}
            borderRadius="md"
          >
            <Text fontSize="xl">💬</Text>
          </Box>
        )}

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

        {/* Color Palette (edit mode) */}
        <ColorPalette isVisible={isEditMode} onColorSelect={handleColorSelect} />

        {/* Script Panel (edit mode) */}
        {isEditMode && isScriptPanelOpen && <ScriptPanel csmText={worldCSM} />}

        {/* Publish World Modal */}
        <PublishWorldModal
          isOpen={isPublishWorldOpen}
          onClose={() => setIsPublishWorldOpen(false)}
          accountManager={accountManager}
          geometryControllerRef={geometryControllerRef}
        />

      </>
  )
}

export default App
