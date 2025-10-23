import { BrowserRouter, Routes, Route } from 'react-router-dom'
import { ChakraProvider } from '@chakra-ui/react'
import { AccountsProvider } from 'applesauce-react/providers'
import { AccountManager } from 'applesauce-accounts'
import { useMemo } from 'react'
import App from './App'
import { EditorApp } from './EditorApp'

export function Root() {
  const accountManager = useMemo(() => new AccountManager(), [])

  return (
    <AccountsProvider manager={accountManager}>
      <ChakraProvider>
        <BrowserRouter basename="/crossworld">
          <Routes>
            <Route path="/" element={<App />} />
            <Route path="/editor" element={<EditorApp />} />
          </Routes>
        </BrowserRouter>
      </ChakraProvider>
    </AccountsProvider>
  )
}
