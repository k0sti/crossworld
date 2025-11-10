/**
 * InitializationOverlay - Displays initialization progress badges
 *
 * Shows compact colored badges at bottom center for each initialization sub-component
 */
import { memo } from 'react';
import { Box, Flex } from '@chakra-ui/react';
import { InitializationBadge } from './InitializationBadge';
import type { InitializationState } from '../initialization/types';

interface InitializationOverlayProps {
  initState: InitializationState | null;
  /** Whether to show the overlay (only during initialization) */
  show: boolean;
}

export const InitializationOverlay = memo(({ initState, show }: InitializationOverlayProps) => {
  if (!show || !initState || initState.phase === 'ready' || initState.phase === 'idle') {
    return null;
  }

  // Convert Map to Array for rendering
  const subComponents = Array.from(initState.subComponents.values());

  // Filter out pending components that haven't started yet (to reduce clutter)
  const activeComponents = subComponents.filter(
    (comp) => comp.status !== 'pending' || comp.progress > 0
  );

  if (activeComponents.length === 0) {
    return null;
  }

  return (
    <Box
      position="fixed"
      bottom="80px" // Just above bottom bar, under top bar
      left="50%"
      transform="translateX(-50%)"
      zIndex={1000}
      pointerEvents="none"
    >
      <Flex gap={2} align="center" justify="center">
        {activeComponents.map((component) => (
          <InitializationBadge key={component.id} status={component} />
        ))}
      </Flex>
    </Box>
  );
});

InitializationOverlay.displayName = 'InitializationOverlay';
