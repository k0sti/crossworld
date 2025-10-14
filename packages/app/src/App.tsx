import { useState, useMemo } from 'react'
import { ChakraProvider } from '@chakra-ui/react'
import { AccountsProvider } from 'applesauce-react/providers'
import { AccountManager } from 'applesauce-accounts'
import { TopBar } from './components/TopBar'

function App() {
  const [pubkey, setPubkey] = useState<string | null>(null)
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
        <TopBar pubkey={pubkey} onLogin={handleLogin} onLogout={handleLogout} />
      </ChakraProvider>
    </AccountsProvider>
  )
}

export default App
