import { Box, VStack, Text, Input, Button, HStack, IconButton, useToast, Switch, Tooltip, Badge, InputGroup, InputRightElement } from '@chakra-ui/react'
import { useState, useEffect } from 'react'
import { FiPlus, FiTrash2, FiRefreshCw, FiCheck, FiAlertCircle, FiLoader } from 'react-icons/fi'
import { DEFAULT_RELAYS } from '../config'

interface RelayConfig {
  url: string
  enabled: boolean
  status: 'connected' | 'connecting' | 'error' | 'disconnected'
  errorMessage?: string
}

interface NetworkConfigPanelProps {
  onClose: () => void
}

export function NetworkConfigPanel({ onClose }: NetworkConfigPanelProps) {
  const toast = useToast()
  const [relays, setRelays] = useState<RelayConfig[]>([])
  const [newRelay, setNewRelay] = useState('')
  const [editingRelays, setEditingRelays] = useState<Set<string>>(new Set())

  useEffect(() => {
    const loadRelays = () => {
      const savedRelays = localStorage.getItem('crossworld_relays')

      if (savedRelays) {
        setRelays(JSON.parse(savedRelays) as RelayConfig[])
      } else {
        const defaultConfigs: RelayConfig[] = DEFAULT_RELAYS.map(url => ({
          url,
          enabled: true,
          status: 'disconnected' as const
        }))
        setRelays(defaultConfigs)
        localStorage.setItem('crossworld_relays', JSON.stringify(defaultConfigs))
      }
    }

    loadRelays()
  }, [])

  const saveRelays = (relays: RelayConfig[]) => {
    localStorage.setItem('crossworld_relays', JSON.stringify(relays))
  }

  const handleToggleRelay = (relay: RelayConfig) => {
    const newStatus: RelayConfig['status'] = relay.enabled ? 'disconnected' : 'connected'
    const updatedRelay: RelayConfig = { ...relay, enabled: !relay.enabled, status: newStatus }

    const updated = relays.map(r => r.url === relay.url ? updatedRelay : r)
    setRelays(updated)
    saveRelays(updated)

    toast({
      title: relay.enabled ? 'Relay Disabled' : 'Relay Enabled',
      description: `${relay.url} ${relay.enabled ? 'disabled' : 'enabled'}`,
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

    const updated = relays.map(r =>
      r.url === oldUrl ? { ...r, url: newUrl } : r
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
      enabled: true,
      status: 'disconnected'
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
    const defaultConfigs: RelayConfig[] = DEFAULT_RELAYS.map(url => ({
      url,
      enabled: true,
      status: 'disconnected' as const
    }))

    setRelays(defaultConfigs)
    saveRelays(defaultConfigs)

    toast({
      title: 'Reset to Defaults',
      description: 'Relays have been reset to defaults',
      status: 'info',
      duration: 2000,
    })
  }

  const renderRelayRow = (relay: RelayConfig) => {
    const isEditing = editingRelays.has(relay.url)
    const statusColor = relay.status === 'connected' ? 'green' :
                       relay.status === 'error' ? 'red' :
                       relay.status === 'connecting' ? 'yellow' : 'gray'

    const statusIcon = relay.status === 'connected' ? <FiCheck /> :
                      relay.status === 'error' ? <FiAlertCircle /> :
                      relay.status === 'connecting' ? <FiLoader /> : null

    return (
      <HStack key={relay.url} p={2} bg="rgba(255, 255, 255, 0.03)" borderRadius="md">
        <Switch
          size="sm"
          isChecked={relay.enabled}
          onChange={() => handleToggleRelay(relay)}
        />

        <Tooltip
          label={relay.status === 'error' ? relay.errorMessage : relay.status}
          placement="top"
        >
          <Badge colorScheme={statusColor} display="flex" alignItems="center" gap={1}>
            {statusIcon}
            <Text fontSize="2xs">{relay.status}</Text>
          </Badge>
        </Tooltip>

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
    <Box
      position="fixed"
      top="220px"
      left="50%"
      transform="translateX(-50%)"
      zIndex={1500}
      bg="rgba(0, 0, 0, 0.1)"
      backdropFilter="blur(8px)"
      p={4}
      minW="400px"
      maxW="500px"
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
      <VStack align="stretch" gap={3}>
        <Text fontSize="md" fontWeight="semibold" color="white">🌐 Network</Text>

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

        <HStack justify="space-between" pt={2}>
          <Button
            onClick={handleResetDefaults}
            size="xs"
            variant="ghost"
            leftIcon={<FiRefreshCw />}
            color="whiteAlpha.700"
            _hover={{ color: 'white', bg: 'rgba(255, 255, 255, 0.1)' }}
          >
            Reset to Defaults
          </Button>
          <Text fontSize="xs" color="whiteAlpha.600">
            Changes take effect on reconnect
          </Text>
        </HStack>
      </VStack>
    </Box>
  )
}
