import { useState, useRef } from 'react';
import { Box, Input, Button, VStack, Text, Divider } from '@chakra-ui/react';
import { ReadyPlayerMeService } from '../services/ready-player-me';

interface GLBPanelProps {
  onAvatarUrlChange: (url: string) => void;
  currentUrl?: string;
}

export function GLBPanel({ onAvatarUrlChange, currentUrl }: GLBPanelProps) {
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
        <Text fontSize="md" fontWeight="semibold" color="white">
          🎭 GLB Avatar
        </Text>

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

      </VStack>
    </Box>
  );
}
