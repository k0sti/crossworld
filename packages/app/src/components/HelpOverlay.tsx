import { Box, VStack, HStack, Text, Heading, SimpleGrid } from '@chakra-ui/react'

interface HelpOverlayProps {
  isOpen: boolean
  onClose: () => void
}

export function HelpOverlay({ isOpen, onClose }: HelpOverlayProps) {
  if (!isOpen) return null

  return (
    <Box
      position="fixed"
      top={0}
      left={0}
      right={0}
      bottom={0}
      bg="rgba(0, 0, 0, 0.85)"
      zIndex={2000}
      overflowY="auto"
      onClick={onClose}
    >
      <Box
        maxWidth="800px"
        margin="auto"
        mt={8}
        mb={8}
        p={8}
        bg="gray.900"
        borderRadius="lg"
        color="white"
        onClick={(e) => e.stopPropagation()}
      >
        <VStack align="stretch" gap={6}>
          <Heading size="lg">Keyboard Shortcuts</Heading>
          <Text color="gray.400" fontSize="sm">
            Press F1 or ESC to close this help
          </Text>

          {/* Basic Controls */}
          <Box>
            <Heading size="md" mb={3}>Basic Controls</Heading>
            <VStack align="stretch" gap={2}>
              <ShortcutRow keys={['ESC']} description="Close chat / Close panel" />
              <ShortcutRow keys={['ALT', 'C']} description="Toggle chat" />
            </VStack>
          </Box>

          {/* Walk Mode */}
          <Box>
            <Heading size="md" mb={3}>🚶 Walk Mode (Default)</Heading>
            <SimpleGrid columns={2} gap={4}>
              <VStack align="stretch" gap={2}>
                <Text fontWeight="bold" fontSize="sm" color="gray.400">Movement</Text>
                <ShortcutRow keys={['W', 'A', 'S', 'D']} description="Move" />
                <ShortcutRow keys={['SHIFT']} description="Run (2x speed)" />
                <ShortcutRow keys={['SPACE']} description="Jump" />
              </VStack>
              <VStack align="stretch" gap={2}>
                <Text fontWeight="bold" fontSize="sm" color="gray.400">Camera</Text>
                <ShortcutRow keys={['F']} description="Toggle camera view" />
                <ShortcutRow keys={['SCROLL']} description="Zoom in/out" />
              </VStack>
            </SimpleGrid>
          </Box>

          {/* Panel Shortcuts */}
          <Box>
            <Heading size="md" mb={3}>Panel Shortcuts</Heading>
            <SimpleGrid columns={2} gap={4}>
              <VStack align="stretch" gap={2}>
                <ShortcutRow keys={['ALT', 'N']} description="Network config" />
                <ShortcutRow keys={['ALT', 'P']} description="Profile" />
                <ShortcutRow keys={['ALT', 'A']} description="Avatar config" />
              </VStack>
              <VStack align="stretch" gap={2}>
                <ShortcutRow keys={['ALT', 'C']} description="Toggle chat" />
                <ShortcutRow keys={['ALT', 'Q']} description="Logout" />
                <ShortcutRow keys={['ESC']} description="Close panel" />
              </VStack>
            </SimpleGrid>
          </Box>

          {/* Chat Panel */}
          <Box>
            <Heading size="md" mb={3}>💬 Chat Panel</Heading>
            <VStack align="stretch" gap={2}>
              <ShortcutRow keys={['ENTER']} description="Send message" />
              <ShortcutRow keys={['SHIFT', 'ENTER']} description="New line" />
              <ShortcutRow keys={['ESC']} description="Close chat" />
            </VStack>
          </Box>

          {/* Global Shortcuts */}
          <Box>
            <Heading size="md" mb={3}>Global Shortcuts</Heading>
            <SimpleGrid columns={2} gap={4}>
              <VStack align="stretch" gap={2}>
                <ShortcutRow keys={['F1']} description="Toggle this help" />
                <ShortcutRow keys={['F11']} description="Toggle fullscreen" />
                <ShortcutRow keys={['ALT', 'H']} description="Show help" />
              </VStack>
            </SimpleGrid>
          </Box>

          <Text textAlign="center" color="gray.500" fontSize="sm" mt={4}>
            Press ESC or click outside to close
          </Text>
        </VStack>
      </Box>
    </Box>
  )
}

interface ShortcutRowProps {
  keys: string[]
  description: string
}

function ShortcutRow({ keys, description }: ShortcutRowProps) {
  return (
    <HStack gap={2}>
      <HStack gap={1} flexShrink={0} minWidth="120px">
        {keys.map((key, index) => (
          <Box key={index}>
            <Box
              as="kbd"
              px={2}
              py={1}
              bg="gray.700"
              borderRadius="md"
              fontSize="xs"
              fontWeight="bold"
              border="1px solid"
              borderColor="gray.600"
            >
              {key}
            </Box>
            {index < keys.length - 1 && <Text display="inline" mx={1}>+</Text>}
          </Box>
        ))}
      </HStack>
      <Text fontSize="sm" color="gray.300">{description}</Text>
    </HStack>
  )
}
