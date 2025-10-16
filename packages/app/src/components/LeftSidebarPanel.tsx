import { Box, VStack, Text, Divider } from '@chakra-ui/react'
import { ConfigPanelType } from './ConfigPanel'

interface SidebarIconProps {
  icon: string
  onClick: () => void
  isActive?: boolean
}

function SidebarIcon({ icon, onClick, isActive }: SidebarIconProps) {
  return (
    <Box
      as="button"
      onClick={onClick}
      w="48px"
      h="48px"
      bg={isActive ? "rgba(120, 120, 120, 0.2)" : "rgba(80, 80, 80, 0.1)"}
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
  onLogout: () => void
  activePanelType: ConfigPanelType
  isEditMode: boolean
  onToggleEditMode: (isEditMode: boolean) => void
  isChatOpen: boolean
  onToggleChat: () => void
}

export function LeftSidebarPanel({
  onOpenPanel,
  onLogout,
  activePanelType,
  isEditMode,
  onToggleEditMode,
  isChatOpen,
  onToggleChat
}: LeftSidebarPanelProps) {
  const handleLogout = () => {
    onOpenPanel(null)
    onLogout()
  }

  const handleOpenPanel = (type: ConfigPanelType) => {
    // If clicking the same panel, close it; otherwise open the new panel
    if (activePanelType === type) {
      onOpenPanel(null)
    } else {
      onOpenPanel(type)
    }
  }

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
        <SidebarIcon
          icon={isEditMode ? "✏️" : "🚶"}
          onClick={() => onToggleEditMode(!isEditMode)}
          isActive={isEditMode}
        />

        <Divider borderColor="rgba(255, 255, 255, 0.1)" my={1} />

        {/* Config Icons */}
        <SidebarIcon
          icon="🌐"
          onClick={() => handleOpenPanel('network')}
          isActive={activePanelType === 'network'}
        />
        <SidebarIcon
          icon="👤"
          onClick={() => handleOpenPanel('profile')}
          isActive={activePanelType === 'profile'}
        />
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
          icon="🚪"
          onClick={handleLogout}
        />
      </VStack>
    </Box>
  )
}
