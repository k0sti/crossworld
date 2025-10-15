import { useState } from 'react';
import { Box, Input, Button, VStack, Text, Heading } from '@chakra-ui/react';
import { ReadyPlayerMeService } from '../services/ready-player-me';

interface AvatarDebugPanelProps {
  onAvatarUrlChange: (url: string) => void;
  currentUrl?: string;
}

export function AvatarDebugPanel({ onAvatarUrlChange, currentUrl }: AvatarDebugPanelProps) {
  const [inputUrl, setInputUrl] = useState(currentUrl || '');

  const handleLoadAvatar = () => {
    if (inputUrl.trim()) {
      onAvatarUrlChange(inputUrl.trim());
    }
  };

  const handleOpenCreator = () => {
    ReadyPlayerMeService.openAvatarCreator();
  };

  return (
    <Box
      position="fixed"
      bottom={3}
      left={3}
      bg="rgba(0, 0, 0, 0.6)"
      backdropFilter="blur(8px)"
      border="1px solid rgba(255, 255, 255, 0.1)"
      p={3}
      borderRadius="md"
      maxW="280px"
      zIndex={1000}
    >
      <VStack align="stretch" spacing={2}>
        <Heading size="xs" color="white" fontWeight="semibold">
          Avatar Debug
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
            Load Avatar
          </Button>

          <Button
            size="xs"
            fontSize="2xs"
            colorScheme="purple"
            onClick={handleOpenCreator}
            width="100%"
          >
            Create New
          </Button>
        </VStack>

        <Text fontSize="2xs" color="gray.500">
          💡 readyplayer.me
        </Text>
      </VStack>
    </Box>
  );
}
