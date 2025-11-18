import { Box, Grid, VStack, Text, Accordion, AccordionItem, AccordionButton, AccordionPanel, AccordionIcon } from '@chakra-ui/react'
import { useState, useEffect } from 'react'

interface Material {
  index: number;
  id: string;
  color: string;
  description: string;
}

interface MaterialsData {
  generated: string;
  count: number;
  materials: Material[];
}

interface ColorPaletteProps {
  isVisible: boolean
  onColorSelect?: (color: string, index: number) => void
}

export function ColorPalette({ isVisible, onColorSelect }: ColorPaletteProps) {
  const [selectedIndex, setSelectedIndex] = useState<number>(128) // Default to black
  const [materials, setMaterials] = useState<Material[]>([])
  const [textureUrls, setTextureUrls] = useState<Map<number, string>>(new Map())

  // Load materials.json
  useEffect(() => {
    const loadMaterials = async () => {
      try {
        const response = await fetch('/crossworld/assets/materials.json')
        if (response.ok) {
          const data: MaterialsData = await response.json()
          setMaterials(data.materials)

          // Pre-load texture URLs for material range (32-127)
          const urls = new Map<number, string>()
          data.materials.forEach(mat => {
            if (mat.index >= 32 && mat.index <= 127) {
              urls.set(mat.index, `/crossworld/assets/textures5/${mat.id}.webp`)
            }
          })
          setTextureUrls(urls)
        }
      } catch (error) {
        console.error('Failed to load materials:', error)
      }
    }

    if (isVisible) {
      loadMaterials()
    }
  }, [isVisible])

  if (!isVisible) return null

  const handleColorClick = (materialIndex: number) => {
    setSelectedIndex(materialIndex)
    const material = materials.find(m => m.index === materialIndex)
    if (onColorSelect && material) {
      const color = material.color
      onColorSelect(color, materialIndex)
    }
  }

  const renderMaterialBox = (mat: Material, showTexture: boolean = false) => {
    const isSelected = selectedIndex === mat.index
    // Convert RGBA format (#AARRGGBB) to RGB format (#RRGGBB) for CSS
    let colorHex = mat.color.startsWith('#') ? mat.color : '#CCCCCC'
    if (colorHex.length === 9) {
      // Strip alpha channel: #AARRGGBB -> #RRGGBB
      colorHex = '#' + colorHex.substring(3)
    }

    return (
      <Box
        key={mat.index}
        as="button"
        w="40px"
        h="40px"
        bg={showTexture ? 'transparent' : colorHex}
        backgroundImage={showTexture && textureUrls.has(mat.index)
          ? `url(${textureUrls.get(mat.index)})`
          : undefined}
        backgroundSize="cover"
        backgroundPosition="center"
        borderRadius="sm"
        border={isSelected ? '2px solid white' : '1px solid rgba(255, 255, 255, 0.3)'}
        cursor="pointer"
        onClick={() => handleColorClick(mat.index)}
        _hover={{
          transform: 'scale(1.05)',
          borderColor: 'white',
          zIndex: 1,
        }}
        transition="all 0.1s"
        title={`${mat.index}: ${mat.id}\n${mat.description}`}
        position="relative"
      >
        <Text
          fontSize="8px"
          fontWeight="bold"
          color="white"
          textShadow="0 0 3px black, 0 0 3px black"
          position="absolute"
          bottom="1px"
          right="2px"
        >
          {mat.index}
        </Text>
      </Box>
    )
  }

  // Get materials by range
  const transparentMaterials = materials.filter(m => m.index >= 0 && m.index <= 31)
  const materialRangeMaterials = materials.filter(m => m.index >= 32 && m.index <= 127)
  const colorPaletteMaterials = materials.filter(m => m.index >= 128 && m.index <= 255)

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
      overflowY="auto"
      width="200px"
    >
      <VStack spacing={2} align="stretch">
        <Text color="white" fontSize="sm" fontWeight="bold" mb={2}>
          Material Palette
        </Text>

        <Accordion allowMultiple defaultIndex={[0, 1, 2]}>
          {/* Transparent Range: 0-31 */}
          <AccordionItem border="1px solid rgba(255, 255, 255, 0.2)" borderRadius="md" mb={2}>
            <AccordionButton bg="rgba(128, 128, 255, 0.2)" _hover={{ bg: 'rgba(128, 128, 255, 0.3)' }}>
              <Box flex="1" textAlign="left">
                <Text color="white" fontSize="xs" fontWeight="bold">
                  Transparent (0-31)
                </Text>
              </Box>
              <AccordionIcon color="white" />
            </AccordionButton>
            <AccordionPanel pb={2} bg="rgba(0, 0, 0, 0.3)">
              <Grid templateColumns="repeat(4, 1fr)" gap={1}>
                {transparentMaterials.map(mat => renderMaterialBox(mat, false))}
              </Grid>
            </AccordionPanel>
          </AccordionItem>

          {/* Material Range: 32-127 (with textures) */}
          <AccordionItem border="1px solid rgba(255, 255, 255, 0.2)" borderRadius="md" mb={2}>
            <AccordionButton bg="rgba(139, 69, 19, 0.4)" _hover={{ bg: 'rgba(139, 69, 19, 0.5)' }}>
              <Box flex="1" textAlign="left">
                <Text color="white" fontSize="xs" fontWeight="bold">
                  Materials (32-127)
                </Text>
              </Box>
              <AccordionIcon color="white" />
            </AccordionButton>
            <AccordionPanel pb={2} bg="rgba(0, 0, 0, 0.3)">
              <Grid templateColumns="repeat(4, 1fr)" gap={1}>
                {materialRangeMaterials.map(mat => renderMaterialBox(mat, true))}
              </Grid>
            </AccordionPanel>
          </AccordionItem>

          {/* Color Palette: 128-255 */}
          <AccordionItem border="1px solid rgba(255, 255, 255, 0.2)" borderRadius="md">
            <AccordionButton bg="rgba(255, 128, 128, 0.2)" _hover={{ bg: 'rgba(255, 128, 128, 0.3)' }}>
              <Box flex="1" textAlign="left">
                <Text color="white" fontSize="xs" fontWeight="bold">
                  Colors (128-255)
                </Text>
              </Box>
              <AccordionIcon color="white" />
            </AccordionButton>
            <AccordionPanel pb={2} bg="rgba(0, 0, 0, 0.3)">
              <Grid templateColumns="repeat(4, 1fr)" gap={1}>
                {colorPaletteMaterials.map(mat => renderMaterialBox(mat, false))}
              </Grid>
            </AccordionPanel>
          </AccordionItem>
        </Accordion>
      </VStack>
    </Box>
  )
}

export function getSelectedColorIndex(): number {
  // This will be managed by the component state
  return 0
}
