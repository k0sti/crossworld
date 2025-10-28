import { Box, Grid, VStack, Text } from '@chakra-ui/react'
import React, { useState } from 'react'
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
      const color = index === -1 ? '' : DAWNBRINGER_32[index]
      onColorSelect(color, index)
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
      <VStack spacing={2} align="stretch">
        {/* Clear/Eraser button */}
        <Box
          as="button"
          w="100%"
          h="28px"
          bg="linear-gradient(135deg, rgba(255,255,255,0.1) 25%, transparent 25%, transparent 50%, rgba(255,255,255,0.1) 50%, rgba(255,255,255,0.1) 75%, transparent 75%, transparent)"
          backgroundSize="8px 8px"
          borderRadius="sm"
          border={selectedIndex === -1 ? '2px solid white' : '1px solid rgba(255, 255, 255, 0.3)'}
          cursor="pointer"
          onClick={() => handleColorClick(-1)}
          _hover={{
            borderColor: 'white',
          }}
          transition="all 0.1s"
          display="flex"
          alignItems="center"
          justifyContent="center"
        >
          <Text fontSize="xs" fontWeight="bold" color="white" textShadow="0 0 2px black">
            CLEAR
          </Text>
        </Box>

        {/* Color Grid - 0-15 in left column, 16-31 in right column */}
        <Grid
          templateColumns="repeat(2, 1fr)"
          gap={1}
        >
          {Array.from({ length: 16 }, (_, i) => {
            const leftIndex = i;
            const rightIndex = i + 16;
            return (
              <React.Fragment key={i}>
                <Box
                  as="button"
                  w="24px"
                  h="24px"
                  bg={DAWNBRINGER_32[leftIndex]}
                  borderRadius="sm"
                  border={selectedIndex === leftIndex ? '2px solid white' : '1px solid rgba(255, 255, 255, 0.3)'}
                  cursor="pointer"
                  onClick={() => handleColorClick(leftIndex)}
                  _hover={{
                    transform: 'scale(1.1)',
                    borderColor: 'white',
                    zIndex: 1,
                  }}
                  transition="all 0.1s"
                  title={`${leftIndex}: ${DAWNBRINGER_32[leftIndex]}`}
                />
                <Box
                  key={rightIndex}
                  as="button"
                  w="24px"
                  h="24px"
                  bg={DAWNBRINGER_32[rightIndex]}
                  borderRadius="sm"
                  border={selectedIndex === rightIndex ? '2px solid white' : '1px solid rgba(255, 255, 255, 0.3)'}
                  cursor="pointer"
                  onClick={() => handleColorClick(rightIndex)}
                  _hover={{
                    transform: 'scale(1.1)',
                    borderColor: 'white',
                    zIndex: 1,
                  }}
                  transition="all 0.1s"
                  title={`${rightIndex}: ${DAWNBRINGER_32[rightIndex]}`}
                />
              </React.Fragment>
            );
          })}
        </Grid>
      </VStack>
    </Box>
  )
}

export function getSelectedColorIndex(): number {
  // This will be managed by the component state
  return 0
}
