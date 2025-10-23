import { useState } from 'react'
import { Box, IconButton } from '@chakra-ui/react'
import { TopBar, ConfigPanelType, ProfilePanel } from '@crossworld/common'
import { CubeEditorView } from '@crossworld/editor'
import { NetworkConfigPanel } from './components/NetworkConfigPanel'
import { InfoPanel } from './components/InfoPanel'
import { FiGlobe } from 'react-icons/fi'

export function EditorApp() {
  const [pubkey, setPubkey] = useState<string | null>(null)
  const [activePanelType, setActivePanelType] = useState<ConfigPanelType>(null)
  const [viewedProfilePubkey, _setViewedProfilePubkey] = useState<string | null>(null)

  const handleLogin = (publicKey: string) => {
    setPubkey(publicKey)
  }

  const handleLogout = async () => {
    setPubkey(null)
  }

  return (
    <Box position="relative" w="100vw" h="100vh" overflow="hidden">
      {/* Top Bar with Nostr login */}
      <TopBar
        pubkey={pubkey}
        onLogin={handleLogin}
        onOpenPanel={setActivePanelType}
        onOpenProfile={() => setActivePanelType('profile')}
        activePanelType={activePanelType}
        centerContent={
          <IconButton
            aria-label="Open world"
            icon={<FiGlobe />}
            onClick={() => window.location.href = '/'}
            variant="ghost"
            size="sm"
            title="3D World"
          />
        }
      />

      {/* Cube Editor View */}
      <CubeEditorView />

      {/* Config Panels */}
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
      />
    </Box>
  )
}
