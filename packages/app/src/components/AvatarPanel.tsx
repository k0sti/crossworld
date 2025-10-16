import { Box, VStack, Text, HStack, Divider, IconButton, Input, Badge, Button } from '@chakra-ui/react';
import { useState, useRef } from 'react';
import { ReadyPlayerMeService } from '../services/ready-player-me';

type VoxelModelType = 'boy' | 'girl';
type AvatarSelection = 'generated' | 'boy' | 'girl';

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
  currentUrl
}: AvatarPanelProps) {
  const [selectedColor, setSelectedColor] = useState('#ff6b6b');
  const colorInputRef = useRef<HTMLInputElement>(null);
  const [inputUrl, setInputUrl] = useState(currentUrl || '');
  const fileInputRef = useRef<HTMLInputElement>(null);

  const currentSelection: AvatarSelection = useVoxFile ? currentModel : 'generated';

  const handleSelectionChange = (selection: AvatarSelection) => {
    if (selection === 'generated') {
      onSourceChange(false);
    } else {
      onSourceChange(true);
      onModelChange(selection);
    }
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
          Avatar Settings
        </Text>

        {/* Voxel/GLB Toggle */}
        <HStack spacing={2}>
          <Box
            as="button"
            onClick={() => onToggleAvatarType(true)}
            flex={1}
            p={2}
            bg={useVoxelAvatar ? 'rgba(100, 150, 250, 0.2)' : 'rgba(80, 80, 80, 0.1)'}
            border="1px solid"
            borderColor={useVoxelAvatar ? 'rgba(100, 150, 250, 0.4)' : 'rgba(255, 255, 255, 0.1)'}
            borderRadius="md"
            _hover={{
              bg: useVoxelAvatar ? 'rgba(100, 150, 250, 0.25)' : 'rgba(120, 120, 120, 0.2)',
              borderColor: useVoxelAvatar ? 'rgba(100, 150, 250, 0.5)' : 'rgba(255, 255, 255, 0.2)'
            }}
            transition="all 0.2s"
            cursor="pointer"
          >
            <Text fontSize="sm" color="white" fontWeight="medium">
              🧱 Voxel
            </Text>
          </Box>

          <Box
            as="button"
            onClick={() => onToggleAvatarType(false)}
            flex={1}
            p={2}
            bg={!useVoxelAvatar ? 'rgba(150, 100, 250, 0.2)' : 'rgba(80, 80, 80, 0.1)'}
            border="1px solid"
            borderColor={!useVoxelAvatar ? 'rgba(150, 100, 250, 0.4)' : 'rgba(255, 255, 255, 0.1)'}
            borderRadius="md"
            _hover={{
              bg: !useVoxelAvatar ? 'rgba(150, 100, 250, 0.25)' : 'rgba(120, 120, 120, 0.2)',
              borderColor: !useVoxelAvatar ? 'rgba(150, 100, 250, 0.5)' : 'rgba(255, 255, 255, 0.2)'
            }}
            transition="all 0.2s"
            cursor="pointer"
          >
            <Text fontSize="sm" color="white" fontWeight="medium">
              🎭 GLB
            </Text>
          </Box>
        </HStack>

        <Divider borderColor="rgba(255, 255, 255, 0.1)" />

        {/* Voxel Content */}
        {useVoxelAvatar && (
          <>
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

            <VStack align="stretch" spacing={2}>
              <Text fontSize="sm" fontWeight="semibold" color="white">
                Avatar Type
              </Text>

              <Box
                as="button"
                onClick={() => handleSelectionChange('generated')}
                p={3}
                bg={currentSelection === 'generated' ? 'rgba(100, 200, 100, 0.2)' : 'rgba(80, 80, 80, 0.1)'}
                border="1px solid"
                borderColor={currentSelection === 'generated' ? 'rgba(100, 200, 100, 0.4)' : 'rgba(255, 255, 255, 0.1)'}
                borderRadius="md"
                _hover={{
                  bg: currentSelection === 'generated' ? 'rgba(100, 200, 100, 0.25)' : 'rgba(120, 120, 120, 0.2)',
                  borderColor: currentSelection === 'generated' ? 'rgba(100, 200, 100, 0.5)' : 'rgba(255, 255, 255, 0.2)'
                }}
                transition="all 0.2s"
                cursor="pointer"
              >
                <HStack justify="space-between">
                  <Text fontSize="sm" color="white" fontWeight="medium">
                    🧱 Generated
                  </Text>
                  <Badge colorScheme="green" fontSize="2xs">
                    gen
                  </Badge>
                </HStack>
              </Box>

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

            {currentSelection !== 'generated' ? (
              <Text fontSize="2xs" color="gray.500" textAlign="center" mt={1}>
                {currentSelection === 'boy' ? 'chr_peasant_guy_blackhair.vox' : 'chr_peasant_girl_orangehair.vox'}
              </Text>
            ) : (
              <Text fontSize="2xs" color="gray.500" textAlign="center" mt={1}>
                Procedurally generated model
              </Text>
            )}
          </>
        )}

        {/* GLB Content */}
        {!useVoxelAvatar && (
          <>
            <VStack align="stretch" spacing={1.5}>
              <Text fontSize="sm" fontWeight="semibold" color="white">
                Ready Player Me URL
              </Text>
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

              <Button
                size="xs"
                fontSize="2xs"
                colorScheme="blue"
                onClick={handleLoadAvatar}
                width="100%"
                isDisabled={!inputUrl.trim()}
              >
                Load from URL
              </Button>

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

            <Divider borderColor="rgba(255, 255, 255, 0.1)" />

            <VStack align="stretch" spacing={1.5}>
              <Text fontSize="sm" fontWeight="semibold" color="white">
                Load from Disk
              </Text>

              <input
                ref={fileInputRef}
                type="file"
                accept=".glb"
                onChange={handleFileSelect}
                style={{ display: 'none' }}
              />

              <Button
                size="xs"
                fontSize="2xs"
                colorScheme="green"
                onClick={() => fileInputRef.current?.click()}
                width="100%"
              >
                Choose GLB File
              </Button>
            </VStack>
          </>
        )}
      </VStack>
    </Box>
  );
}

export type { VoxelModelType, AvatarSelection };
