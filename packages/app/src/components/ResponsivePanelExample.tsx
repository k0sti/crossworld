/**
 * Example of using ResponsivePanel component
 *
 * This demonstrates how to refactor existing panels to use ResponsivePanel
 * which automatically switches to fullscreen when content overflows.
 */

import { VStack, Text, Box, Button } from '@chakra-ui/react'
import { useState } from 'react'
import { ResponsivePanel } from './ResponsivePanel'

export function ResponsivePanelExample() {
  const [isOpen, setIsOpen] = useState(false)
  const [contentSize, setContentSize] = useState<'small' | 'large'>('small')

  return (
    <>
      <Button onClick={() => setIsOpen(true)}>
        Open Panel
      </Button>

      <ResponsivePanel
        isOpen={isOpen}
        onClose={() => setIsOpen(false)}
        top="60px"
        left="68px"
        minWidth="400px"
        maxWidth="500px"
        closeOnClickOutside={true}
      >
        <VStack align="stretch" gap={4}>
          <Text fontSize="xl" color="white" fontWeight="bold">
            Responsive Panel Example
          </Text>

          <Text fontSize="sm" color="whiteAlpha.700">
            This panel automatically switches to fullscreen when content overflows
            the viewport horizontally or vertically.
          </Text>

          <Button
            onClick={() => setContentSize(contentSize === 'small' ? 'large' : 'small')}
            size="sm"
            colorScheme="blue"
          >
            Toggle Content Size
          </Button>

          {contentSize === 'large' && (
            <VStack align="stretch" gap={2}>
              <Text fontSize="lg" color="white" fontWeight="semibold">
                Large Content Mode
              </Text>
              {Array.from({ length: 50 }).map((_, i) => (
                <Box
                  key={i}
                  p={2}
                  bg="rgba(255, 255, 255, 0.05)"
                  borderRadius="md"
                >
                  <Text fontSize="sm" color="white">
                    Content Item {i + 1}
                  </Text>
                </Box>
              ))}
            </VStack>
          )}
        </VStack>
      </ResponsivePanel>
    </>
  )
}

/**
 * How to convert existing panels to use ResponsivePanel:
 *
 * 1. Import ResponsivePanel:
 *    import { ResponsivePanel } from './ResponsivePanel'
 *
 * 2. Replace the outer Box with ResponsivePanel:
 *
 *    BEFORE:
 *    <Box
 *      ref={panelRef}
 *      position="fixed"
 *      top="60px"
 *      left="68px"
 *      zIndex={1500}
 *      bg="rgba(0, 0, 0, 0.1)"
 *      backdropFilter="blur(8px)"
 *      p={4}
 *      minW="400px"
 *      maxW="500px"
 *    >
 *      {content}
 *    </Box>
 *
 *    AFTER:
 *    <ResponsivePanel
 *      isOpen={true}  // or pass your open state
 *      onClose={onClose}
 *      top="60px"
 *      left="68px"
 *      minWidth="400px"
 *      maxWidth="500px"
 *      padding={4}
 *      closeOnClickOutside={true}
 *    >
 *      {content}
 *    </ResponsivePanel>
 *
 * 3. Remove manual click-outside handling - ResponsivePanel handles it
 *
 * 4. Remove the panelRef if it was only used for click-outside detection
 *
 * 5. The panel will automatically:
 *    - Switch to fullscreen when content overflows
 *    - Show a close button in fullscreen mode
 *    - Handle smooth transitions
 *    - Maintain the same visual style
 */
