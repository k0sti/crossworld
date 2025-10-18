import { useState, useMemo } from 'react'
import { ChakraProvider } from '@chakra-ui/react'
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
import { HelpOverlay } from './components/HelpOverlay'
import { useKeyboardManager } from './hooks/useKeyboardManager'

function App() {
  const [pubkey, setPubkey] = useState<string | null>(null)
  const [useVoxelAvatar, setUseVoxelAvatar] = useState(true)
  const [isEditMode, setIsEditMode] = useState(false)
  const [activePanelType, setActivePanelType] = useState<ConfigPanelType>(null)
  const [isChatOpen, setIsChatOpen] = useState(false)
  const [viewedProfilePubkey, setViewedProfilePubkey] = useState<string | null>(null)
  const [showHelp, setShowHelp] = useState(false)
  const accountManager = useMemo(() => new AccountManager(), [])

  // Avatar state
  const [avatarUrl, setAvatarUrl] = useState<string | undefined>()
  const [voxelModel, setVoxelModel] = useState<VoxelModelType>('boy')
  const [useVoxFile, setUseVoxFile] = useState(false)
  const [useOriginalColors, setUseOriginalColors] = useState(false)
  const [colorChangeCounter, setColorChangeCounter] = useState(0)

  const handleLogin = (publicKey: string) => {
    setPubkey(publicKey)
  }

  const handleLogout = () => {
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

  const handleToggleFullscreen = () => {
    if (!document.fullscreenElement) {
      document.documentElement.requestFullscreen()
    } else {
      document.exitFullscreen()
    }
  }

  // Keyboard manager with callbacks
  const { getKeysPressed } = useKeyboardManager(
    pubkey !== null,
    isChatOpen,
    {
      onOpenNetworkPanel: () => setActivePanelType('network'),
      onOpenProfilePanel: () => setActivePanelType('profile'),
      onOpenAvatarPanel: () => setActivePanelType('avatar'),
      onToggleChat: () => {
        console.log('[App] onToggleChat - current:', isChatOpen, '-> new:', !isChatOpen)
        setIsChatOpen(!isChatOpen)
      },
      onLogout: handleLogout,
      onClosePanel: () => setActivePanelType(null),
      onToggleCameraView: () => {
        // TODO: Implement camera view toggle in SceneManager
        console.log('Toggle camera view')
      },
      onToggleHelp: () => setShowHelp(!showHelp),
      onToggleFullscreen: handleToggleFullscreen,
    }
  )

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
          getKeysPressed={getKeysPressed}
          isChatOpen={isChatOpen}
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
            onToggleChat={() => {
              console.log('[App] LeftSidebar onToggleChat - current:', isChatOpen, '-> new:', !isChatOpen)
              setIsChatOpen(!isChatOpen)
            }}
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
        <ChatPanel
          isOpen={isChatOpen}
          currentPubkey={pubkey}
          onViewProfile={handleViewProfile}
        />

        {/* Help Overlay */}
        <HelpOverlay isOpen={showHelp} onClose={() => setShowHelp(false)} />
      </ChakraProvider>
    </AccountsProvider>
  )
}

export default App
