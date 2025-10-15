import { Box, VStack, Text, Heading, HStack, Button, Divider } from '@chakra-ui/react';

type VoxelModelType = 'boy' | 'girl';
type VoxelSourceType = 'vox' | 'generated';

interface VoxelModelPanelProps {
  currentModel: VoxelModelType;
  onModelChange: (model: VoxelModelType) => void;
  useVoxFile: boolean;
  onSourceChange: (useVox: boolean) => void;
}

export function VoxelModelPanel({ currentModel, onModelChange, useVoxFile, onSourceChange }: VoxelModelPanelProps) {
  return (
    <Box
      position="fixed"
      bottom={3}
      left={3}
      bg="rgba(0, 0, 0, 0.8)"
      backdropFilter="blur(8px)"
      border="1px solid rgba(255, 255, 255, 0.1)"
      p={3}
      borderRadius="md"
      maxW="280px"
      zIndex={1000}
    >
      <VStack align="stretch" spacing={2}>
        <Heading size="xs" color="white" fontWeight="semibold">
          Voxel Avatar
        </Heading>

        <VStack align="stretch" spacing={1.5}>
          <Text fontSize="2xs" color="gray.400">
            Source:
          </Text>

          <HStack spacing={2}>
            <Button
              size="sm"
              fontSize="xs"
              flex={1}
              colorScheme={useVoxFile ? 'purple' : 'gray'}
              variant={useVoxFile ? 'solid' : 'outline'}
              onClick={() => onSourceChange(true)}
            >
              📦 .vox
            </Button>

            <Button
              size="sm"
              fontSize="xs"
              flex={1}
              colorScheme={!useVoxFile ? 'green' : 'gray'}
              variant={!useVoxFile ? 'solid' : 'outline'}
              onClick={() => onSourceChange(false)}
            >
              🧱 Generated
            </Button>
          </HStack>
        </VStack>

        <Divider borderColor="rgba(255, 255, 255, 0.1)" />

        <VStack align="stretch" spacing={1.5}>
          <Text fontSize="2xs" color="gray.400">
            Character:
          </Text>

          <HStack spacing={2}>
            <Button
              size="sm"
              fontSize="xs"
              flex={1}
              colorScheme={currentModel === 'boy' ? 'blue' : 'gray'}
              variant={currentModel === 'boy' ? 'solid' : 'outline'}
              onClick={() => onModelChange('boy')}
            >
              👦 Boy
            </Button>

            <Button
              size="sm"
              fontSize="xs"
              flex={1}
              colorScheme={currentModel === 'girl' ? 'pink' : 'gray'}
              variant={currentModel === 'girl' ? 'solid' : 'outline'}
              onClick={() => onModelChange('girl')}
            >
              👧 Girl
            </Button>
          </HStack>
        </VStack>

        {useVoxFile ? (
          <Text fontSize="2xs" color="gray.500" textAlign="center" mt={1}>
            {currentModel === 'boy' ? 'chr_peasant_guy_blackhair.vox' : 'chr_peasant_girl_orangehair.vox'}
          </Text>
        ) : (
          <Text fontSize="2xs" color="gray.500" textAlign="center" mt={1}>
            Procedurally generated model
          </Text>
        )}
      </VStack>
    </Box>
  );
}

export type { VoxelModelType, VoxelSourceType };
