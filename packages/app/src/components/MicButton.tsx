import { Box, Text, Tooltip } from '@chakra-ui/react'
import { keyframes } from '@emotion/react'

interface MicButtonProps {
  micEnabled: boolean
  speaking: boolean
  onClick: () => void
}

const pulse = keyframes`
  0% {
    box-shadow: 0 0 0 0 rgba(40, 200, 80, 0.7);
  }
  70% {
    box-shadow: 0 0 0 10px rgba(40, 200, 80, 0);
  }
  100% {
    box-shadow: 0 0 0 0 rgba(40, 200, 80, 0);
  }
`

export function MicButton({ micEnabled, speaking, onClick }: MicButtonProps) {
  const getIcon = () => {
    if (micEnabled) return '🎤'
    return '🔇'
  }

  const getTooltip = () => {
    if (micEnabled) {
      return speaking ? 'Speaking... (click to mute)' : 'Mic active (click to mute)'
    }
    return 'Click to enable microphone'
  }

  return (
    <Tooltip label={getTooltip()} placement="right">
      <Box
        as="button"
        onClick={onClick}
        w="48px"
        h="48px"
        bg={
          micEnabled
            ? speaking
              ? 'rgba(40, 200, 80, 0.3)'
              : 'rgba(40, 200, 80, 0.2)'
            : 'rgba(200, 40, 40, 0.2)'
        }
        border={`1px solid ${
          micEnabled
            ? speaking
              ? 'rgba(40, 200, 80, 0.6)'
              : 'rgba(40, 200, 80, 0.4)'
            : 'rgba(200, 40, 40, 0.4)'
        }`}
        _hover={{
          bg: micEnabled ? 'rgba(40, 200, 80, 0.3)' : 'rgba(200, 40, 40, 0.3)',
          borderColor: micEnabled ? 'rgba(40, 200, 80, 0.6)' : 'rgba(200, 40, 40, 0.6)',
        }}
        _active={{
          bg: 'rgba(60, 60, 60, 0.3)',
        }}
        transition="all 0.1s"
        cursor="pointer"
        display="flex"
        alignItems="center"
        justifyContent="center"
        animation={speaking ? `${pulse} 1.5s infinite` : undefined}
      >
        <Text fontSize="xl" transition="all 0.3s">
          {getIcon()}
        </Text>
      </Box>
    </Tooltip>
  )
}
