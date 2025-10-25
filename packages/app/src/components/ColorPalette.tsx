import { Box, Grid } from '@chakra-ui/react'
import { useState } from 'react'
import { DAWNBRINGER_32 } from '@crossworld/editor'

interface ColorPaletteProps {
  isVisible: boolean
  onColorSelect?: (color: string, index: number) => void
}

export function ColorPalette({ isVisible, onColorSelect }: ColorPaletteProps) {
  const [selectedIndex, setSelectedIndex] = useState<number>(0)

  if (!isVisible) return null

  const handleColorClick = (index: number) => {
    setSelectedIndex(index)
    if (onColorSelect) {
      onColorSelect(DAWNBRINGER_32[index], index)
    }
  }

  return (
    <Box
      position="fixed"
      top="60px"
      right="0px"
      bottom="0px"
      zIndex={1000}
      bg="rgba(0, 0, 0, 0.8)"
      backdropFilter="blur(8px)"
      borderLeft="1px solid rgba(255, 255, 255, 0.2)"
      p={2}
    >
      <Grid
        templateColumns="repeat(2, 1fr)"
        gap={1}
      >
        {DAWNBRINGER_32.map((color, index) => (
          <Box
            key={index}
            as="button"
            w="24px"
            h="24px"
            bg={color}
            borderRadius="sm"
            border={selectedIndex === index ? '2px solid white' : '1px solid rgba(255, 255, 255, 0.3)'}
            cursor="pointer"
            onClick={() => handleColorClick(index)}
            _hover={{
              transform: 'scale(1.1)',
              borderColor: 'white',
              zIndex: 1,
            }}
            transition="all 0.1s"
            title={`${index}: ${color}`}
          />
        ))}
      </Grid>
    </Box>
  )
}

export function getSelectedColorIndex(): number {
  // This will be managed by the component state
  return 0
}
