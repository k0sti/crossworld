import { useEffect, useCallback } from 'react'
import { Box, VStack, Text, Divider } from '@chakra-ui/react'
import { ConfigPanelType } from './ConfigPanel'
import { VoiceButton } from './VoiceButton'
import { ENABLE_EDIT_MODE, ENABLE_VOICE_CHAT } from '../constants/features'

interface SidebarIconProps {
  icon: string
  onClick: () => void
  isActive?: boolean
  activeBgColor?: string
}

function SidebarIcon({ icon, onClick, isActive, activeBgColor }: SidebarIconProps) {
  const defaultActiveBg = "rgba(120, 120, 120, 0.2)"
  const activeBg = isActive ? (activeBgColor || defaultActiveBg) : "rgba(80, 80, 80, 0.1)"

  return (
    <Box
      as="button"
      onClick={onClick}
      w="48px"
      h="48px"
      bg={activeBg}
      border="1px solid rgba(255, 255, 255, 0.1)"
      _hover={{
        bg: 'rgba(120, 120, 120, 0.2)',
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
    >
      <Text fontSize="xl" transition="all 0.3s">{icon}</Text>
    </Box>
  )
}

interface LeftSidebarPanelProps {
  onOpenPanel: (type: ConfigPanelType) => void
  activePanelType: ConfigPanelType
  isEditMode: boolean
  onToggleEditMode: (isEditMode: boolean) => void
  isChatOpen: boolean
  onToggleChat: () => void
  isClientListOpen: boolean
  onToggleClientList: () => void
  // Voice props
  voiceConnected: boolean
  voiceConnecting: boolean
  micEnabled: boolean
  participantCount: number
  voiceError: string | null
  onToggleVoice: () => void
  onToggleMic: () => void
}

export function LeftSidebarPanel({
  onOpenPanel,
  activePanelType,
  isEditMode,
  onToggleEditMode,
  isChatOpen,
  onToggleChat,
  isClientListOpen,
  onToggleClientList,
  voiceConnected,
  voiceConnecting,
  micEnabled,
  participantCount,
  voiceError,
  onToggleVoice,
  onToggleMic,
}: LeftSidebarPanelProps) {
  const handleOpenPanel = useCallback((type: ConfigPanelType) => {
    // If clicking the same panel, close it; otherwise open the new panel
    if (activePanelType === type) {
      onOpenPanel(null)
    } else {
      onOpenPanel(type)
    }
  }, [activePanelType, onOpenPanel])

  // Keyboard shortcuts for sidebar buttons
  useEffect(() => {
    const handleKeyPress = (e: KeyboardEvent) => {
      // Ignore if typing in input fields
      if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) {
        return
      }

      // Ignore if edit mode is active (WASD keys used for camera there)
      if (isEditMode) {
        return
      }

      // Build action list based on visible buttons
      const actions: Array<() => void> = []

      // 1. Edit mode toggle (if enabled)
      if (ENABLE_EDIT_MODE) {
        actions.push(() => onToggleEditMode(!isEditMode))
      }

      // 2. Avatar panel
      actions.push(() => handleOpenPanel('avatar'))

      // 3. Chat
      actions.push(onToggleChat)

      // 4. Client list
      actions.push(onToggleClientList)

      // 5. Voice (if enabled)
      if (ENABLE_VOICE_CHAT) {
        actions.push(onToggleVoice)
      }

      // 6. Mic (if voice connected)
      if (ENABLE_VOICE_CHAT && voiceConnected) {
        actions.push(onToggleMic)
      }

      // Map number keys to actions
      const key = e.key
      const num = parseInt(key)
      if (!isNaN(num) && num >= 1 && num <= actions.length) {
        e.preventDefault()
        actions[num - 1]()
      }
    }

    window.addEventListener('keydown', handleKeyPress)

    return () => {
      window.removeEventListener('keydown', handleKeyPress)
    }
  }, [
    isEditMode,
    activePanelType,
    isChatOpen,
    isClientListOpen,
    voiceConnected,
    onToggleEditMode,
    onToggleChat,
    onToggleClientList,
    onToggleVoice,
    onToggleMic,
    handleOpenPanel,
  ])

  return (
    <Box
      position="fixed"
      top="60px"
      left={0}
      bottom={0}
      zIndex={999}
      w="48px"
      bg="rgba(0, 0, 0, 0.5)"
      backdropFilter="blur(8px)"
      borderRight="1px solid rgba(255, 255, 255, 0.1)"
    >
      <VStack spacing={0} align="stretch">
        {/* Walk/Edit Mode Toggle */}
        {ENABLE_EDIT_MODE && (
          <>
            <SidebarIcon
              icon={isEditMode ? "✏️" : "🚶"}
              onClick={() => onToggleEditMode(!isEditMode)}
              isActive={isEditMode}
            />
            <Divider borderColor="rgba(255, 255, 255, 0.1)" my={1} />
          </>
        )}

        {/* Config Icons */}
        <SidebarIcon
          icon="🎭"
          onClick={() => handleOpenPanel('avatar')}
          isActive={activePanelType === 'avatar'}
        />
        <SidebarIcon
          icon="💬"
          onClick={onToggleChat}
          isActive={isChatOpen}
        />
        <SidebarIcon
          icon="👥"
          onClick={onToggleClientList}
          isActive={isClientListOpen}
        />

        {ENABLE_VOICE_CHAT && (
          <>
            <Divider borderColor="rgba(255, 255, 255, 0.1)" my={1} />

            {/* Voice Chat with integrated mic button */}
            <VoiceButton
              isConnected={voiceConnected}
              isConnecting={voiceConnecting}
              onToggleConnection={onToggleVoice}
              onToggleMic={onToggleMic}
              micEnabled={micEnabled}
              micError={voiceError}
              participantCount={participantCount}
            />

            <Divider borderColor="rgba(255, 255, 255, 0.1)" my={1} />
          </>
        )}

      </VStack>
    </Box>
  )
}
