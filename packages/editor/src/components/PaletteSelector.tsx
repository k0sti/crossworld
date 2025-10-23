import { Box, VStack, HStack, Select, Text, Grid, Button } from '@chakra-ui/react'
import { useState, useMemo } from 'react'
import { Screen } from '@crossworld/common'
import { generateHSVPalette } from '../palettes/hsv'
import { getDawnbringerPalette, getDawnbringerSizes } from '../palettes/dawnbringer'

export type PaletteSource = 'hsv' | 'dawnbringer'

interface PaletteSelectorProps {
  isOpen: boolean
  onClose: () => void
  selectedColor: string
  onColorSelect: (color: string) => void
}

export function PaletteSelector({ isOpen, onClose, selectedColor, onColorSelect }: PaletteSelectorProps) {
  const [paletteSource, setPaletteSource] = useState<PaletteSource>('hsv')
  const [paletteSize, setPaletteSize] = useState(16)

  // Generate palette based on source
  const palette = useMemo(() => {
    if (paletteSource === 'hsv') {
      return generateHSVPalette(paletteSize)
    } else {
      return getDawnbringerPalette(paletteSize as 16 | 32)
    }
  }, [paletteSource, paletteSize])

  // Get available sizes based on source
  const availableSizes = useMemo(() => {
    if (paletteSource === 'hsv') {
      return [8, 16, 32, 64]
    } else {
      return getDawnbringerSizes()
    }
  }, [paletteSource])

  // Reset size when changing source
  const handleSourceChange = (newSource: PaletteSource) => {
    setPaletteSource(newSource)
    if (newSource === 'dawnbringer') {
      setPaletteSize(16)
    } else {
      setPaletteSize(16)
    }
  }

  return (
    <Screen
      isOpen={isOpen}
      onClose={onClose}
      title="Palette Selector"
      actions={
        <>
          <Button
            size="sm"
            colorScheme="blue"
            onClick={onClose}
          >
            Okay
          </Button>
          <Button
            size="sm"
            variant="ghost"
            onClick={onClose}
            color="whiteAlpha.700"
            _hover={{ color: 'white' }}
          >
            Close
          </Button>
        </>
      }
    >
      <VStack spacing={4} align="stretch">
        {/* Source Selector */}
        <Box>
          <Text fontSize="sm" fontWeight="semibold" mb={2} color="white">
            Palette Source
          </Text>
          <Select
            value={paletteSource}
            onChange={(e) => handleSourceChange(e.target.value as PaletteSource)}
            bg="rgba(255, 255, 255, 0.05)"
            border="1px solid rgba(255, 255, 255, 0.1)"
            color="white"
          >
            <option value="hsv">HSV Palette</option>
            <option value="dawnbringer">DawnBringer Palette</option>
          </Select>
        </Box>

        {/* Size Selector */}
        <Box>
          <Text fontSize="sm" fontWeight="semibold" mb={2} color="white">
            Palette Size
          </Text>
          <Select
            value={paletteSize}
            onChange={(e) => setPaletteSize(Number(e.target.value))}
            bg="rgba(255, 255, 255, 0.05)"
            border="1px solid rgba(255, 255, 255, 0.1)"
            color="white"
          >
            {availableSizes.map(size => (
              <option key={size} value={size}>{size} colors</option>
            ))}
          </Select>
        </Box>

        {/* Color Grid */}
        <Box>
          <Text fontSize="sm" fontWeight="semibold" mb={2} color="white">
            Colors
          </Text>
          <Grid
            templateColumns="repeat(8, 1fr)"
            gap={1}
            p={2}
            bg="rgba(0, 0, 0, 0.2)"
            borderRadius="md"
          >
            {palette.map((color, index) => (
              <Box
                key={index}
                as="button"
                aspectRatio={1}
                bg={color}
                borderRadius="sm"
                border={selectedColor === color ? '2px solid white' : '1px solid rgba(255, 255, 255, 0.2)'}
                cursor="pointer"
                onClick={() => onColorSelect(color)}
                _hover={{
                  transform: 'scale(1.1)',
                  borderColor: 'white',
                }}
                transition="all 0.1s"
                title={color}
              />
            ))}
          </Grid>
        </Box>

        {/* Selected Color Display */}
        <HStack spacing={2} p={3} bg="rgba(0, 0, 0, 0.2)" borderRadius="md">
          <Box
            w="40px"
            h="40px"
            bg={selectedColor}
            borderRadius="md"
            border="1px solid rgba(255, 255, 255, 0.2)"
          />
          <VStack align="start" spacing={0}>
            <Text fontSize="xs" color="whiteAlpha.700">Selected Color</Text>
            <Text fontSize="sm" fontWeight="mono" color="white">{selectedColor}</Text>
          </VStack>
        </HStack>
      </VStack>
    </Screen>
  )
}
