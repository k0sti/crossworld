import { Box, Flex, IconButton, Text, HStack } from '@chakra-ui/react'
import { FiMenu } from 'react-icons/fi'
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
  isEditMode?: boolean
  onToggleEditMode?: () => void
}

export function TopBar({ pubkey, onLogin, onOpenPanel, onOpenProfile, activePanelType, centerContent, isEditMode, onToggleEditMode }: TopBarProps) {
  const handleMenuClick = () => {
    // Toggle config panel
    if (activePanelType === 'config') {
      onOpenPanel(null)
    } else {
      onOpenPanel('config')
    }
  }

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
          {/* Edit Mode Toggle */}
          {pubkey && isEditMode !== undefined && onToggleEditMode && (
            <Box
              as="button"
              onClick={onToggleEditMode}
              w="40px"
              h="40px"
              bg={isEditMode ? "rgba(255, 165, 0, 0.2)" : "rgba(80, 80, 80, 0.1)"}
              border="1px solid rgba(255, 255, 255, 0.1)"
              _hover={{
                bg: isEditMode ? 'rgba(255, 165, 0, 0.3)' : 'rgba(120, 120, 120, 0.2)',
                borderColor: 'rgba(255, 255, 255, 0.2)'
              }}
              _active={{
                bg: 'rgba(60, 60, 60, 0.3)',
              }}
              transition="all 0.1s"
              cursor="pointer"
              display="flex"
              alignItems="center"
              justifyContent="center"
              borderRadius="md"
            >
              <Text fontSize="lg">{isEditMode ? '🏗️' : '🚶'}</Text>
            </Box>
          )}

          <IconButton
            aria-label="Menu"
            icon={<FiMenu />}
            onClick={handleMenuClick}
            variant="ghost"
            size="sm"
          />
        </HStack>
      </Flex>
    </Box>
  )
}
