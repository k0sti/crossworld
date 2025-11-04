import { useState, useEffect } from 'react';
import { Box, Text, VStack, HStack, Switch, Badge, Popover, PopoverTrigger, PopoverContent, PopoverBody, Button } from '@chakra-ui/react';
import { CubeCoord } from '../types/cube-coord';
import { getMacroDepth, getMicroDepth, getBorderDepth, onDepthChange, setMacroDepth as setGlobalMacroDepth, setMicroDepth as setGlobalMicroDepth, setBorderDepth as setGlobalBorderDepth } from '../config/depth-config';

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
  placementModel?: string | null;
  placementScale?: number;
}

interface WorldPanelProps {
  info: DebugInfo;
  onApplyDepthSettings?: (worldDepth: number, scaleDepth: number) => void;
  // Speech feature
  speechEnabled: boolean;
  onSpeechEnabledChange: (enabled: boolean) => void;
  // World Grid toggle
  worldGridVisible: boolean;
  onWorldGridVisibleChange: (visible: boolean) => void;
  // Face mesh toggle
  faceMeshEnabled: boolean;
  onFaceMeshEnabledChange: (enabled: boolean) => void;
  // Wireframe toggle
  wireframeEnabled: boolean;
  onWireframeEnabledChange: (enabled: boolean) => void;
  triangleCount?: number;
  // Textures toggle
  texturesEnabled: boolean;
  onTexturesEnabledChange: (enabled: boolean) => void;
  // Publish world
  onPublishWorld?: () => void;
  isLoggedIn?: boolean;
}

