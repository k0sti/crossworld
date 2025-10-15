import { useState, useRef } from 'react';
import { Box, Input, Button, VStack, Text, Heading, Divider } from '@chakra-ui/react';
import { ReadyPlayerMeService } from '../services/ready-player-me';

interface AvatarDebugPanelProps {
  onAvatarUrlChange: (url: string) => void;
  onCreateVoxelAvatar?: () => void;
  currentUrl?: string;
}

export function AvatarDebugPanel({ onAvatarUrlChange, currentUrl }: AvatarDebugPanelProps) {
  const [inputUrl, setInputUrl] = useState(currentUrl || '');
  const fileInputRef = useRef<HTMLInputElement>(null);

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
          GLB Avatar Loader
        </Heading>

        <VStack align="stretch" spacing={1.5}>
          <Text fontSize="2xs" color="gray.400">
            Ready Player Me URL:
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
          <Text fontSize="2xs" color="gray.400">
            Load from Disk:
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

        <Text fontSize="2xs" color="gray.500" textAlign="center">
          💡 Toggle to voxel mode in settings
        </Text>
      </VStack>
    </Box>
  );
}
