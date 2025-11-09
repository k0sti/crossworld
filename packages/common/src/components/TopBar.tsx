import { Box, Flex, IconButton, Text, HStack, Switch } from '@chakra-ui/react'
import { FiMenu } from 'react-icons/fi'
import { ProfileButton } from './ProfileButton'
import { ConfigPanelType } from '../types/config'
import { ReactNode } from 'react'

export type MainMode = 'walk' | 'edit' | 'placement'

interface TopBarProps {
  pubkey: string | null
  onLogin: (pubkey: string) => void
  onOpenPanel: (type: ConfigPanelType) => void
  onOpenProfile: () => void
  activePanelType: ConfigPanelType
  centerContent?: ReactNode
  mainMode?: MainMode
  onModeChange?: (mode: MainMode) => void
  speechEnabled?: boolean
  onSpeechEnabledChange?: (enabled: boolean) => void
}

export function TopBar({ pubkey, onLogin, onOpenPanel, onOpenProfile, activePanelType, centerContent, mainMode = 'walk', onModeChange, speechEnabled, onSpeechEnabledChange }: TopBarProps) {
  const handleMenuClick = () => {
    // Toggle config panel
    if (activePanelType === 'config') {
      onOpenPanel(null)
    } else {
      onOpenPanel('config')
    }
  }

  const modes: Array<{ mode: MainMode; emoji: string; label: string }> = [
    { mode: 'walk', emoji: '🚶', label: 'Walk' },
    { mode: 'edit', emoji: '🏗️', label: 'Edit' },
    { mode: 'placement', emoji: '📦', label: 'Placement' }
  ]

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
          {/* Mode Radio Buttons */}
          {pubkey && onModeChange && (
            <HStack spacing={1}>
              {modes.map(({ mode, emoji, label }) => (
                <Box
                  key={mode}
                  as="button"
                  onClick={() => onModeChange(mode)}
                  px={3}
                  py={2}
                  bg={mainMode === mode ? "rgba(255, 165, 0, 0.2)" : "rgba(80, 80, 80, 0.1)"}
                  border="1px solid"
                  borderColor={mainMode === mode ? "rgba(255, 165, 0, 0.5)" : "rgba(255, 255, 255, 0.1)"}
                  _hover={{
                    bg: mainMode === mode ? 'rgba(255, 165, 0, 0.3)' : 'rgba(120, 120, 120, 0.2)',
                    borderColor: mainMode === mode ? 'rgba(255, 165, 0, 0.6)' : 'rgba(255, 255, 255, 0.2)'
                  }}
                  _active={{
                    bg: 'rgba(60, 60, 60, 0.3)',
                  }}
                  transition="all 0.1s"
                  cursor="pointer"
                  display="flex"
                  alignItems="center"
                  gap={2}
                  borderRadius="md"
                >
                  <Text fontSize="md">{emoji}</Text>
                  <Text fontSize="sm" color="white">{label}</Text>
                </Box>
              ))}
            </HStack>
          )}

          {/* Speech Toggle */}
          {pubkey && onSpeechEnabledChange && (
            <HStack
              spacing={2}
              px={3}
              py={2}
              bg="rgba(80, 80, 80, 0.1)"
              border="1px solid rgba(255, 255, 255, 0.1)"
              borderRadius="md"
            >
              <Text fontSize="sm" color="white">Speech</Text>
              <Switch
                isChecked={speechEnabled}
                onChange={(e) => onSpeechEnabledChange(e.target.checked)}
                size="sm"
                colorScheme="purple"
              />
            </HStack>
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
