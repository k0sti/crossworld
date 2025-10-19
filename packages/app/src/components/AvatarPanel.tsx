import { Box, VStack, Text, HStack, Divider, IconButton, Input, Badge, Button } from '@chakra-ui/react';
import { useState, useRef } from 'react';
import { ReadyPlayerMeService } from '../services/ready-player-me';
import type { TeleportAnimationType } from '../renderer/teleport-animation';

type VoxelModelType = 'boy' | 'girl';
type AvatarSelectionType = 'boy' | 'girl' | 'man' | 'simple';

interface AvatarPanelProps {
  useVoxelAvatar: boolean;
  onToggleAvatarType: (useVoxel: boolean) => void;
  currentModel: VoxelModelType;
  onModelChange: (model: VoxelModelType) => void;
  useVoxFile: boolean;
  onSourceChange: (useVox: boolean) => void;
  useOriginalColors: boolean;
  onColorModeChange: (useOriginal: boolean) => void;
  onRandomizeColors: () => void;
  onCustomColor: (color: string) => void;
  onAvatarUrlChange: (url: string) => void;
  currentUrl?: string;
  teleportAnimationType: TeleportAnimationType;
  onTeleportAnimationChange: (type: TeleportAnimationType) => void;
}

export function AvatarPanel({
  useVoxelAvatar,
  onToggleAvatarType,
  currentModel,
  onModelChange,
  useVoxFile,
  onSourceChange,
  useOriginalColors,
  onColorModeChange,
  onRandomizeColors,
  onCustomColor,
  onAvatarUrlChange,
  currentUrl,
  teleportAnimationType,
  onTeleportAnimationChange
}: AvatarPanelProps) {
  const [selectedColor, setSelectedColor] = useState('#ff6b6b');
  const colorInputRef = useRef<HTMLInputElement>(null);
  const [inputUrl, setInputUrl] = useState(currentUrl || '');
  const fileInputRef = useRef<HTMLInputElement>(null);
  const [showLoadModel, setShowLoadModel] = useState(false);

  // Determine current selection
  const currentSelection: AvatarSelectionType =
    !useVoxelAvatar ? 'man' :
    !useVoxFile ? 'simple' :
    currentModel;

  const handleSelectionChange = (selection: AvatarSelectionType) => {
    if (selection === 'boy' || selection === 'girl') {
      onToggleAvatarType(true);
      onSourceChange(true);
      onModelChange(selection);
    } else if (selection === 'simple') {
      onToggleAvatarType(true);
      onSourceChange(false);
    } else if (selection === 'man') {
      onToggleAvatarType(false);
    }
    setShowLoadModel(false);
  };

  const handleLoadAvatar = () => {
    if (inputUrl.trim()) {
      onAvatarUrlChange(inputUrl.trim());
    }
  };

  const handleOpenCreator = () => {
    ReadyPlayerMeService.openAvatarCreator();
  };

  const handleFileSelect = (event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    if (file && file.name.endsWith('.glb')) {
      const url = URL.createObjectURL(file);
      onAvatarUrlChange(url);
      setInputUrl(file.name);
    }
  };

  return (
    <Box
      position="fixed"
      top="60px"
      left="68px"
      zIndex={1500}
      bg="rgba(0, 0, 0, 0.1)"
      backdropFilter="blur(8px)"
      p={4}
      minW="320px"
      maxW="400px"
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
        <Text fontSize="md" fontWeight="semibold" color="white">
          Select Avatar
        </Text>

        {/* Voxel Models */}
        <VStack align="stretch" spacing={2}>
          <Box
            as="button"
            onClick={() => handleSelectionChange('boy')}
            p={3}
            bg={currentSelection === 'boy' ? 'rgba(100, 150, 250, 0.2)' : 'rgba(80, 80, 80, 0.1)'}
            border="1px solid"
            borderColor={currentSelection === 'boy' ? 'rgba(100, 150, 250, 0.4)' : 'rgba(255, 255, 255, 0.1)'}
            borderRadius="md"
            _hover={{
              bg: currentSelection === 'boy' ? 'rgba(100, 150, 250, 0.25)' : 'rgba(120, 120, 120, 0.2)',
              borderColor: currentSelection === 'boy' ? 'rgba(100, 150, 250, 0.5)' : 'rgba(255, 255, 255, 0.2)'
            }}
            transition="all 0.2s"
            cursor="pointer"
          >
            <HStack justify="space-between">
              <Text fontSize="sm" color="white" fontWeight="medium">
                👦 Boy
              </Text>
              <Badge colorScheme="purple" fontSize="2xs">
                vox
              </Badge>
            </HStack>
          </Box>

          <Box
            as="button"
            onClick={() => handleSelectionChange('girl')}
            p={3}
            bg={currentSelection === 'girl' ? 'rgba(250, 150, 200, 0.2)' : 'rgba(80, 80, 80, 0.1)'}
            border="1px solid"
            borderColor={currentSelection === 'girl' ? 'rgba(250, 150, 200, 0.4)' : 'rgba(255, 255, 255, 0.1)'}
            borderRadius="md"
            _hover={{
              bg: currentSelection === 'girl' ? 'rgba(250, 150, 200, 0.25)' : 'rgba(120, 120, 120, 0.2)',
              borderColor: currentSelection === 'girl' ? 'rgba(250, 150, 200, 0.5)' : 'rgba(255, 255, 255, 0.2)'
            }}
            transition="all 0.2s"
            cursor="pointer"
          >
            <HStack justify="space-between">
              <Text fontSize="sm" color="white" fontWeight="medium">
                👧 Girl
              </Text>
              <Badge colorScheme="purple" fontSize="2xs">
                vox
              </Badge>
            </HStack>
          </Box>
        </VStack>

        {/* GLB Models */}
        <VStack align="stretch" spacing={2}>
          <Box
            as="button"
            onClick={() => handleSelectionChange('man')}
            p={3}
            bg={currentSelection === 'man' ? 'rgba(150, 100, 250, 0.2)' : 'rgba(80, 80, 80, 0.1)'}
            border="1px solid"
            borderColor={currentSelection === 'man' ? 'rgba(150, 100, 250, 0.4)' : 'rgba(255, 255, 255, 0.1)'}
            borderRadius="md"
            _hover={{
              bg: currentSelection === 'man' ? 'rgba(150, 100, 250, 0.25)' : 'rgba(120, 120, 120, 0.2)',
              borderColor: currentSelection === 'man' ? 'rgba(150, 100, 250, 0.5)' : 'rgba(255, 255, 255, 0.2)'
            }}
            transition="all 0.2s"
            cursor="pointer"
          >
            <HStack justify="space-between">
              <Text fontSize="sm" color="white" fontWeight="medium">
                🎭 Man
              </Text>
              <Badge colorScheme="blue" fontSize="2xs">
                GLB
              </Badge>
            </HStack>
          </Box>
        </VStack>

        {/* Load Model Button */}
        <Button
          size="sm"
          fontSize="xs"
          colorScheme="cyan"
          onClick={() => setShowLoadModel(!showLoadModel)}
          width="100%"
        >
          {showLoadModel ? 'Hide' : 'Load Model'}
        </Button>

        {/* Load Model Section */}
        {showLoadModel && (
          <VStack align="stretch" spacing={2}>
            <Input
              value={inputUrl}
              onChange={(e) => setInputUrl(e.target.value)}
              placeholder="https://models.readyplayer.me/..."
              size="xs"
              fontSize="2xs"
              bg="rgba(255, 255, 255, 0.05)"
              color="white"
              borderColor="rgba(255, 255, 255, 0.2)"
              _hover={{ borderColor: 'rgba(255, 255, 255, 0.3)' }}
              _focus={{ borderColor: 'blue.400', boxShadow: '0 0 0 1px #3182ce' }}
            />

            <HStack spacing={2}>
              <Button
                size="xs"
                fontSize="2xs"
                colorScheme="blue"
                onClick={handleLoadAvatar}
                flex={1}
                isDisabled={!inputUrl.trim()}
              >
                Load URL
              </Button>

              <input
                ref={fileInputRef}
                type="file"
                accept=".glb,.vox"
                onChange={handleFileSelect}
                style={{ display: 'none' }}
              />

              <Button
                size="xs"
                fontSize="2xs"
                colorScheme="green"
                onClick={() => fileInputRef.current?.click()}
                flex={1}
              >
                Load File
              </Button>
            </HStack>

            <Button
              size="xs"
              fontSize="2xs"
              colorScheme="purple"
              onClick={handleOpenCreator}
              width="100%"
            >
              Create New (RPM)
            </Button>
          </VStack>
        )}

        {/* Generated Models */}
        <VStack align="stretch" spacing={2}>
          <Box
            as="button"
            onClick={() => handleSelectionChange('simple')}
            p={3}
            bg={currentSelection === 'simple' ? 'rgba(100, 200, 100, 0.2)' : 'rgba(80, 80, 80, 0.1)'}
            border="1px solid"
            borderColor={currentSelection === 'simple' ? 'rgba(100, 200, 100, 0.4)' : 'rgba(255, 255, 255, 0.1)'}
            borderRadius="md"
            _hover={{
              bg: currentSelection === 'simple' ? 'rgba(100, 200, 100, 0.25)' : 'rgba(120, 120, 120, 0.2)',
              borderColor: currentSelection === 'simple' ? 'rgba(100, 200, 100, 0.5)' : 'rgba(255, 255, 255, 0.2)'
            }}
            transition="all 0.2s"
            cursor="pointer"
          >
            <HStack justify="space-between">
              <Text fontSize="sm" color="white" fontWeight="medium">
                🧱 Simple
              </Text>
              <Badge colorScheme="green" fontSize="2xs">
                gen
              </Badge>
            </HStack>
          </Box>
        </VStack>

        {/* Generate Avatar Button */}
        <Button
          size="sm"
          fontSize="xs"
          colorScheme="teal"
          width="100%"
          isDisabled
        >
          Generate Avatar
        </Button>

        <Divider borderColor="rgba(255, 255, 255, 0.1)" />

        {/* Color Selection */}
        <VStack align="stretch" spacing={2}>
          <Text fontSize="sm" fontWeight="semibold" color="white">
            Color Options
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

        {/* Teleport Animation Selector */}

        <VStack align="stretch" spacing={2}>
          <HStack>
            <Text fontSize="sm" fontWeight="semibold" color="white">
              Teleport
            </Text>
            <Badge colorScheme="gray" fontSize="2xs" px={2}>
              CTRL+Click
            </Badge>
          </HStack>

          <HStack spacing={2} justify="center">
            <IconButton
              aria-label="Fade animation"
              icon={<Text fontSize="lg">🌫️</Text>}
              size="sm"
              bg={teleportAnimationType === 'fade' ? 'rgba(100, 150, 250, 0.3)' : 'rgba(80, 80, 80, 0.1)'}
              borderColor={teleportAnimationType === 'fade' ? 'rgba(100, 150, 250, 0.5)' : 'rgba(255, 255, 255, 0.1)'}
              borderWidth="1px"
              _hover={{
                bg: teleportAnimationType === 'fade' ? 'rgba(100, 150, 250, 0.4)' : 'rgba(120, 120, 120, 0.2)',
              }}
              onClick={() => onTeleportAnimationChange('fade')}
            />

            <IconButton
              aria-label="Scale animation"
              icon={<Text fontSize="lg">⚫</Text>}
              size="sm"
              bg={teleportAnimationType === 'scale' ? 'rgba(100, 150, 250, 0.3)' : 'rgba(80, 80, 80, 0.1)'}
              borderColor={teleportAnimationType === 'scale' ? 'rgba(100, 150, 250, 0.5)' : 'rgba(255, 255, 255, 0.1)'}
              borderWidth="1px"
              _hover={{
                bg: teleportAnimationType === 'scale' ? 'rgba(100, 150, 250, 0.4)' : 'rgba(120, 120, 120, 0.2)',
              }}
              onClick={() => onTeleportAnimationChange('scale')}
            />

            <IconButton
              aria-label="Spin animation"
              icon={<Text fontSize="lg">🌀</Text>}
              size="sm"
              bg={teleportAnimationType === 'spin' ? 'rgba(100, 150, 250, 0.3)' : 'rgba(80, 80, 80, 0.1)'}
              borderColor={teleportAnimationType === 'spin' ? 'rgba(100, 150, 250, 0.5)' : 'rgba(255, 255, 255, 0.1)'}
              borderWidth="1px"
              _hover={{
                bg: teleportAnimationType === 'spin' ? 'rgba(100, 150, 250, 0.4)' : 'rgba(120, 120, 120, 0.2)',
              }}
              onClick={() => onTeleportAnimationChange('spin')}
            />

            <IconButton
              aria-label="Slide animation"
              icon={<Text fontSize="lg">⬇️</Text>}
              size="sm"
              bg={teleportAnimationType === 'slide' ? 'rgba(100, 150, 250, 0.3)' : 'rgba(80, 80, 80, 0.1)'}
              borderColor={teleportAnimationType === 'slide' ? 'rgba(100, 150, 250, 0.5)' : 'rgba(255, 255, 255, 0.1)'}
              borderWidth="1px"
              _hover={{
                bg: teleportAnimationType === 'slide' ? 'rgba(100, 150, 250, 0.4)' : 'rgba(120, 120, 120, 0.2)',
              }}
              onClick={() => onTeleportAnimationChange('slide')}
            />

            <IconButton
              aria-label="Burst animation"
              icon={<Text fontSize="lg">✨</Text>}
              size="sm"
              bg={teleportAnimationType === 'burst' ? 'rgba(100, 150, 250, 0.3)' : 'rgba(80, 80, 80, 0.1)'}
              borderColor={teleportAnimationType === 'burst' ? 'rgba(100, 150, 250, 0.5)' : 'rgba(255, 255, 255, 0.1)'}
              borderWidth="1px"
              _hover={{
                bg: teleportAnimationType === 'burst' ? 'rgba(100, 150, 250, 0.4)' : 'rgba(120, 120, 120, 0.2)',
              }}
              onClick={() => onTeleportAnimationChange('burst')}
            />
          </HStack>
        </VStack>
      </VStack>
    </Box>
  );
}

export type { VoxelModelType };
