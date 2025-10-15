import { Box, Flex, IconButton, Text } from '@chakra-ui/react'
import { useState } from 'react'
import { ProfileButton } from './ProfileButton'
import { ConfigPanel, ConfigPanelType } from './ConfigPanel'
import { NetworkConfigPanel } from './NetworkConfigPanel'
import { ProfilePanel } from './ProfilePanel'

interface TopBarProps {
  pubkey: string | null
  onLogin: (pubkey: string) => void
  onLogout: () => void
  useVoxelAvatar: boolean
  onToggleAvatarType: (useVoxel: boolean) => void
}

export function TopBar({ pubkey, onLogin, onLogout, useVoxelAvatar, onToggleAvatarType }: TopBarProps) {
  const [showConfigPanel, setShowConfigPanel] = useState(false)
  const [activePanelType, setActivePanelType] = useState<ConfigPanelType>(null)

  const handleOpenPanel = (type: ConfigPanelType) => {
    setActivePanelType(type)
  }

  const handleCloseAllPanels = () => {
    setShowConfigPanel(false)
    setActivePanelType(null)
  }

  return (
    <>
      <Box
        as="header"
        position="fixed"
        top={0}
        left={0}
        right={0}
        zIndex={1000}
        bg="rgba(0, 0, 0, 0.5)"
        backdropFilter="blur(8px)"
        borderBottom="1px solid rgba(255, 255, 255, 0.1)"
        px={3}
        py={2}
      >
        <Flex justify="space-between" align="center">
          <ProfileButton pubkey={pubkey} onLogin={onLogin} onLogout={onLogout} />
          <IconButton
            aria-label="Open configuration"
            icon={<Text fontSize="xl">⚙️</Text>}
            size="md"
            variant="ghost"
            colorScheme="whiteAlpha"
            color="white"
            _hover={{ bg: 'rgba(255, 255, 255, 0.1)' }}
            onClick={() => setShowConfigPanel(true)}
          />
        </Flex>
      </Box>

      {/* Config Panel */}
      {showConfigPanel && (
        <ConfigPanel
          onClose={handleCloseAllPanels}
          onOpenPanel={handleOpenPanel}
          useVoxelAvatar={useVoxelAvatar}
          onToggleAvatarType={onToggleAvatarType}
          onLogout={onLogout}
          activePanelType={activePanelType}
        />
      )}

      {/* Network Config Panel */}
      {activePanelType === 'network' && (
        <NetworkConfigPanel onClose={handleCloseAllPanels} />
      )}

      {/* Profile Panel */}
      {activePanelType === 'profile' && (
        <ProfilePanel pubkey={pubkey} onClose={handleCloseAllPanels} />
      )}
    </>
  )
}
