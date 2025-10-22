import { Box, HStack, VStack, Text } from '@chakra-ui/react'
import { ReactNode, useEffect, useRef, useState } from 'react'

interface ResponsivePanelProps {
  /** Panel content */
  children: ReactNode
  /** Whether panel is open */
  isOpen: boolean
  /** Callback when panel should close */
  onClose?: () => void
  /** Panel title (shown in top bar) */
  title?: string
  /** Action buttons for top bar (shown on second line) */
  actions?: ReactNode
  /** Initial position from top (in pixels or string like "60px") */
  top?: string | number
  /** Initial position from left (in pixels or string like "68px") */
  left?: string | number
  /** Initial position from right (in pixels or string) */
  right?: string | number
  /** Initial position from bottom (in pixels or string) */
  bottom?: string | number
  /** Minimum width in normal mode */
  minWidth?: string | number
  /** Maximum width in normal mode */
  maxWidth?: string | number
  /** Minimum height in normal mode */
  minHeight?: string | number
  /** Maximum height in normal mode */
  maxHeight?: string | number
  /** Z-index */
  zIndex?: number
  /** Padding */
  padding?: string | number
  /** Close on click outside */
  closeOnClickOutside?: boolean
  /** Optional class name */
  className?: string
  /** Force fullscreen mode */
  forceFullscreen?: boolean
  /** Threshold in pixels for overflow detection (default: 50) */
  overflowThreshold?: number
  /** Center the panel (ignores top/left/right/bottom when true) */
  centered?: boolean
  /** Close on ESC key */
  closeOnEsc?: boolean
}

export function ResponsivePanel({
  children,
  isOpen,
  onClose,
  title,
  actions,
  top = '60px',
  left,
  right,
  bottom,
  minWidth = '400px',
  maxWidth = '500px',
  minHeight,
  maxHeight,
  zIndex = 1500,
  padding = 4,
  closeOnClickOutside = true,
  className,
  forceFullscreen = false,
  overflowThreshold = 50,
  centered = false,
  closeOnEsc = true,
}: ResponsivePanelProps) {
  const panelRef = useRef<HTMLDivElement>(null)
  const [isFullscreen, setIsFullscreen] = useState(forceFullscreen)
  const resizeObserverRef = useRef<ResizeObserver | null>(null)

  // Check if content overflows viewport
  const checkOverflow = () => {
    if (!panelRef.current || forceFullscreen) {
      setIsFullscreen(forceFullscreen)
      return
    }

    const rect = panelRef.current.getBoundingClientRect()
    const windowWidth = window.innerWidth
    const windowHeight = window.innerHeight

    // Check if panel width/height exceeds viewport
    const contentWidth = rect.width
    const contentHeight = rect.height

    // Check if panel extends beyond viewport with threshold
    const overflowsHorizontally =
      rect.right > windowWidth - overflowThreshold ||
      rect.left < overflowThreshold ||
      contentWidth > windowWidth - (overflowThreshold * 2)
    const overflowsVertically =
      rect.bottom > windowHeight - overflowThreshold ||
      rect.top < overflowThreshold ||
      contentHeight > windowHeight - (overflowThreshold * 2)

    setIsFullscreen(overflowsHorizontally || overflowsVertically)
  }

  // Monitor content size changes
  useEffect(() => {
    if (!panelRef.current) return

    // Initial check
    checkOverflow()

    // Set up ResizeObserver to watch for content changes
    resizeObserverRef.current = new ResizeObserver(() => {
      checkOverflow()
    })

    resizeObserverRef.current.observe(panelRef.current)

    // Also check on window resize
    window.addEventListener('resize', checkOverflow)

    return () => {
      if (resizeObserverRef.current) {
        resizeObserverRef.current.disconnect()
      }
      window.removeEventListener('resize', checkOverflow)
    }
  }, [forceFullscreen, overflowThreshold])

  // Re-check when content changes
  useEffect(() => {
    if (isOpen) {
      // Small delay to allow content to render
      const timer = setTimeout(checkOverflow, 100)
      return () => clearTimeout(timer)
    }
  }, [isOpen, children])

  // Handle click outside to close panel
  useEffect(() => {
    if (!closeOnClickOutside || !onClose || !isOpen) return

    const handleClickOutside = (event: MouseEvent) => {
      if (panelRef.current && !panelRef.current.contains(event.target as Node)) {
        onClose()
      }
    }

    // Add listener after a small delay to prevent immediate closing
    const timeoutId = setTimeout(() => {
      document.addEventListener('mousedown', handleClickOutside)
    }, 100)

    return () => {
      clearTimeout(timeoutId)
      document.removeEventListener('mousedown', handleClickOutside)
    }
  }, [closeOnClickOutside, onClose, isOpen])

  // Handle ESC key to close panel
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

  // Calculate positioning based on centered prop
  const getPositionProps = () => {
    if (isFullscreen) {
      return {
        top: 0,
        left: 0,
        right: 0,
        bottom: 0,
        transform: 'none',
      }
    }

    if (centered) {
      return {
        top: '50%',
        left: '50%',
        transform: 'translate(-50%, -50%)',
        right: undefined,
        bottom: undefined,
      }
    }

    return {
      top,
      left,
      right,
      bottom,
      transform: 'none',
    }
  }

  const positionProps = getPositionProps()

  return (
    <Box
      ref={panelRef}
      className={className}
      position="fixed"
      {...positionProps}
      width={isFullscreen ? '100vw' : 'auto'}
      height={isFullscreen ? '100vh' : 'auto'}
      minW={isFullscreen ? undefined : minWidth}
      maxW={isFullscreen ? undefined : maxWidth}
      minH={isFullscreen ? undefined : minHeight}
      maxH={isFullscreen ? '100vh' : maxHeight}
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
