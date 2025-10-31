import { Box, Text, VStack, HStack, Slider, SliderTrack, SliderFilledTrack, SliderThumb, Switch, Popover, PopoverTrigger, PopoverContent, PopoverBody } from '@chakra-ui/react'
import { useState, useEffect } from 'react'
import type { ConfigPanelType } from '@crossworld/common'
import { type LogTag, isLogEnabled, setLogEnabled, subscribeToLogConfig, isMasterLogEnabled, setMasterLogEnabled } from '../utils/logger'

export type { ConfigPanelType } from '@crossworld/common'

const LOG_TAGS: Array<{ tag: LogTag; label: string }> = [
  { tag: 'common', label: 'Common' },
  { tag: 'avatar', label: 'Avatar' },
  { tag: 'geometry', label: 'Geometry' },
  { tag: 'renderer', label: 'Renderer' },
  { tag: 'voice', label: 'Voice' },
  { tag: 'storage', label: 'Storage' },
  { tag: 'network', label: 'Network' },
  { tag: 'ui', label: 'UI' },
  { tag: 'worker', label: 'Worker' },
  { tag: 'profile', label: 'Profile' },
  { tag: 'service', label: 'Service' },
]

interface ConfigButtonProps {
  label: string
  onClick: () => void
}

function ConfigButton({ label, onClick }: ConfigButtonProps) {
  return (
    <Box
      as="button"
      onClick={onClick}
      w="100%"
      py={2}
      px={3}
      bg="rgba(80, 80, 80, 0.3)"
      border="1px solid rgba(255, 255, 255, 0.1)"
      borderRadius="md"
      _hover={{
        bg: 'rgba(120, 120, 120, 0.4)',
        borderColor: 'rgba(255, 255, 255, 0.2)'
      }}
      _active={{
        bg: 'rgba(60, 60, 60, 0.3)',
      }}
      transition="all 0.1s"
      cursor="pointer"
      textAlign="center"
    >
      <Text fontSize="sm" color="white" fontWeight="medium">{label}</Text>
    </Box>
  )
}

interface ConfigPanelProps {
  onClose: () => void
  onOpenPanel: (type: ConfigPanelType) => void
  timeOfDay: number
  onTimeOfDayChange: (time: number) => void
  sunAutoMove: boolean
  onSunAutoMoveChange: (auto: boolean) => void
  sunSpeed: number
  onSunSpeedChange: (speed: number) => void
}

export function ConfigPanel({
  onClose,
  onOpenPanel,
  timeOfDay,
  onTimeOfDayChange,
  sunAutoMove,
  onSunAutoMoveChange,
  sunSpeed,
  onSunSpeedChange,
}: ConfigPanelProps) {
  const [enabledTags, setEnabledTags] = useState<Set<LogTag>>(new Set())
  const [masterEnabled, setMasterEnabled] = useState(isMasterLogEnabled())

  // Subscribe to log config changes
  useEffect(() => {
    const updateConfig = () => {
      const enabled = new Set<LogTag>()
      LOG_TAGS.forEach(({ tag }) => {
        if (isLogEnabled(tag)) {
          enabled.add(tag)
        }
      })
      setEnabledTags(enabled)
      setMasterEnabled(isMasterLogEnabled())
    }

    updateConfig()
    const unsubscribe = subscribeToLogConfig(updateConfig)
    return unsubscribe
  }, [])

  const handleOpenPanel = (type: ConfigPanelType) => {
    // Close config panel and open the selected panel
    onClose()
    onOpenPanel(type)
  }

  const handleToggleTag = (tag: LogTag) => {
    setLogEnabled(tag, !enabledTags.has(tag))
  }

  const handleToggleMaster = () => {
    setMasterLogEnabled(!masterEnabled)
  }

  const getTimeOfDayLabel = (time: number): string => {
    if (time < 0.25) return 'Night';
    if (time < 0.35) return 'Dawn';
    if (time < 0.65) return 'Day';
    if (time < 0.75) return 'Dusk';
    return 'Night';
  };

  return (
    <Box
      position="fixed"
      top="60px"
      right={2}
      zIndex={1500}
      bg="rgba(0, 0, 0, 0.75)"
      color="white"
      p={2}
      borderRadius="md"
      fontSize="xs"
      fontFamily="monospace"
      userSelect="none"
      minWidth="320px"
    >
      <VStack align="stretch" spacing={2}>
        {/* Navigation Buttons */}
        <VStack spacing={1} align="stretch">
          <ConfigButton
            label="Network"
            onClick={() => handleOpenPanel('network')}
          />
          <ConfigButton
            label="Avatar"
            onClick={() => handleOpenPanel('avatar')}
          />
          <ConfigButton
            label="Info"
            onClick={() => handleOpenPanel('info')}
          />
        </VStack>

        {/* Time of day controls */}
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

        {/* Logging Section */}
        <Popover placement="left">
          <PopoverTrigger>
            <HStack
              as="button"
              justify="space-between"
              p={2}
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
            >
              <Text color="white" fontSize="sm">
                Logging
              </Text>
              <Switch
                isChecked={masterEnabled}
                onChange={(e) => {
                  e.stopPropagation()
                  handleToggleMaster()
                }}
                size="sm"
                colorScheme={masterEnabled ? 'green' : 'red'}
                pointerEvents="auto"
              />
            </HStack>
          </PopoverTrigger>
          <PopoverContent bg="gray.800" borderColor="cyan.500" width="280px">
            <PopoverBody p={2}>
              <VStack align="stretch" gap={1}>
                {LOG_TAGS.map(({ tag, label }) => {
                  const isEnabled = enabledTags.has(tag)
                  return (
                    <HStack
                      key={tag}
                      p={2}
                      bg="rgba(80, 80, 80, 0.3)"
                      borderRadius="md"
                      border="1px solid rgba(255, 255, 255, 0.1)"
                      justify="space-between"
                      opacity={masterEnabled ? 1 : 0.5}
                    >
                      <Text color="white" fontSize="xs">
                        {label}
                      </Text>
                      <Switch
                        isChecked={isEnabled}
                        onChange={() => handleToggleTag(tag)}
                        size="sm"
                        colorScheme={isEnabled ? 'green' : 'gray'}
                        isDisabled={!masterEnabled}
                      />
                    </HStack>
                  )
                })}
              </VStack>
            </PopoverBody>
          </PopoverContent>
        </Popover>
      </VStack>
    </Box>
  )
}