export function WorldPanel({
  info,
  onApplyDepthSettings,
  speechEnabled,
  onSpeechEnabledChange,
  worldGridVisible,
  onWorldGridVisibleChange,
  faceMeshEnabled,
  onFaceMeshEnabledChange,
  wireframeEnabled,
  onWireframeEnabledChange,
  triangleCount,
  texturesEnabled,
  onTexturesEnabledChange,
  onPublishWorld,
  isLoggedIn,
}: WorldPanelProps) {
  const [macroDepth, setMacroDepth] = useState(getMacroDepth());
  const [microDepth, setMicroDepth] = useState(getMicroDepth());
  const [borderDepth, setBorderDepth] = useState(getBorderDepth());

  // Subscribe to depth changes from config
  useEffect(() => {
    const unsubscribe = onDepthChange((newMacroDepth, newMicroDepth, newBorderDepth) => {
      setMacroDepth(newMacroDepth);
      setMicroDepth(newMicroDepth);
      setBorderDepth(newBorderDepth);
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
    setGlobalMicroDepth(newMicro);
    if (onApplyDepthSettings) {
      const totalDepth = macroDepth + newMicro;
      onApplyDepthSettings(totalDepth, newMicro);
    }
  };

  const handleBorderChange = (newBorder: number) => {
    setBorderDepth(newBorder);
    setGlobalBorderDepth(newBorder);
    if (onApplyDepthSettings) {
      const totalDepth = macroDepth + microDepth;
      onApplyDepthSettings(totalDepth, microDepth);
    }
  };

  const currentMacro = macroDepth;
  const currentMicro = microDepth;
  const currentBorder = borderDepth;
  const worldSize = 1 << currentMacro; // 2^macro (world size independent of micro)

  const formatNum = (n: number | undefined) => n?.toFixed(3) ?? 'N/A';
  const formatInt = (n: number | undefined) => n !== undefined ? Math.round(n).toString() : 'N/A';
  const formatVec = (v: { x: number; y: number; z: number } | undefined) =>
    v ? `${formatNum(v.x)}, ${formatNum(v.y)}, ${formatNum(v.z)}` : 'N/A';
  const formatVecInt = (v: { x: number; y: number; z: number } | undefined) =>
    v ? `${formatInt(v.x)}, ${formatInt(v.y)}, ${formatInt(v.z)}` : 'N/A';

  return (
    <Box
      position="fixed"
      top="60px"
      left={2}
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
            {({ onClose }) => (
              <>
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
                          onClick={() => {
                            handleMacroChange(depth);
                            onClose();
                          }}
                          width="100%"
                        >
                          {depth}
                        </Button>
                      ))}
                    </VStack>
                  </PopoverBody>
                </PopoverContent>
              </>
            )}
          </Popover>

          {/* Micro depth selector */}
          <Popover placement="top">
            {({ onClose }) => (
              <>
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
                          onClick={() => {
                            handleMicroChange(depth);
                            onClose();
                          }}
                          width="100%"
                        >
                          {depth}
                        </Button>
                      ))}
                    </VStack>
                  </PopoverBody>
                </PopoverContent>
              </>
            )}
          </Popover>

          {/* Border depth selector */}
          <Popover placement="top">
            {({ onClose }) => (
              <>
                <PopoverTrigger>
                  <Badge
                    colorScheme="purple"
                    fontSize="xs"
                    cursor="pointer"
                    _hover={{ opacity: 0.8 }}
                  >
                    border {currentBorder}
                  </Badge>
                </PopoverTrigger>
                <PopoverContent bg="gray.800" borderColor="purple.500" width="auto" pointerEvents="auto">
                  <PopoverBody p={1}>
                    <VStack spacing={1}>
                      {[5, 4, 3, 2, 1, 0].map((depth) => (
                        <Button
                          key={depth}
                          size="xs"
                          variant={currentBorder === depth ? 'solid' : 'ghost'}
                          colorScheme="purple"
                          onClick={() => {
                            handleBorderChange(depth);
                            onClose();
                          }}
                          width="100%"
                        >
                          {depth}
                        </Button>
                      ))}
                    </VStack>
                  </PopoverBody>
                </PopoverContent>
              </>
            )}
          </Popover>

          <Badge colorScheme="blue" fontSize="xs">size {worldSize}</Badge>
        </HStack>

        {/* Cursor info with badge */}
        <HStack spacing={1}>
          <Text color="yellow.300">Cursor</Text>
          <Badge colorScheme="yellow" fontSize="xs">depth {info.cursorDepth ?? 'N/A'}</Badge>
          <Text fontSize="xs">({formatVecInt(info.cursorWorld)})</Text>
        </HStack>

        {/* Placement Model info (only in placement mode) */}
        {info.placementModel && (
          <HStack spacing={1}>
            <Text color="orange.300">Model</Text>
            <Badge colorScheme="orange" fontSize="xs">scale {info.placementScale ?? 0}</Badge>
            <Text fontSize="xs" isTruncated maxWidth="150px">{info.placementModel}</Text>
          </HStack>
        )}

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

        {/* Face Mesh toggle with triangle count */}
        <HStack spacing={2} justify="space-between" pointerEvents="auto">
          <HStack spacing={1}>
            <Text color="orange.300">Face Mesh</Text>
            {triangleCount !== undefined && (
              <Badge colorScheme="orange" fontSize="xs">
                {triangleCount.toLocaleString()} tris
              </Badge>
            )}
          </HStack>
          <Switch
            isChecked={faceMeshEnabled}
            onChange={(e) => onFaceMeshEnabledChange(e.target.checked)}
            size="sm"
            colorScheme="orange"
          />
        </HStack>

        {/* Wireframe toggle */}
        <HStack spacing={2} justify="space-between" pointerEvents="auto">
          <Text color="pink.300">Wireframe</Text>
          <Switch
            isChecked={wireframeEnabled}
            onChange={(e) => onWireframeEnabledChange(e.target.checked)}
            size="sm"
            colorScheme="pink"
          />
        </HStack>

        {/* Textures toggle */}
        <HStack spacing={2} justify="space-between" pointerEvents="auto">
          <Text color="teal.300">Textures</Text>
          <Switch
            isChecked={texturesEnabled}
            onChange={(e) => onTexturesEnabledChange(e.target.checked)}
            size="sm"
            colorScheme="teal"
          />
        </HStack>

        {/* Publish World button (only visible when logged in) */}
        {isLoggedIn && onPublishWorld && (
          <Box
            as="button"
            onClick={onPublishWorld}
            width="100%"
            py={2}
            px={2}
            bg="rgba(80, 80, 80, 0.3)"
            borderRadius="md"
            border="1px solid rgba(255, 255, 255, 0.1)"
            _hover={{
              bg: 'rgba(100, 100, 100, 0.4)',
              borderColor: 'rgba(255, 255, 255, 0.2)',
            }}
            cursor="pointer"
            transition="all 0.1s"
            pointerEvents="auto"
            mt={1}
          >
            <Text color="white" fontSize="sm" textAlign="center">
              Publish
            </Text>
          </Box>
        )}
      </VStack>
    </Box>
  );
}

// Re-export as DebugPanel for backward compatibility
export { WorldPanel as DebugPanel };
