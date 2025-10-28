import * as logger from '../utils/logger';
import { VStack, Text, Input, Button, HStack, IconButton, useToast, Tooltip, InputGroup, InputRightElement, Badge, Box } from '@chakra-ui/react'
import { useState, useEffect, useRef } from 'react'
import { FiPlus, FiTrash2, FiRefreshCw } from 'react-icons/fi'
import { Relay } from 'applesauce-relay'
import { DEFAULT_RELAYS, DEFAULT_RELAY_STATES } from '../config'
import { ResponsivePanel } from './ResponsivePanel'

interface RelayStats {
  sent: number
  received: number
}

interface RelayConfig {
  url: string
  enabledForProfile: boolean
  enabledForWorld: boolean
  status: 'connected' | 'connecting' | 'error' | 'disconnected'
  errorMessage?: string
  stats?: RelayStats
}

interface NetworkConfigPanelProps {
  isOpen: boolean
  onClose: () => void
}

export function NetworkConfigPanel({ isOpen, onClose }: NetworkConfigPanelProps) {
  const toast = useToast()
  const [relays, setRelays] = useState<RelayConfig[]>([])
  const [newRelay, setNewRelay] = useState('')
  const [editingRelays, setEditingRelays] = useState<Set<string>>(new Set())
  const relayInstances = useRef<Map<string, Relay>>(new Map())

  useEffect(() => {
    const loadRelays = () => {
      const savedRelays = localStorage.getItem('crossworld_relays')

      if (savedRelays) {
        const loaded = JSON.parse(savedRelays) as RelayConfig[]
        // Reset status to disconnected on load
        setRelays(loaded.map(r => ({ ...r, status: 'disconnected' as const, stats: { sent: 0, received: 0 } })))
      } else {
        const defaultConfigs: RelayConfig[] = DEFAULT_RELAYS.map(url => {
          const defaults = DEFAULT_RELAY_STATES[url as keyof typeof DEFAULT_RELAY_STATES]
          return {
            url,
            enabledForProfile: defaults?.enabledForProfile ?? true,
            enabledForWorld: defaults?.enabledForWorld ?? true,
            status: 'disconnected' as const,
            stats: { sent: 0, received: 0 }
          }
        })
        setRelays(defaultConfigs)
        localStorage.setItem('crossworld_relays', JSON.stringify(defaultConfigs))
      }
    }

    loadRelays()

    // Cleanup on unmount
    return () => {
      relayInstances.current.forEach(relay => {
        try {
          relay.close()
        } catch (e) {
          logger.error('ui', 'Error closing relay:', e)
        }
      })
      relayInstances.current.clear()
    }
  }, [])

  // Connect/disconnect relays based on enabled state (enabled if either profile or chat is enabled)
  useEffect(() => {
    relays.forEach(relay => {
      const instance = relayInstances.current.get(relay.url)
      const shouldBeEnabled = relay.enabledForProfile || relay.enabledForWorld

      if (shouldBeEnabled && !instance) {
        // Connect
        connectRelay(relay.url)
      } else if (!shouldBeEnabled && instance) {
        // Disconnect
        disconnectRelay(relay.url)
      }
    })
  }, [relays])

  const connectRelay = async (url: string) => {
    setRelays(prev => prev.map(r =>
      r.url === url ? { ...r, status: 'connecting' as const } : r
    ))

    try {
      const relay = new Relay(url)
      relayInstances.current.set(url, relay)

      // Test connection with a simple subscription
      const sub = relay.request({ kinds: [1], limit: 1 }).subscribe({
        next: () => {
          setRelays(prev => prev.map(r =>
            r.url === url ? {
              ...r,
              status: 'connected' as const,
              stats: { sent: r.stats?.sent || 0, received: (r.stats?.received || 0) + 1 }
            } : r
          ))
          // Unsubscribe after first event
          try { sub.unsubscribe() } catch (e) {}
        },
        error: (err: any) => {
          const errorMsg = err?.message || String(err) || 'Connection failed'
          setRelays(prev => prev.map(r =>
            r.url === url ? {
              ...r,
              status: 'error' as const,
              errorMessage: errorMsg
            } : r
          ))
        }
      })

      // Set timeout for connection
      setTimeout(() => {
        setRelays(prev => {
          const current = prev.find(r => r.url === url)
          if (current && current.status === 'connecting') {
            return prev.map(r =>
              r.url === url ? {
                ...r,
                status: 'connected' as const
              } : r
            )
          }
          return prev
        })
      }, 2000)

    } catch (error: any) {
      const errorMsg = error?.message || String(error) || 'Connection failed'
      setRelays(prev => prev.map(r =>
        r.url === url ? {
          ...r,
          status: 'error' as const,
          errorMessage: errorMsg
        } : r
      ))
    }
  }

  const disconnectRelay = (url: string) => {
    const instance = relayInstances.current.get(url)
    if (instance) {
      try {
        instance.close()
      } catch (e) {
        logger.error('ui', `Error closing relay ${url}:`, e)
      }
      relayInstances.current.delete(url)
    }

    setRelays(prev => prev.map(r =>
      r.url === url ? { ...r, status: 'disconnected' as const } : r
    ))
  }

  const saveRelays = (relays: RelayConfig[]) => {
    localStorage.setItem('crossworld_relays', JSON.stringify(relays))
    // Dispatch custom event to notify other components (like ChatPanel)
    window.dispatchEvent(new Event('relayConfigChanged'))
  }

  const handleToggleProfile = (relay: RelayConfig) => {
    const updatedRelay: RelayConfig = {
      ...relay,
      enabledForProfile: !relay.enabledForProfile,
    }

    const updated = relays.map(r => r.url === relay.url ? updatedRelay : r)
    setRelays(updated)
    saveRelays(updated)

    toast({
      title: relay.enabledForProfile ? 'Profile Disabled' : 'Profile Enabled',
      description: `${relay.url} ${relay.enabledForProfile ? 'disabled' : 'enabled'} for profiles`,
      status: 'info',
      duration: 2000,
    })
  }

  const handleToggleChat = (relay: RelayConfig) => {
    const updatedRelay: RelayConfig = {
      ...relay,
      enabledForWorld: !relay.enabledForWorld,
    }

    const updated = relays.map(r => r.url === relay.url ? updatedRelay : r)
    setRelays(updated)
    saveRelays(updated)

    toast({
      title: relay.enabledForWorld ? 'World Disabled' : 'World Enabled',
      description: `${relay.url} ${relay.enabledForWorld ? 'disabled' : 'enabled'} for world`,
      status: 'info',
      duration: 2000,
    })
  }

  const handleUpdateRelayUrl = (oldUrl: string, newUrl: string) => {
    if (!newUrl || newUrl === oldUrl) {
      setEditingRelays(prev => {
        const next = new Set(prev)
        next.delete(oldUrl)
        return next
      })
      return
    }

    // Disconnect old relay
    disconnectRelay(oldUrl)

    const updated = relays.map(r =>
      r.url === oldUrl ? { ...r, url: newUrl, status: 'disconnected' as const } : r
    )

    setRelays(updated)
    saveRelays(updated)

    setEditingRelays(prev => {
      const next = new Set(prev)
      next.delete(oldUrl)
      return next
    })

    toast({
      title: 'Relay Updated',
      description: 'Relay URL has been updated',
      status: 'success',
      duration: 2000,
    })
  }

  const handleDeleteRelay = (url: string) => {
    disconnectRelay(url)
    const updated = relays.filter(r => r.url !== url)
    setRelays(updated)
    saveRelays(updated)

    toast({
      title: 'Relay Removed',
      description: 'Relay has been removed from the list',
      status: 'info',
      duration: 2000,
    })
  }

  const handleAddRelay = () => {
    if (!newRelay) {
      toast({
        title: 'URL Required',
        description: 'Please enter a relay URL',
        status: 'warning',
        duration: 2000,
      })
      return
    }

    const newRelayConfig: RelayConfig = {
      url: newRelay,
      enabledForProfile: true,
      enabledForWorld: true,
      status: 'disconnected',
      stats: { sent: 0, received: 0 }
    }

    const updated = [...relays, newRelayConfig]
    setRelays(updated)
    saveRelays(updated)
    setNewRelay('')

    toast({
      title: 'Relay Added',
      description: 'New relay has been added to the list',
      status: 'success',
      duration: 2000,
    })
  }

  const handleResetDefaults = () => {
    // Disconnect all current relays
    relayInstances.current.forEach((relay, url) => {
      try {
        relay.close()
      } catch (e) {
        logger.error('ui', `Error closing relay ${url}:`, e)
      }
    })
    relayInstances.current.clear()

    const defaultConfigs: RelayConfig[] = DEFAULT_RELAYS.map(url => {
      const defaults = DEFAULT_RELAY_STATES[url as keyof typeof DEFAULT_RELAY_STATES]
      return {
        url,
        enabledForProfile: defaults?.enabledForProfile ?? true,
        enabledForWorld: defaults?.enabledForWorld ?? true,
        status: 'disconnected' as const,
        stats: { sent: 0, received: 0 }
      }
    })

    setRelays(defaultConfigs)
    saveRelays(defaultConfigs)

    toast({
      title: 'Reset to Defaults',
      description: 'Relays have been reset to defaults',
      status: 'info',
      duration: 2000,
    })
  }

  const getStatusColor = (status: RelayConfig['status']) => {
    switch (status) {
      case 'connected': return '#48bb78' // green
      case 'connecting': return '#ed8936' // orange
      case 'error': return '#f56565' // red
      case 'disconnected': return '#718096' // gray
      default: return '#718096'
    }
  }

  const getTooltipLabel = (relay: RelayConfig) => {
    switch (relay.status) {
      case 'connected':
        return `Connected - Sent: ${relay.stats?.sent || 0}, Received: ${relay.stats?.received || 0}`
      case 'error':
        return relay.errorMessage || 'Connection error'
      case 'connecting':
        return 'Connecting...'
      case 'disconnected':
        return 'Not connected'
      default:
        return relay.status
    }
  }

  const renderRelayRow = (relay: RelayConfig) => {
    const isEditing = editingRelays.has(relay.url)
    const statusColor = getStatusColor(relay.status)

    return (
      <HStack key={relay.url} p={2} bg="rgba(255, 255, 255, 0.03)" borderRadius="md" spacing={2}>
        <Tooltip
          label={getTooltipLabel(relay)}
          placement="top"
        >
          <Box
            w="12px"
            h="12px"
            borderRadius="full"
            bg={statusColor}
            cursor="help"
            flexShrink={0}
          />
        </Tooltip>

        <HStack spacing={1} flexShrink={0}>
          <Badge
            cursor="pointer"
            onClick={() => handleToggleProfile(relay)}
            fontSize="2xs"
            px={2}
            py={0.5}
            borderRadius="md"
            bg={relay.enabledForProfile ? "blue.500" : "rgba(255, 255, 255, 0.1)"}
            color={relay.enabledForProfile ? "white" : "whiteAlpha.600"}
            _hover={{ opacity: 0.8 }}
          >
            profile
          </Badge>
          <Badge
            cursor="pointer"
            onClick={() => handleToggleChat(relay)}
            fontSize="2xs"
            px={2}
            py={0.5}
            borderRadius="md"
            bg={relay.enabledForWorld ? "green.500" : "rgba(255, 255, 255, 0.1)"}
            color={relay.enabledForWorld ? "white" : "whiteAlpha.600"}
            _hover={{ opacity: 0.8 }}
          >
            world
          </Badge>
        </HStack>

        <Box flex={1}>
          {isEditing ? (
            <InputGroup size="sm">
              <Input
                defaultValue={relay.url}
                onBlur={(e) => handleUpdateRelayUrl(relay.url, e.target.value)}
                onKeyPress={(e) => {
                  if (e.key === 'Enter') {
                    handleUpdateRelayUrl(relay.url, e.currentTarget.value)
                  }
                }}
                autoFocus
                bg="rgba(255, 255, 255, 0.05)"
                border="1px solid rgba(255, 255, 255, 0.1)"
                color="white"
                fontSize="xs"
                fontFamily="monospace"
              />
              <InputRightElement>
                <IconButton
                  aria-label="Cancel edit"
                  icon={<Text>✕</Text>}
                  size="xs"
                  variant="ghost"
                  onClick={() => setEditingRelays(prev => {
                    const next = new Set(prev)
                    next.delete(relay.url)
                    return next
                  })}
                />
              </InputRightElement>
            </InputGroup>
          ) : (
            <Text
              fontSize="xs"
              fontFamily="monospace"
              color="whiteAlpha.900"
              cursor="pointer"
              onClick={() => setEditingRelays(prev => new Set(prev).add(relay.url))}
              _hover={{ textDecoration: 'underline' }}
            >
              {relay.url}
            </Text>
          )}
        </Box>

        <IconButton
          aria-label="Delete relay"
          icon={<FiTrash2 />}
          size="xs"
          variant="ghost"
          color="whiteAlpha.700"
          onClick={() => handleDeleteRelay(relay.url)}
        />
      </HStack>
    )
  }

  return (
    <ResponsivePanel
      isOpen={isOpen}
      onClose={onClose}
      forceFullscreen={true}
      title="🌐 Relays"
      actions={
        <HStack spacing={3}>
          <Button
            onClick={handleResetDefaults}
            size="sm"
            variant="outline"
            leftIcon={<FiRefreshCw />}
            color="white"
          >
            Defaults
          </Button>
          <Button onClick={onClose} size="sm" colorScheme="blue">
            Done
          </Button>
        </HStack>
      }
    >
      <VStack align="stretch" gap={3} maxW="800px" mx="auto">

        {relays.map(relay => renderRelayRow(relay))}

        <HStack>
          <Input
            placeholder="wss://relay.example.com"
            value={newRelay}
            onChange={(e) => setNewRelay(e.target.value)}
            onKeyPress={(e) => e.key === 'Enter' && handleAddRelay()}
            bg="rgba(255, 255, 255, 0.05)"
            border="1px solid rgba(255, 255, 255, 0.1)"
            color="white"
            _placeholder={{ color: 'whiteAlpha.500' }}
            fontSize="sm"
          />
          <Button
            onClick={handleAddRelay}
            colorScheme="blue"
            size="sm"
            leftIcon={<FiPlus />}
          >
            Add
          </Button>
        </HStack>

        <HStack justify="flex-end" pt={2}>
          <Text fontSize="xs" color="whiteAlpha.600">
            {relays.filter(r => r.status === 'connected').length} connected
          </Text>
        </HStack>
      </VStack>
    </ResponsivePanel>
  )
}
