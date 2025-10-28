import { Box, Code, Heading, VStack } from '@chakra-ui/react';
import { useEffect, useState } from 'react';

interface ScriptPanelProps {
  csmText: string;
}

export const ScriptPanel: React.FC<ScriptPanelProps> = ({ csmText }) => {
  const [displayText, setDisplayText] = useState('# World CSM will appear here after placing voxels');

  useEffect(() => {
    if (csmText) {
      setDisplayText(csmText);
    }
  }, [csmText]);

  return (
    <Box
      position="absolute"
      top="80px"
      right="20px"
      width="400px"
      maxHeight="60vh"
      bg="blackAlpha.800"
      borderRadius="md"
      border="1px solid"
      borderColor="whiteAlpha.300"
      p={4}
      zIndex={10}
    >
      <VStack align="stretch" gap={2}>
        <Heading size="sm" color="white">
          World Script (CSM)
        </Heading>
        <Box
          bg="blackAlpha.600"
          borderRadius="md"
          p={3}
          maxHeight="50vh"
          overflowY="auto"
          fontFamily="mono"
          fontSize="xs"
        >
          <Code
            display="block"
            whiteSpace="pre"
            bg="transparent"
            color="green.300"
            p={0}
          >
            {displayText}
          </Code>
        </Box>
      </VStack>
    </Box>
  );
};
