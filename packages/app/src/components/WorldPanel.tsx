import { useState, useEffect } from 'react';
import { Box, Text, VStack, HStack, Slider, SliderTrack, SliderFilledTrack, SliderThumb, Switch, Badge, Popover, PopoverTrigger, PopoverContent, PopoverBody, Button } from '@chakra-ui/react';
import { CubeCoord } from '../types/cube-coord';
import { getMacroDepth, getMicroDepth, onDepthChange, setMacroDepth as setGlobalMacroDepth } from '../config/depth-config';

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
  // World Grid toggle
  worldGridVisible: boolean;
  onWorldGridVisibleChange: (visible: boolean) => void;
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
  worldGridVisible,
  onWorldGridVisibleChange,
}: WorldPanelProps) {
  const [macroDepth, setMacroDepth] = useState(getMacroDepth());
  const [microDepth, setMicroDepth] = useState(getMicroDepth());

  // Subscribe to depth changes from config
  useEffect(() => {
    const unsubscribe = onDepthChange((newMacroDepth, newMicroDepth) => {
      setMacroDepth(newMacroDepth);
      setMicroDepth(newMicroDepth);
    });
    return unsubscribe;
  }, []);

  const handleMacroChange = (newMacro: number) => {
    setMacroDepth(newMacro);
    setGlobalMacroDepth(newMacro);
    if (onApplyDepthSettings) {
      const totalDepth = newMacro + microDepth;
      onApplyDepthSettings(totalDepth, microDepth);
    }
  };

  const handleMicroChange = (newMicro: number) => {
    setMicroDepth(newMicro);
    if (onApplyDepthSettings) {
      const totalDepth = macroDepth + newMicro;
      onApplyDepthSettings(totalDepth, newMicro);
    }
  };

  const getTimeOfDayLabel = (time: number): string => {
    if (time < 0.25) return 'Night';
    if (time < 0.35) return 'Dawn';
    if (time < 0.65) return 'Day';
    if (time < 0.75) return 'Dusk';
    return 'Night';
  };

  const currentMacro = macroDepth;
  const currentMicro = microDepth;
  const worldSize = 1 << currentMacro; // 2^macro

  const formatNum = (n: number | undefined) => n?.toFixed(3) ?? 'N/A';
  const formatInt = (n: number | undefined) => n !== undefined ? Math.round(n).toString() : 'N/A';
  const formatVec = (v: { x: number; y: number; z: number } | undefined) =>
    v ? `${formatNum(v.x)}, ${formatNum(v.y)}, ${formatNum(v.z)}` : 'N/A';
  const formatVecInt = (v: { x: number; y: number; z: number } | undefined) =>
    v ? `${formatInt(v.x)}, ${formatInt(v.y)}, ${formatInt(v.z)}` : 'N/A';

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
    >
      <VStack align="stretch" spacing={1}>
        {/* World info with badges */}
        <HStack spacing={1}>
          <Text color="cyan.300">World</Text>

          {/* Macro depth selector */}
          <Popover placement="top">
            <PopoverTrigger>
              <Badge
                colorScheme="cyan"
                fontSize="xs"
                cursor="pointer"
                _hover={{ opacity: 0.8 }}
              >
                macro {currentMacro}
              </Badge>
            </PopoverTrigger>
            <PopoverContent bg="gray.800" borderColor="cyan.500" width="auto" pointerEvents="auto">
              <PopoverBody p={1}>
                <VStack spacing={1}>
                  {[8, 7, 6, 5, 4, 3, 2, 1].map((depth) => (
                    <Button
                      key={depth}
                      size="xs"
                      variant={currentMacro === depth ? 'solid' : 'ghost'}
                      colorScheme="cyan"
                      onClick={() => handleMacroChange(depth)}
                      width="100%"
                    >
                      {depth}
                    </Button>
                  ))}
                </VStack>
              </PopoverBody>
            </PopoverContent>
          </Popover>

          {/* Micro depth selector */}
          <Popover placement="top">
            <PopoverTrigger>
              <Badge
                colorScheme="cyan"
                fontSize="xs"
                cursor="pointer"
                _hover={{ opacity: 0.8 }}
              >
                micro {currentMicro}
              </Badge>
            </PopoverTrigger>
            <PopoverContent bg="gray.800" borderColor="cyan.500" width="auto" pointerEvents="auto">
              <PopoverBody p={1}>
                <VStack spacing={1}>
                  {[3, 2, 1, 0].map((depth) => (
                    <Button
                      key={depth}
                      size="xs"
                      variant={currentMicro === depth ? 'solid' : 'ghost'}
                      colorScheme="cyan"
                      onClick={() => handleMicroChange(depth)}
                      width="100%"
                    >
                      {depth}
                    </Button>
                  ))}
                </VStack>
              </PopoverBody>
            </PopoverContent>
          </Popover>

          <Badge colorScheme="blue" fontSize="xs">size {worldSize}×{worldSize}</Badge>
        </HStack>

        {/* Time of day */}
        <Box pointerEvents="auto">
          <HStack spacing={2} mb={1}>
            <Text color="yellow.300">Time</Text>
            <Text color="yellow.200" fontWeight="bold" fontSize="xs">
              {getTimeOfDayLabel(timeOfDay)}
            </Text>
            <Switch
              isChecked={sunAutoMove}
              onChange={(e) => onSunAutoMoveChange(e.target.checked)}
              size="sm"
              colorScheme="yellow"
              ml="auto"
            />
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
          {sunAutoMove && (
            <Box mb={1}>
              <HStack spacing={2} mb={1}>
                <Text color="yellow.300">Speed</Text>
                <Text color="yellow.200" fontSize="xs">{sunSpeed.toFixed(3)}x</Text>
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
                  <SliderFilledTrack bg="yellow.400" />
                </SliderTrack>
                <SliderThumb boxSize={3} />
              </Slider>
            </Box>
          )}
        </Box>

        {/* Cursor info with badge */}
        <HStack spacing={1}>
          <Text color="yellow.300">Cursor</Text>
          <Badge colorScheme="yellow" fontSize="xs">depth {info.cursorDepth ?? 'N/A'}</Badge>
          <Text fontSize="xs">({formatVecInt(info.cursorWorld)})</Text>
        </HStack>

        {/* Camera info */}
        <HStack spacing={1}>
          <Text color="blue.300">Camera</Text>
          <Text fontSize="xs">({formatVec(info.cameraPos)})</Text>
        </HStack>

        {/* Speech toggle */}
        <HStack spacing={2} justify="space-between" pointerEvents="auto">
          <Text color="purple.300">Speech</Text>
          <Switch
            isChecked={speechEnabled}
            onChange={(e) => onSpeechEnabledChange(e.target.checked)}
            size="sm"
            colorScheme="purple"
          />
        </HStack>

        {/* World Grid toggle */}
        <HStack spacing={2} justify="space-between" pointerEvents="auto">
          <Text color="green.300">World Grid</Text>
          <Switch
            isChecked={worldGridVisible}
            onChange={(e) => onWorldGridVisibleChange(e.target.checked)}
            size="sm"
            colorScheme="green"
          />
        </HStack>

      </VStack>
    </Box>
  );
}

// Re-export as DebugPanel for backward compatibility
export { WorldPanel as DebugPanel };
