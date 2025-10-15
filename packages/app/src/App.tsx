import { useState, useMemo } from 'react'
import { ChakraProvider } from '@chakra-ui/react'
import { AccountsProvider } from 'applesauce-react/providers'
import { AccountManager } from 'applesauce-accounts'
import { TopBar } from './components/TopBar'
import { WorldCanvas } from './components/WorldCanvas'
import { LeftSidebarPanel } from './components/LeftSidebarPanel'
import { ConfigPanelType } from './components/ConfigPanel'

function App() {
  const [pubkey, setPubkey] = useState<string | null>(null)
  const [useVoxelAvatar, setUseVoxelAvatar] = useState(true)
  const [isEditMode, setIsEditMode] = useState(false)
  const [activePanelType, setActivePanelType] = useState<ConfigPanelType>(null)
  const accountManager = useMemo(() => new AccountManager(), [])

  const handleLogin = (publicKey: string) => {
    setPubkey(publicKey)
  }

  const handleLogout = () => {
    setPubkey(null)
  }

  const handleCloseAllPanels = () => {
    setActivePanelType(null)
  }

  return (
    <AccountsProvider manager={accountManager}>
      <ChakraProvider>
        <WorldCanvas
          isLoggedIn={pubkey !== null}
          useVoxelAvatar={useVoxelAvatar}
          onToggleAvatarType={setUseVoxelAvatar}
          isEditMode={isEditMode}
        />
        <TopBar
          pubkey={pubkey}
          onLogin={handleLogin}
          onLogout={handleLogout}
          activePanelType={activePanelType}
          onCloseAllPanels={handleCloseAllPanels}
        />
        {pubkey && (
          <LeftSidebarPanel
            onOpenPanel={setActivePanelType}
            useVoxelAvatar={useVoxelAvatar}
            onToggleAvatarType={setUseVoxelAvatar}
            onLogout={handleLogout}
            activePanelType={activePanelType}
            isEditMode={isEditMode}
            onToggleEditMode={setIsEditMode}
          />
        )}
      </ChakraProvider>
    </AccountsProvider>
  )
}

export default App
