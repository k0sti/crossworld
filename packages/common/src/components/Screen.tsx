import { Box, HStack, VStack, Text } from '@chakra-ui/react'
import { ReactNode, useEffect } from 'react'

interface ScreenProps {
  /** Screen content */
  children: ReactNode
  /** Whether screen is open */
  isOpen: boolean
  /** Callback when screen should close */
  onClose?: () => void
  /** Screen title (shown in top bar) */
  title?: string
  /** Action buttons for top bar */
  actions?: ReactNode
  /** Z-index */
  zIndex?: number
  /** Padding */
  padding?: string | number
  /** Close on click outside */
  closeOnClickOutside?: boolean
  /** Optional class name */
  className?: string
  /** Close on ESC key */
  closeOnEsc?: boolean
}

export function Screen({
  children,
  isOpen,
  onClose,
  title,
  actions,
  zIndex = 1500,
  padding = 4,
  closeOnClickOutside: _closeOnClickOutside = true,
  className,
  closeOnEsc = true,
}: ScreenProps) {
  // Handle ESC key to close screen
  useEffect(() => {
    if (!closeOnEsc || !onClose || !isOpen) return

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        onClose()
      }
    }

    document.addEventListener('keydown', handleKeyDown)

    return () => {
      document.removeEventListener('keydown', handleKeyDown)
    }
  }, [closeOnEsc, onClose, isOpen])

  if (!isOpen) return null

  return (
    <Box
      className={className}
      position="fixed"
      top={0}
      left={0}
      right={0}
      bottom={0}
      width="100vw"
      height="100vh"
      zIndex={zIndex}
      bg="rgba(0, 0, 0, 0.1)"
      backdropFilter="blur(8px)"
      overflow="hidden"
      display="flex"
      flexDirection="column"
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
      {/* Top Bar */}
      {(title || actions) && (
        <VStack
          align="stretch"
          spacing={3}
          p={padding}
          borderBottom="1px solid rgba(255, 255, 255, 0.1)"
          bg="rgba(0, 0, 0, 0.2)"
          flexShrink={0}
        >
          {title && (
            <Text fontSize="2xl" fontWeight="bold" color="white" textAlign="center">
              {title}
            </Text>
          )}
          {actions && (
            <HStack spacing={3} justify="center">
              {actions}
            </HStack>
          )}
        </VStack>
      )}

      {/* Scrollable Content */}
      <Box
        flex={1}
        overflow="auto"
        p={padding}
      >
        {children}
      </Box>
    </Box>
  )
}
