import { useState, useMemo, useEffect } from 'react'
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
import { fetchLiveEvent } from './services/live-event'
import { useVoice } from './hooks/useVoice'
import { npubEncode } from 'nostr-tools/nip19'

function App() {
  const [pubkey, setPubkey] = useState<string | null>(null)
  const [useVoxelAvatar, setUseVoxelAvatar] = useState(true)
  const [isEditMode, setIsEditMode] = useState(false)
  const [activePanelType, setActivePanelType] = useState<ConfigPanelType>(null)
  const [isChatOpen, setIsChatOpen] = useState(false)
  const [viewedProfilePubkey, setViewedProfilePubkey] = useState<string | null>(null)
  const [streamingUrl, setStreamingUrl] = useState<string | null>(null)
  const accountManager = useMemo(() => new AccountManager(), [])
  const toast = useToast()

  // Voice chat
  const voice = useVoice()

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

  const handleLogin = (publicKey: string) => {
    setPubkey(publicKey)
  }

  const handleLogout = async () => {
    // Disconnect voice if connected
    if (voice.isConnected) {
      await voice.disconnect()
    }
    setPubkey(null)
    setIsChatOpen(false)
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
      </ChakraProvider>
    </AccountsProvider>
  )
}

export default App
