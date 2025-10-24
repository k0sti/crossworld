import { Box, VStack, HStack, Text, Avatar, Tooltip, Badge, Wrap } from '@chakra-ui/react'
import { useState, useEffect, useCallback } from 'react'
import { FiMapPin, FiMessageSquare, FiCompass, FiEdit3, FiMic, FiHeadphones } from 'react-icons/fi'
import { type AvatarStateService, type AvatarState } from '../services/avatar-state'
import { profileCache } from '../services/profile-cache'
import { DEFAULT_RELAYS } from '../config'

interface ClientListPanelProps {
  isOpen: boolean
  statusService: AvatarStateService
  onOpenProfile?: (pubkey: string) => void
}

interface RelayConfig {
  url: string
  enabledForProfile: boolean
  enabledForWorld: boolean
  status: 'connected' | 'connecting' | 'error' | 'disconnected'
}

export function ClientListPanel({ isOpen, statusService, onOpenProfile }: ClientListPanelProps) {
  const [clients, setClients] = useState<Map<string, AvatarState>>(new Map())
  const [enabledRelays, setEnabledRelays] = useState<string[]>([])

  // Load enabled relays from localStorage
  useEffect(() => {
    const loadEnabledRelays = () => {
      try {
        const savedRelays = localStorage.getItem('crossworld_relays')
        if (savedRelays) {
          const relays = JSON.parse(savedRelays) as RelayConfig[]
          const enabled = relays.filter(r => r.enabledForProfile).map(r => r.url)
          setEnabledRelays(enabled)
        } else {
          setEnabledRelays(DEFAULT_RELAYS)
        }
      } catch (error) {
        console.error('Failed to load relay config:', error)
        setEnabledRelays(DEFAULT_RELAYS)
      }
    }

    loadEnabledRelays()

    // Listen for relay config changes
    const handleRelayConfigChanged = () => {
      loadEnabledRelays()
      // Clear profile cache when relay config changes
      profileCache.clearCache()
    }

    window.addEventListener('relayConfigChanged', handleRelayConfigChanged)

    return () => {
      window.removeEventListener('relayConfigChanged', handleRelayConfigChanged)
    }
  }, [])

  // Fetch profile metadata for a pubkey using cache
  const fetchProfile = useCallback(async (pubkey: string) => {
    if (enabledRelays.length === 0) return

    // Use profile cache to prevent duplicate fetches
    await profileCache.getProfile(pubkey, enabledRelays)
  }, [enabledRelays])

  // Subscribe to client changes
  useEffect(() => {
    const unsubscribe = statusService.onChange((clientsMap) => {
      setClients(clientsMap)
      // Fetch profiles for all new clients
      clientsMap.forEach((client) => {
        if (!profileCache.isCached(client.pubkey)) {
          fetchProfile(client.pubkey).catch(console.error)
        }
      })
    })

    // Get initial clients
    setClients(statusService.getUserStates())

    return unsubscribe
    // Note: fetchProfile and profilesVersion intentionally excluded to prevent re-fetch loop
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [statusService, enabledRelays])

  const clientList = Array.from(clients.values()).sort((a, b) => {
    // Sort alphabetically by npub
    return a.npub.localeCompare(b.npub)
  })

  const getDisplayName = (client: AvatarState): string => {
    const profile = profileCache.getCached(client.pubkey)
    return profile?.display_name || profile?.name || client.npub.slice(0, 12) + '...'
  }

  const getProfilePicture = (client: AvatarState): string | undefined => {
    const profile = profileCache.getCached(client.pubkey)
    return profile?.picture
  }

  const formatPosition = (pos?: { x: number; y: number; z: number }): string => {
    if (!pos) return ''
    return `(${Math.round(pos.x)}, ${Math.round(pos.y)}, ${Math.round(pos.z)})`
  }

  const getTimeSince = (timestamp: number): string => {
    const now = Math.floor(Date.now() / 1000)
    const diff = now - timestamp

    if (diff < 60) return 'just now'
    if (diff < 3600) return `${Math.floor(diff / 60)}m ago`
    if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`
    return `${Math.floor(diff / 86400)}d ago`
  }

  if (!isOpen) return null

  return (
    <Box
      position="fixed"
      right="20px"
      top="80px"
      width="360px"
      maxHeight="calc(100vh - 100px)"
      bg="rgba(0, 0, 0, 0.85)"
      backdropFilter="blur(10px)"
      borderRadius="12px"
      border="1px solid rgba(255, 255, 255, 0.1)"
      boxShadow="0 8px 32px rgba(0, 0, 0, 0.4)"
      color="white"
      zIndex={1000}
      overflow="hidden"
      display="flex"
      flexDirection="column"
    >
      {/* Header */}
      <HStack
        p={4}
        borderBottom="1px solid rgba(255, 255, 255, 0.1)"
        justify="space-between"
      >
        <HStack spacing={2}>
          <Text fontSize="lg" fontWeight="bold">
            Clients
          </Text>
          <Badge colorScheme="green" fontSize="sm">
            {clientList.length}
          </Badge>
        </HStack>
      </HStack>

      {/* Client List */}
      <VStack
        spacing={0}
        align="stretch"
        overflowY="auto"
        flex={1}
        css={{
          '&::-webkit-scrollbar': {
            width: '8px',
          },
          '&::-webkit-scrollbar-track': {
            background: 'rgba(255, 255, 255, 0.05)',
          },
          '&::-webkit-scrollbar-thumb': {
            background: 'rgba(255, 255, 255, 0.2)',
            borderRadius: '4px',
          },
          '&::-webkit-scrollbar-thumb:hover': {
            background: 'rgba(255, 255, 255, 0.3)',
          },
        }}
      >
        {clientList.length === 0 ? (
          <Box p={8} textAlign="center" color="whiteAlpha.600">
            <Text>No clients online</Text>
          </Box>
        ) : (
          clientList.map((client) => (
            <Tooltip
              key={client.pubkey}
              label={
                <VStack align="start" spacing={1}>
                  <Text fontWeight="bold">{client.npub}</Text>
                  <Text fontSize="xs">{client.clientName} {client.clientVersion}</Text>
                  <Text fontSize="xs">Last seen: {getTimeSince(client.lastUpdateTimestamp)}</Text>
                </VStack>
              }
              placement="left"
              hasArrow
            >
              <Box
                p={3}
                borderBottom="1px solid rgba(255, 255, 255, 0.05)"
                _hover={{
                  bg: 'rgba(255, 255, 255, 0.05)',
                  cursor: 'pointer',
                }}
                transition="background 0.2s"
                position="relative"
                onClick={() => onOpenProfile?.(client.pubkey)}
              >
                <HStack spacing={3} align="start">
                  {/* Avatar */}
                  <Avatar
                    size="sm"
                    name={getDisplayName(client)}
                    src={getProfilePicture(client)}
                  />

                  {/* Client Info */}
                  <VStack align="start" spacing={1} flex={1} minW={0}>
                    <Text
                      fontSize="sm"
                      fontWeight="medium"
                      noOfLines={1}
                      w="full"
                    >
                      {getDisplayName(client)}
                    </Text>

                    {/* Activity Badges */}
                    <Wrap spacing={1}>
                      {client.position && (
                        <Badge
                          fontSize="2xs"
                          px={1.5}
                          py={0.5}
                          borderRadius="md"
                          bg="gray.600"
                          color="white"
                          display="flex"
                          alignItems="center"
                          gap={1}
                        >
                          <FiMapPin size={10} />
                          {formatPosition(client.position)}
                        </Badge>
                      )}
                      {client.voiceConnected && (
                        <Badge
                          fontSize="2xs"
                          px={1.5}
                          py={0.5}
                          borderRadius="md"
                          bg="purple.500"
                          color="white"
                          display="flex"
                          alignItems="center"
                          gap={1}
                        >
                          <FiHeadphones size={10} />
                          listening
                        </Badge>
                      )}
                      {client.micEnabled && (
                        <Badge
                          fontSize="2xs"
                          px={1.5}
                          py={0.5}
                          borderRadius="md"
                          bg="red.500"
                          color="white"
                          display="flex"
                          alignItems="center"
                          gap={1}
                        >
                          <FiMic size={10} />
                          speaking
                        </Badge>
                      )}
                      {client.activities.includes('chatting') && (
                        <Badge
                          fontSize="2xs"
                          px={1.5}
                          py={0.5}
                          borderRadius="md"
                          bg="blue.500"
                          color="white"
                          display="flex"
                          alignItems="center"
                          gap={1}
                        >
                          <FiMessageSquare size={10} />
                          chatting
                        </Badge>
                      )}
                      {client.activities.includes('exploring') && (
                        <Badge
                          fontSize="2xs"
                          px={1.5}
                          py={0.5}
                          borderRadius="md"
                          bg="green.500"
                          color="white"
                          display="flex"
                          alignItems="center"
                          gap={1}
                        >
                          <FiCompass size={10} />
                          exploring
                        </Badge>
                      )}
                      {client.activities.includes('editing') && (
                        <Badge
                          fontSize="2xs"
                          px={1.5}
                          py={0.5}
                          borderRadius="md"
                          bg="orange.500"
                          color="white"
                          display="flex"
                          alignItems="center"
                          gap={1}
                        >
                          <FiEdit3 size={10} />
                          editing
                        </Badge>
                      )}
                    </Wrap>
                  </VStack>
                </HStack>
              </Box>
            </Tooltip>
          ))
        )}
      </VStack>
    </Box>
  )
}
