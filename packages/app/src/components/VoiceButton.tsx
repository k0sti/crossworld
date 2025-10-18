import { Box, Text, Tooltip } from '@chakra-ui/react'

interface VoiceButtonProps {
  isConnected: boolean
  isConnecting: boolean
  onClick: () => void
  participantCount?: number
}

export function VoiceButton({
  isConnected,
  isConnecting,
  onClick,
  participantCount = 0,
}: VoiceButtonProps) {
  const getIcon = () => {
    if (isConnecting) return '⏳'
    if (isConnected) return '🎧'
    return '🎧'
  }

  const getTooltip = () => {
    if (isConnecting) return 'Connecting to voice...'
    if (isConnected) return `Voice connected (${participantCount} ${participantCount === 1 ? 'participant' : 'participants'})`
    return 'Connect to voice chat'
  }

  return (
    <Tooltip label={getTooltip()} placement="right">
      <Box
        as="button"
        onClick={onClick}
        w="48px"
        h="48px"
        bg={
          isConnected
            ? 'rgba(40, 200, 80, 0.2)'
            : isConnecting
            ? 'rgba(200, 200, 40, 0.2)'
            : 'rgba(80, 80, 80, 0.1)'
        }
        border={`1px solid ${
          isConnected
            ? 'rgba(40, 200, 80, 0.4)'
            : isConnecting
            ? 'rgba(200, 200, 40, 0.4)'
            : 'rgba(255, 255, 255, 0.1)'
        }`}
        _hover={{
          bg: isConnected ? 'rgba(40, 200, 80, 0.3)' : 'rgba(120, 120, 120, 0.2)',
          borderColor: isConnected ? 'rgba(40, 200, 80, 0.6)' : 'rgba(255, 255, 255, 0.2)',
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
          {getIcon()}
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
  )
}
