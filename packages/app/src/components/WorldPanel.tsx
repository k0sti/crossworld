import { useState } from 'react';
import { Box, Text, VStack, HStack, Input, Button } from '@chakra-ui/react';
import { CubeCoord, printCubeCoord } from '../types/cube-coord';
import { DEFAULT_MACRO_DEPTH, DEFAULT_MICRO_DEPTH } from '../constants/geometry';

export interface DebugInfo {
  cursorWorld?: { x: number; y: number; z: number };
  cursorOctree?: CubeCoord | null;
  cursorDepth?: number;
  cursorSize?: number;
  avatarPos?: { x: number; y: number; z: number };
  cameraPos?: { x: number; y: number; z: number };
  worldSize?: number;
  isEditMode?: boolean;
}

interface WorldPanelProps {
  info: DebugInfo;
  onApplyDepthSettings?: (worldDepth: number, scaleDepth: number) => void;
}

export function WorldPanel({ info, onApplyDepthSettings }: WorldPanelProps) {
  const [macroDepth, setMacroDepth] = useState(String(DEFAULT_MACRO_DEPTH));
  const [microDepth, setMicroDepth] = useState(String(DEFAULT_MICRO_DEPTH));

  const formatNum = (n: number | undefined) => n?.toFixed(3) ?? 'N/A';
  const formatVec = (v: { x: number; y: number; z: number } | undefined) =>
    v ? `${formatNum(v.x)}, ${formatNum(v.y)}, ${formatNum(v.z)}` : 'N/A';

  const handleApply = () => {
    const macro = parseInt(macroDepth);
    const micro = parseInt(microDepth);
    if (!isNaN(macro) && !isNaN(micro) && onApplyDepthSettings) {
      const totalDepth = macro + micro;
      onApplyDepthSettings(totalDepth, micro);
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') {
      handleApply();
    }
  };

  return (
    <Box
      position="fixed"
      bottom={2}
      left={14}
      bg="rgba(0, 0, 0, 0.75)"
      color="white"
      p={2}
      borderRadius="md"
      fontSize="xs"
      fontFamily="monospace"
      userSelect="none"
      zIndex={1000}
      minWidth="280px"
    >
      <VStack align="stretch" spacing={0.5}>
        <Text fontWeight="bold" color="cyan.300">WORLD PANEL</Text>
        <Text>World: {info.worldSize ?? 'N/A'}×{info.worldSize ?? 'N/A'}×{info.worldSize ?? 'N/A'}</Text>
        <Text>Mode: {info.isEditMode ? 'EDIT' : 'WALK'}</Text>

        <Text color="yellow.300">─ Cursor ─</Text>
        <Text>World: {formatVec(info.cursorWorld)}</Text>
        <Text>Octree: {printCubeCoord(info.cursorOctree)}</Text>
        <Text>Depth: {info.cursorDepth ?? 'N/A'} (size: {formatNum(info.cursorSize)})</Text>

        <Text color="green.300">─ Avatar ─</Text>
        <Text>Pos: {formatVec(info.avatarPos)}</Text>

        <Text color="blue.300">─ Camera ─</Text>
        <Text>Pos: {formatVec(info.cameraPos)}</Text>

        <Text color="orange.300">─ Settings ─</Text>
        <Box pointerEvents="auto">
          <HStack spacing={2} mb={1}>
            <Text minWidth="80px">Macro depth:</Text>
            <Input
              size="xs"
              value={macroDepth}
              onChange={(e) => setMacroDepth(e.target.value)}
              onKeyDown={handleKeyDown}
              bg="rgba(0, 0, 0, 0.5)"
              border="1px solid"
              borderColor="gray.600"
              width="60px"
              textAlign="center"
            />
          </HStack>
          <HStack spacing={2} mb={1}>
            <Text minWidth="80px">Micro depth:</Text>
            <Input
              size="xs"
              value={microDepth}
              onChange={(e) => setMicroDepth(e.target.value)}
              onKeyDown={handleKeyDown}
              bg="rgba(0, 0, 0, 0.5)"
              border="1px solid"
              borderColor="gray.600"
              width="60px"
              textAlign="center"
            />
          </HStack>
          <Button
            size="xs"
            onClick={handleApply}
            colorScheme="blue"
            width="100%"
          >
            Apply
          </Button>
        </Box>
      </VStack>
    </Box>
  );
}

// Re-export as DebugPanel for backward compatibility
export { WorldPanel as DebugPanel };
