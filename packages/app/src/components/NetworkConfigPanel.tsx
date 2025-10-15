import { Box, VStack, Text, Input, Button, HStack, IconButton } from '@chakra-ui/react'
import { useState } from 'react'
import { DEFAULT_RELAYS } from '../config'

interface NetworkConfigPanelProps {
  onClose: () => void
}

export function NetworkConfigPanel({ onClose }: NetworkConfigPanelProps) {
  const [relays, setRelays] = useState<string[]>([...DEFAULT_RELAYS])
  const [newRelay, setNewRelay] = useState('')

  const handleAddRelay = () => {
    if (newRelay.trim() && newRelay.startsWith('wss://')) {
      setRelays([...relays, newRelay.trim()])
      setNewRelay('')
    }
  }

  const handleRemoveRelay = (index: number) => {
    setRelays(relays.filter((_, i) => i !== index))
  }

  return (
    <Box
      position="fixed"
      top="220px"
      left="50%"
      transform="translateX(-50%)"
      zIndex={1500}
      bg="rgba(0, 0, 0, 0.1)"
      backdropFilter="blur(8px)"
      p={4}
      minW="400px"
      maxW="500px"
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
      <VStack align="stretch" gap={3}>
        <Text fontSize="md" fontWeight="semibold" color="white">🌐 Network</Text>

        {relays.map((relay, index) => (
          <HStack key={index} bg="rgba(255, 255, 255, 0.05)" p={2} borderRadius="md">
            <Text fontSize="sm" color="white" flex={1} fontFamily="monospace">{relay}</Text>
            <IconButton
              aria-label="Remove relay"
              icon={<Text>❌</Text>}
              size="xs"
              variant="ghost"
              colorScheme="red"
              onClick={() => handleRemoveRelay(index)}
            />
          </HStack>
        ))}

        <HStack>
          <Input
            placeholder="wss://relay.example.com"
            value={newRelay}
            onChange={(e) => setNewRelay(e.target.value)}
            onKeyPress={(e) => e.key === 'Enter' && handleAddRelay()}
            bg="rgba(255, 255, 255, 0.05)"
            border="1px solid rgba(255, 255, 255, 0.1)"
            color="white"
            _placeholder={{ color: 'whiteAlpha.500' }}
            fontSize="sm"
          />
          <Button
            onClick={handleAddRelay}
            colorScheme="blue"
            size="sm"
          >
            Add
          </Button>
        </HStack>

        <Text fontSize="xs" color="whiteAlpha.600" mt={2}>
          Changes will take effect on next reconnection
        </Text>
      </VStack>
    </Box>
  )
}
