import { useState, useEffect } from 'react';
import { Box, Text, VStack, HStack, Input, Button, Slider, SliderTrack, SliderFilledTrack, SliderThumb, Switch } from '@chakra-ui/react';
import { CubeCoord, printCubeCoord } from '../types/cube-coord';
import { getMacroDepth, getMicroDepth, onDepthChange } from '../config/depth-config';

export interface DebugInfo {
  cursorWorld?: { x: number; y: number; z: number };
  cursorOctree?: CubeCoord | null;
  cursorDepth?: number;
  cursorSize?: number;
  avatarPos?: { x: number; y: number; z: number };
  cameraPos?: { x: number; y: number; z: number };
  worldSize?: number;
  isEditMode?: boolean;
  timeOfDay?: number;
}

interface WorldPanelProps {
  info: DebugInfo;
  onApplyDepthSettings?: (worldDepth: number, scaleDepth: number) => void;
  // Sun controls
  timeOfDay: number;
  onTimeOfDayChange: (time: number) => void;
  sunAutoMove: boolean;
  onSunAutoMoveChange: (auto: boolean) => void;
  sunSpeed: number;
  onSunSpeedChange: (speed: number) => void;
  // Speech feature
  speechEnabled: boolean;
  onSpeechEnabledChange: (enabled: boolean) => void;
}

export function WorldPanel({
  info,
  onApplyDepthSettings,
  timeOfDay,
  onTimeOfDayChange,
  sunAutoMove,
  onSunAutoMoveChange,
  sunSpeed,
  onSunSpeedChange,
  speechEnabled,
  onSpeechEnabledChange,
}: WorldPanelProps) {
  const [macroDepth, setMacroDepth] = useState(String(getMacroDepth()));
  const [microDepth, setMicroDepth] = useState(String(getMicroDepth()));

  // Subscribe to depth changes from config
  useEffect(() => {
    const unsubscribe = onDepthChange((newMacroDepth, newMicroDepth) => {
      setMacroDepth(String(newMacroDepth));
      setMicroDepth(String(newMicroDepth));
    });
    return unsubscribe;
  }, []);

  const getTimeOfDayLabel = (time: number): string => {
    if (time < 0.25) return 'Night';
    if (time < 0.35) return 'Dawn';
    if (time < 0.65) return 'Day';
    if (time < 0.75) return 'Dusk';
    return 'Night';
  };

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
      minWidth="320px"
      maxHeight="95vh"
      overflowY="auto"
    >
      <VStack align="stretch" spacing={0.5}>
        <Text fontWeight="bold" color="cyan.300">WORLD PANEL</Text>
        <Text>World: {info.worldSize ?? 'N/A'}×{info.worldSize ?? 'N/A'}×{info.worldSize ?? 'N/A'}</Text>
        <Text>Mode: {info.isEditMode ? 'EDIT' : 'WALK'}</Text>

        <Text color="yellow.300">─ Sun System ─</Text>
        <Box pointerEvents="auto">
          <HStack spacing={2} mb={1}>
            <Text minWidth="80px">Time of Day:</Text>
            <Text color="yellow.200" fontWeight="bold" fontSize="xs">
              {getTimeOfDayLabel(timeOfDay)}
            </Text>
          </HStack>
          <Slider
            value={timeOfDay}
            onChange={onTimeOfDayChange}
            min={0}
            max={1}
            step={0.01}
            size="sm"
            mb={1}
          >
            <SliderTrack bg="gray.700">
              <SliderFilledTrack bg="yellow.400" />
            </SliderTrack>
            <SliderThumb boxSize={3} />
          </Slider>
          <HStack spacing={2} justify="space-between" mb={1}>
            <Text minWidth="80px">Auto Move:</Text>
            <Switch
              isChecked={sunAutoMove}
              onChange={(e) => onSunAutoMoveChange(e.target.checked)}
              size="sm"
              colorScheme="cyan"
            />
          </HStack>
          {sunAutoMove && (
            <Box mb={1}>
              <HStack spacing={2} mb={1}>
                <Text minWidth="80px">Sun Speed:</Text>
                <Text color="cyan.200" fontSize="xs">{sunSpeed.toFixed(3)}x</Text>
              </HStack>
              <Slider
                value={sunSpeed}
                onChange={onSunSpeedChange}
                min={0.001}
                max={0.1}
                step={0.001}
                size="sm"
              >
                <SliderTrack bg="gray.700">
                  <SliderFilledTrack bg="cyan.400" />
                </SliderTrack>
                <SliderThumb boxSize={3} />
              </Slider>
            </Box>
          )}
        </Box>

        <Text color="yellow.300">─ Cursor ─</Text>
        <Text>World: {formatVec(info.cursorWorld)}</Text>
        <Text>Octree: {printCubeCoord(info.cursorOctree)}</Text>
        <Text>Depth: {info.cursorDepth ?? 'N/A'} (size: {formatNum(info.cursorSize)})</Text>

        <Text color="green.300">─ Avatar ─</Text>
        <Text>Pos: {formatVec(info.avatarPos)}</Text>

        <Text color="blue.300">─ Camera ─</Text>
        <Text>Pos: {formatVec(info.cameraPos)}</Text>

        <Text color="orange.300">─ World Settings ─</Text>
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
            mb={1}
          >
            Apply World Settings
          </Button>
        </Box>

        <Text color="purple.300">─ Features ─</Text>
        <Box pointerEvents="auto">
          <HStack spacing={2} justify="space-between">
            <Text>Speech:</Text>
            <Switch
              isChecked={speechEnabled}
              onChange={(e) => onSpeechEnabledChange(e.target.checked)}
              size="sm"
              colorScheme="purple"
            />
          </HStack>
        </Box>
      </VStack>
    </Box>
  );
}

// Re-export as DebugPanel for backward compatibility
export { WorldPanel as DebugPanel };
