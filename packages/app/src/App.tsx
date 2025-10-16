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

function App() {
  const [pubkey, setPubkey] = useState<string | null>(null)
  const [useVoxelAvatar, setUseVoxelAvatar] = useState(true)
  const [isEditMode, setIsEditMode] = useState(false)
  const [activePanelType, setActivePanelType] = useState<ConfigPanelType>(null)
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
          />
        )}

        {/* Config Panels */}
        {activePanelType === 'network' && <NetworkConfigPanel />}
        {activePanelType === 'profile' && <ProfilePanel pubkey={pubkey} />}
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
      </ChakraProvider>
    </AccountsProvider>
  )
}

export default App
