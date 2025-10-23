import { Box, Flex, IconButton, HStack } from '@chakra-ui/react'
import { FiGlobe, FiInfo } from 'react-icons/fi'
import { ProfileButton } from './ProfileButton'
import { ConfigPanelType } from '../types/config'
import { ReactNode } from 'react'

interface TopBarProps {
  pubkey: string | null
  onLogin: (pubkey: string) => void
  onOpenPanel: (type: ConfigPanelType) => void
  onOpenProfile: () => void
  activePanelType: ConfigPanelType
  centerContent?: ReactNode
}

export function TopBar({ pubkey, onLogin, onOpenPanel, onOpenProfile, activePanelType, centerContent }: TopBarProps) {
  return (
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
        <ProfileButton pubkey={pubkey} onLogin={onLogin} onOpenProfile={onOpenProfile} />

        {centerContent && (
          <Box position="absolute" left="50%" transform="translateX(-50%)">
            {centerContent}
          </Box>
        )}

        <HStack spacing={2}>
          <IconButton
            aria-label="Network settings"
            icon={<FiGlobe />}
            onClick={() => onOpenPanel(activePanelType === 'network' ? null : 'network')}
            variant={activePanelType === 'network' ? 'solid' : 'ghost'}
            size="sm"
          />
          <IconButton
            aria-label="About"
            icon={<FiInfo />}
            onClick={() => onOpenPanel(activePanelType === 'info' ? null : 'info')}
            variant={activePanelType === 'info' ? 'solid' : 'ghost'}
            size="sm"
          />
        </HStack>
      </Flex>
    </Box>
  )
}
