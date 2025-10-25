import { Box, VStack, Text, HStack } from '@chakra-ui/react'

interface EditModeControlsProps {
  isVisible: boolean
}

export function EditModeControls({ isVisible }: EditModeControlsProps) {
  if (!isVisible) return null

  return (
    <Box
      position="fixed"
      bottom="20px"
      left="60px"
      zIndex={1000}
      bg="rgba(0, 0, 0, 0.85)"
      backdropFilter="blur(8px)"
      borderRadius="md"
      border="1px solid rgba(255, 255, 255, 0.2)"
      p={3}
      maxW="300px"
    >
      <VStack spacing={2} align="stretch">
        <Text fontSize="sm" fontWeight="bold" color="white" mb={1}>
          Edit Mode Controls
        </Text>

        <HStack spacing={2} fontSize="xs" color="whiteAlpha.900">
          <Text fontWeight="bold" minW="80px">Left Click:</Text>
          <Text>Place voxel</Text>
        </HStack>

        <HStack spacing={2} fontSize="xs" color="whiteAlpha.900">
          <Text fontWeight="bold" minW="80px">WASD:</Text>
          <Text>Move camera</Text>
        </HStack>

        <HStack spacing={2} fontSize="xs" color="whiteAlpha.900">
          <Text fontWeight="bold" minW="80px">F / V:</Text>
          <Text>Up / Down</Text>
        </HStack>

        <HStack spacing={2} fontSize="xs" color="whiteAlpha.900">
          <Text fontWeight="bold" minW="80px">Right Drag:</Text>
          <Text>Look around</Text>
        </HStack>
      </VStack>
    </Box>
  )
}
