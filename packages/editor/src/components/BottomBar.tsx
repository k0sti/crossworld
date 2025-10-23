import { Box, HStack, Grid, VStack, Text } from '@chakra-ui/react'

interface BottomBarProps {
  palette: string[]
  selectedColor: string
  selectedColorIndex: number
  onColorSelect: (color: string, index: number) => void
  onColorChange: (index: number, newColor: string) => void
  cursorPosition?: { x: number; y: number; z: number } | null
}

export function BottomBar({
  palette,
  selectedColor,
  selectedColorIndex,
  onColorSelect,
  cursorPosition,
}: BottomBarProps) {
  const handleColorClick = (color: string, index: number) => {
    onColorSelect(color, index)
  }

  return (
    <Box
      position="fixed"
      bottom={0}
      left={0}
      right={0}
      bg="rgba(0, 0, 0, 0.8)"
      backdropFilter="blur(8px)"
      borderTop="1px solid rgba(255, 255, 255, 0.1)"
      p={1}
      zIndex={1000}
    >
      <HStack spacing={2} justify="center" align="center">
        {/* Color Grid - 8 columns x 4 rows */}
        <Grid
          templateColumns="repeat(8, 1fr)"
          templateRows="repeat(4, 1fr)"
          gap={0.5}
          maxW="600px"
        >
          {palette.slice(0, 32).map((color, index) => (
            <Box
              key={index}
              as="button"
              w="24px"
              h="24px"
              bg={color}
              borderRadius="sm"
              border={selectedColorIndex === index ? '2px solid white' : '1px solid rgba(255, 255, 255, 0.3)'}
              cursor="pointer"
              onClick={() => handleColorClick(color, index)}
              _hover={{
                transform: 'scale(1.1)',
                borderColor: 'white',
                zIndex: 10,
              }}
              transition="all 0.1s"
              title={`${color} (${index})`}
            />
          ))}
        </Grid>

        {/* Selected Color Indicator */}
        <HStack spacing={2} minW="150px">
          <Box
            w="40px"
            h="40px"
            bg={selectedColor}
            borderRadius="md"
            border="2px solid white"
          />
          <VStack align="start" spacing={0}>
            <Text fontSize="xs" color="whiteAlpha.700">Selected</Text>
            <Text fontSize="xs" fontFamily="mono" color="white">
              {selectedColor}
            </Text>
            <Text fontSize="xs" color="whiteAlpha.600">
              Index: {selectedColorIndex}
            </Text>
          </VStack>
        </HStack>

        {/* Cursor Position */}
        {cursorPosition && (
          <VStack align="end" spacing={0} minW="100px">
            <Text fontSize="xs" color="whiteAlpha.700">Cursor</Text>
            <Text fontSize="xs" fontFamily="mono" color="white">
              X: {cursorPosition.x}
            </Text>
            <Text fontSize="xs" fontFamily="mono" color="white">
              Y: {cursorPosition.y}
            </Text>
            <Text fontSize="xs" fontFamily="mono" color="white">
              Z: {cursorPosition.z}
            </Text>
          </VStack>
        )}
      </HStack>
    </Box>
  )
}
