import { Box, VStack, Text, HStack, Button, Divider, IconButton, Input } from '@chakra-ui/react';
import { useState, useRef } from 'react';

type VoxelModelType = 'boy' | 'girl';
type VoxelSourceType = 'vox' | 'generated';

interface VoxelModelPanelProps {
  currentModel: VoxelModelType;
  onModelChange: (model: VoxelModelType) => void;
  useVoxFile: boolean;
  onSourceChange: (useVox: boolean) => void;
  useOriginalColors: boolean;
  onColorModeChange: (useOriginal: boolean) => void;
  onRandomizeColors: () => void;
  onCustomColor: (color: string) => void;
}

export function VoxelModelPanel({
  currentModel,
  onModelChange,
  useVoxFile,
  onSourceChange,
  useOriginalColors,
  onColorModeChange,
  onRandomizeColors,
  onCustomColor
}: VoxelModelPanelProps) {
  const [selectedColor, setSelectedColor] = useState('#ff6b6b');
  const colorInputRef = useRef<HTMLInputElement>(null);
  return (
    <Box
      position="fixed"
      top="60px"
      right="20px"
      zIndex={1500}
      bg="rgba(0, 0, 0, 0.1)"
      backdropFilter="blur(8px)"
      p={4}
      minW="280px"
      maxW="320px"
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
      <VStack align="stretch" spacing={3}>
        <VStack align="stretch" spacing={2}>
          <Text fontSize="md" fontWeight="semibold" color="white">
            🧱 Voxel Avatar
          </Text>

          <HStack spacing={2}>
            <IconButton
              aria-label={useOriginalColors ? "Original colors" : "Random colors"}
              icon={<Text fontSize="lg">{useOriginalColors ? "🎨" : "🎲"}</Text>}
              size="sm"
              colorScheme={useOriginalColors ? 'orange' : 'cyan'}
              onClick={() => onColorModeChange(!useOriginalColors)}
            />

            {!useOriginalColors && (
              <>
                <IconButton
                  aria-label="Randomize colors"
                  icon={<Text fontSize="lg">🌈</Text>}
                  size="sm"
                  colorScheme="teal"
                  onClick={onRandomizeColors}
                />

                <IconButton
                  aria-label="Pick color"
                  icon={<Text fontSize="lg">🎨</Text>}
                  size="sm"
                  bg={selectedColor}
                  _hover={{ opacity: 0.8 }}
                  onClick={() => colorInputRef.current?.click()}
                  border="2px solid rgba(255, 255, 255, 0.3)"
                />

                <Input
                  ref={colorInputRef}
                  type="color"
                  value={selectedColor}
                  onChange={(e) => {
                    setSelectedColor(e.target.value);
                    onCustomColor(e.target.value);
                  }}
                  position="absolute"
                  opacity={0}
                  pointerEvents="none"
                  w="0"
                  h="0"
                />
              </>
            )}
          </HStack>
        </VStack>

        <Divider borderColor="rgba(255, 255, 255, 0.1)" />

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
