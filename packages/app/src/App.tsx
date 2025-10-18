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
import { fetchLiveEvent } from './services/live-event'
import { ClientStatusService } from './services/client-status'
import { useVoice } from './hooks/useVoice'
import { npubEncode } from 'nostr-tools/nip19'

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
  const clientStatusService = useMemo(() => new ClientStatusService(accountManager), [accountManager])
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

  // Fetch live event on mount
  useEffect(() => {
    const loadLiveEvent = async () => {
      try {
        // TEMP: Use local relay (confirmed working)
        // const localRelay = 'http://localhost:4443/anon'
        // setStreamingUrl(localRelay)
        // console.log('MoQ streaming URL (local relay):', localRelay)

        // Other relays to test later:
        // const hangRelay = 'https://relay.moq.dev/anon'
        // const cfRelay = 'https://relay.cloudflare.mediaoverquic.com/crossworld-dev'

        // Original live event fetching (commented out):
        const liveEvent = await fetchLiveEvent()
        if (liveEvent?.streaming_url) {
          setStreamingUrl(liveEvent.streaming_url)
          console.log('MoQ streaming URL:', liveEvent.streaming_url)
        } else {
          console.warn('No streaming URL found in live event')
        }
      } catch (err) {
        console.error('Failed to fetch live event:', err)
      }
    }
    loadLiveEvent()
  }, [])

  // Start client status subscription on mount
  useEffect(() => {
    clientStatusService.startSubscription()

    return () => {
      clientStatusService.stopSubscription()
    }
  }, [clientStatusService])

  // Publish client status when logged in
  useEffect(() => {
    if (!pubkey) return

    // Set client status service on voice manager
    voice.setClientStatusService?.(clientStatusService)

    // Start publishing status updates
    clientStatusService.startStatusUpdates({
      status: 'active',
      clientName: 'Crossworld Web',
      clientVersion: '0.1.0',
    })

    return () => {
      clientStatusService.stopStatusUpdates()
    }
  }, [pubkey, clientStatusService, voice])

  // Update client status when voice or activity state changes
  useEffect(() => {
    if (!pubkey) return

    // Publish immediately for UI actions (voice, mic, chat, edit mode)
    const isUIAction = true

    clientStatusService.publishStatus({
      voiceConnected: voice.isConnected,
      micEnabled: voice.micEnabled,
      isChatting: isChatOpen,
      isExploring,
      isEditing: isEditMode,
    }, isUIAction).catch(console.error)
  }, [pubkey, voice.isConnected, voice.micEnabled, isChatOpen, isExploring, isEditMode, clientStatusService])

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
          clientStatusService.publishStatus({
            voiceConnected: voice.isConnected,
            micEnabled: voice.micEnabled,
            isChatting: isChatOpen,
            isExploring: true,
            isEditing: isEditMode,
          }, true).catch(console.error)
        }

        // Clear any existing timeout
        if (exploringTimeoutRef.current) {
          clearTimeout(exploringTimeoutRef.current)
        }

        // Set timeout to clear exploring after 5 seconds of inactivity
        exploringTimeoutRef.current = setTimeout(() => {
          setIsExploring(false)

          // Publish immediately when exploring stops
          clientStatusService.publishStatus({
            voiceConnected: voice.isConnected,
            micEnabled: voice.micEnabled,
            isChatting: isChatOpen,
            isExploring: false,
            isEditing: isEditMode,
          }, true).catch(console.error)
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
  }, [pubkey, isEditMode, isExploring, clientStatusService, voice.isConnected, voice.micEnabled, isChatOpen])

  const handleLogin = (publicKey: string) => {
    setPubkey(publicKey)
  }

  const handleLogout = async () => {
    // Disconnect voice if connected
    if (voice.isConnected) {
      await voice.disconnect()
    }
    // Stop client status updates
    clientStatusService.stopStatusUpdates()
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
          />
        )}

        {/* Chat Panel */}
        <ChatPanel isOpen={isChatOpen} currentPubkey={pubkey} onViewProfile={handleViewProfile} />

        {/* Client List Panel */}
        <ClientListPanel
          isOpen={isClientListOpen}
          statusService={clientStatusService}
        />
      </ChakraProvider>
    </AccountsProvider>
  )
}

export default App
