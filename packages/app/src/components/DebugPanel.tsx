import { Box, Text, VStack } from '@chakra-ui/react';

export interface DebugInfo {
  cursorWorld?: { x: number; y: number; z: number };
  cursorOctree?: { x: number; y: number; z: number; depth: number };
  cursorDepth?: number;
  cursorSize?: number;
  avatarPos?: { x: number; y: number; z: number };
  cameraPos?: { x: number; y: number; z: number };
  worldSize?: number;
  isEditMode?: boolean;
}

interface DebugPanelProps {
  info: DebugInfo;
}

export function DebugPanel({ info }: DebugPanelProps) {
  const formatNum = (n: number | undefined) => n?.toFixed(3) ?? 'N/A';
  const formatVec = (v: { x: number; y: number; z: number } | undefined) =>
    v ? `${formatNum(v.x)}, ${formatNum(v.y)}, ${formatNum(v.z)}` : 'N/A';

  return (
    <Box
      position="fixed"
      top="10px"
      right="10px"
      bg="rgba(0, 0, 0, 0.75)"
      color="white"
      p={2}
      borderRadius="md"
      fontSize="xs"
      fontFamily="monospace"
      userSelect="none"
      pointerEvents="none"
      zIndex={1000}
      minWidth="280px"
    >
      <VStack align="stretch" spacing={0.5}>
        <Text fontWeight="bold" color="cyan.300">DEBUG INFO</Text>
        <Text>World: {info.worldSize ?? 'N/A'}×{info.worldSize ?? 'N/A'}×{info.worldSize ?? 'N/A'}</Text>
        <Text>Mode: {info.isEditMode ? 'EDIT' : 'WALK'}</Text>
        <Text color="yellow.300">─ Cursor ─</Text>
        <Text>World: {formatVec(info.cursorWorld)}</Text>
        <Text>Octree: {info.cursorOctree ? `${info.cursorOctree.x}, ${info.cursorOctree.y}, ${info.cursorOctree.z}` : 'N/A'}</Text>
        <Text>Depth: {info.cursorDepth ?? 'N/A'} (size: {formatNum(info.cursorSize)})</Text>
        <Text color="green.300">─ Avatar ─</Text>
        <Text>Pos: {formatVec(info.avatarPos)}</Text>
        <Text color="blue.300">─ Camera ─</Text>
        <Text>Pos: {formatVec(info.cameraPos)}</Text>
      </VStack>
    </Box>
  );
}
