import { useState, useMemo } from 'react'
import { ChakraProvider } from '@chakra-ui/react'
import { AccountsProvider } from 'applesauce-react/providers'
import { AccountManager } from 'applesauce-accounts'
import { TopBar } from './components/TopBar'
import { WorldCanvas } from './components/WorldCanvas'

function App() {
  const [pubkey, setPubkey] = useState<string | null>(null)
  const [useVoxelAvatar, setUseVoxelAvatar] = useState(true)
  const accountManager = useMemo(() => new AccountManager(), [])

  const handleLogin = (publicKey: string) => {
    setPubkey(publicKey)
  }

  const handleLogout = () => {
    setPubkey(null)
  }

  return (
    <AccountsProvider manager={accountManager}>
      <ChakraProvider>
        <WorldCanvas
          isLoggedIn={pubkey !== null}
          useVoxelAvatar={useVoxelAvatar}
          onToggleAvatarType={setUseVoxelAvatar}
        />
        <TopBar
          pubkey={pubkey}
          onLogin={handleLogin}
          onLogout={handleLogout}
          useVoxelAvatar={useVoxelAvatar}
          onToggleAvatarType={setUseVoxelAvatar}
        />
      </ChakraProvider>
    </AccountsProvider>
  )
}

export default App
