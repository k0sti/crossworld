import { Box, VStack, Text } from '@chakra-ui/react'
import { useState, useEffect } from 'react'

interface ModelEntry {
  name: string
  path: string
  type: 'vox' | 'glb'
  size: number
}

interface ModelsIndex {
  generated: string
  count: number
  models: ModelEntry[]
}

interface ModelSelectorProps {
  isVisible: boolean
  onModelSelect?: (modelPath: string, index: number) => void
}

export function ModelSelector({ isVisible, onModelSelect }: ModelSelectorProps) {
  const [selectedIndex, setSelectedIndex] = useState<number>(0)
  const [models, setModels] = useState<ModelEntry[]>([])
  const [loading, setLoading] = useState<boolean>(true)

  // Load models.json on mount
  useEffect(() => {
    const loadModels = async () => {
      try {
        const response = await fetch('/crossworld/assets/models.json')
        if (!response.ok) {
          throw new Error('Failed to load models.json')
        }
        const data: ModelsIndex = await response.json()
        setModels(data.models)
      } catch (error) {
        console.error('Error loading models:', error)
      } finally {
        setLoading(false)
      }
    }

    loadModels()
  }, [])

  if (!isVisible) return null

  const handleModelClick = (index: number) => {
    setSelectedIndex(index)
    if (onModelSelect && models.length > 0) {
      const model = models[index]
      onModelSelect(model.path, index)
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
      minW="150px"
      maxW="200px"
    >
      <VStack spacing={1} align="stretch" maxH="calc(100vh - 80px)" overflowY="auto">
        <Text fontSize="sm" fontWeight="bold" color="white" mb={2}>
          Models ({models.length})
        </Text>
        {loading && (
          <Text fontSize="xs" color="gray.400">Loading models...</Text>
        )}
        {!loading && models.length === 0 && (
          <Text fontSize="xs" color="gray.400">No models found</Text>
        )}
        {models.map((model, index) => (
          <Box
            key={index}
            as="button"
            w="100%"
            px={3}
            py={2}
            bg={selectedIndex === index ? 'rgba(255, 165, 0, 0.3)' : 'rgba(80, 80, 80, 0.3)'}
            borderRadius="sm"
            border={selectedIndex === index ? '2px solid rgba(255, 165, 0, 0.8)' : '1px solid rgba(255, 255, 255, 0.2)'}
            cursor="pointer"
            onClick={() => handleModelClick(index)}
            _hover={{
              bg: selectedIndex === index ? 'rgba(255, 165, 0, 0.4)' : 'rgba(120, 120, 120, 0.4)',
              borderColor: 'white',
            }}
            transition="all 0.1s"
            textAlign="left"
          >
            <Text fontSize="xs" color="white" noOfLines={1} title={model.name}>
              {model.name}
            </Text>
          </Box>
        ))}
      </VStack>
    </Box>
  )
}
