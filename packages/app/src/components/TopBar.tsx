import { Box, Flex } from '@chakra-ui/react'
import { ProfileButton } from './ProfileButton'
import { ConfigPanelType } from './ConfigPanel'
import { NetworkConfigPanel } from './NetworkConfigPanel'
import { ProfilePanel } from './ProfilePanel'

interface TopBarProps {
  pubkey: string | null
  onLogin: (pubkey: string) => void
  onLogout: () => void
  activePanelType: ConfigPanelType
  onCloseAllPanels: () => void
}

export function TopBar({ pubkey, onLogin, activePanelType }: TopBarProps) {
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
          <ProfileButton pubkey={pubkey} onLogin={onLogin} />
        </Flex>
      </Box>

      {/* Network Config Panel */}
      {activePanelType === 'network' && (
        <NetworkConfigPanel />
      )}

      {/* Profile Panel */}
      {activePanelType === 'profile' && (
        <ProfilePanel pubkey={pubkey} />
      )}
    </>
  )
}
