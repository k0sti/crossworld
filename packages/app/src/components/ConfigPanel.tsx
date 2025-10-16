import { Box, SimpleGrid, Text } from '@chakra-ui/react'

export type ConfigPanelType = 'network' | 'profile' | 'avatar' | null

interface ConfigIconProps {
  icon: string
  onClick: () => void
}

function ConfigIcon({ icon, onClick }: ConfigIconProps) {
  return (
    <Box
      as="button"
      onClick={onClick}
      aspectRatio={1}
      bg="rgba(80, 80, 80, 0.1)"
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
      <Text fontSize="2xl" transition="all 0.3s">{icon}</Text>
    </Box>
  )
}

interface ConfigPanelProps {
  onClose: () => void
  onOpenPanel: (type: ConfigPanelType) => void
  onLogout: () => void
  activePanelType: ConfigPanelType
}

export function ConfigPanel({ onClose, onOpenPanel, onLogout, activePanelType }: ConfigPanelProps) {
  const handleLogout = () => {
    onOpenPanel(null)
    onLogout()
    onClose()
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
      left="50%"
      transform="translateX(-50%)"
      zIndex={1500}
      bg="rgba(0, 0, 0, 0.1)"
      backdropFilter="blur(8px)"
      _before={{
        content: '""',
        position: 'absolute',
        top: 0,
        left: 0,
        right: 0,
        bottom: 0,
        background: `
          radial-gradient(ellipse at 20% 30%, rgba(255, 255, 255, 0.03) 0%, transparent 50%),
          radial-gradient(ellipse at 80% 70%, rgba(255, 255, 255, 0.03) 0%, transparent 50%),
          repeating-linear-gradient(
            45deg,
            transparent,
            transparent 10px,
            rgba(255, 255, 255, 0.01) 10px,
            rgba(255, 255, 255, 0.01) 20px
          )
        `,
        pointerEvents: 'none',
        zIndex: -1,
      }}
    >
      <SimpleGrid columns={4} gap={0}>
        <ConfigIcon
          icon="🌐"
          onClick={() => handleOpenPanel('network')}
        />
        <ConfigIcon
          icon="👤"
          onClick={() => handleOpenPanel('profile')}
        />
        <ConfigIcon
          icon="🎭"
          onClick={() => handleOpenPanel('avatar')}
        />
        <ConfigIcon
          icon="🚪"
          onClick={handleLogout}
        />
        {/* Placeholder icons for future features */}
        {Array.from({ length: 8 }).map((_, i) => (
          <Box
            key={i}
            aspectRatio={1}
            bg="rgba(40, 40, 40, 0.05)"
            border="1px solid rgba(255, 255, 255, 0.05)"
          />
        ))}
      </SimpleGrid>
    </Box>
  )
}
