import { Box, Text, Tooltip, VStack } from '@chakra-ui/react'

interface VoiceButtonProps {
  isConnected: boolean
  isConnecting: boolean
  onToggleConnection: () => void
  onToggleMic: () => void
  micEnabled: boolean
  micError: string | null
  participantCount?: number
}

export function VoiceButton({
  isConnected,
  isConnecting,
  onToggleConnection,
  onToggleMic,
  micEnabled,
  micError,
  participantCount = 0,
}: VoiceButtonProps) {
  const getSpeakerTooltip = () => {
    if (isConnecting) return 'Connecting to voice...'
    if (isConnected) return `Voice connected (${participantCount} ${participantCount === 1 ? 'participant' : 'participants'})`
    return 'Voice disconnected - Click to connect'
  }

  const getMicTooltip = () => {
    if (micError) return micError
    if (!isConnected) return 'Connect to voice first'
    if (micEnabled) return 'Microphone ON - Click to mute'
    return 'Microphone OFF - Click to unmute'
  }

  const getMicBg = () => {
    if (micError) return 'rgba(220, 38, 38, 0.3)' // red for error
    if (!isConnected) return 'rgba(80, 80, 80, 0.1)' // gray when voice off
    if (micEnabled) return 'rgba(40, 200, 80, 0.2)' // green when on
    return 'rgba(251, 146, 60, 0.2)' // orange when off but connected
  }

  const getMicBorder = () => {
    if (micError) return 'rgba(220, 38, 38, 0.5)'
    if (!isConnected) return 'rgba(255, 255, 255, 0.1)'
    if (micEnabled) return 'rgba(40, 200, 80, 0.4)'
    return 'rgba(251, 146, 60, 0.4)'
  }

  return (
    <VStack gap={2}>
      {/* Speaker/Voice Connection Button */}
      <Tooltip label={getSpeakerTooltip()} placement="right">
        <Box
          as="button"
          onClick={onToggleConnection}
          w="48px"
          h="48px"
          bg={
            isConnected
              ? 'rgba(40, 200, 80, 0.3)' // green when on
              : isConnecting
              ? 'rgba(200, 200, 40, 0.2)'
              : 'rgba(220, 38, 38, 0.3)' // red when off
          }
          border={`1px solid ${
            isConnected
              ? 'rgba(40, 200, 80, 0.5)'
              : isConnecting
              ? 'rgba(200, 200, 40, 0.4)'
              : 'rgba(220, 38, 38, 0.5)'
          }`}
          _hover={{
            bg: isConnected ? 'rgba(40, 200, 80, 0.4)' : 'rgba(220, 38, 38, 0.4)',
            borderColor: isConnected ? 'rgba(40, 200, 80, 0.6)' : 'rgba(220, 38, 38, 0.6)',
          }}
          _active={{
            bg: 'rgba(60, 60, 60, 0.3)',
          }}
          transition="all 0.1s"
          cursor="pointer"
          display="flex"
          alignItems="center"
          justifyContent="center"
          position="relative"
        >
          <Text fontSize="xl" transition="all 0.3s">
            {isConnecting ? '⏳' : '🔊'}
          </Text>
          {isConnected && participantCount > 0 && (
            <Box
              position="absolute"
              top="-4px"
              right="-4px"
              bg="green.500"
              borderRadius="full"
              w="16px"
              h="16px"
              display="flex"
              alignItems="center"
              justifyContent="center"
              fontSize="10px"
              fontWeight="bold"
              color="white"
            >
              {participantCount > 9 ? '9+' : participantCount}
            </Box>
          )}
        </Box>
      </Tooltip>

      {/* Microphone Button */}
      <Tooltip label={getMicTooltip()} placement="right">
        <Box
          as="button"
          onClick={onToggleMic}
          w="48px"
          h="48px"
          bg={getMicBg()}
          border={`1px solid ${getMicBorder()}`}
          _hover={{
            opacity: isConnected ? 0.8 : 1,
          }}
          _active={{
            bg: 'rgba(60, 60, 60, 0.3)',
          }}
          transition="all 0.1s"
          cursor={isConnected ? 'pointer' : 'not-allowed'}
          display="flex"
          alignItems="center"
          justifyContent="center"
          opacity={isConnected ? 1 : 0.5}
        >
          <Text fontSize="xl" transition="all 0.3s">
            🎤
          </Text>
        </Box>
      </Tooltip>
    </VStack>
  )
}
