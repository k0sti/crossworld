import { Box, HStack, Avatar, Popover, PopoverTrigger, PopoverContent, PopoverBody, VStack, Text, IconButton, Badge, Wrap, useToast } from '@chakra-ui/react'
import { useState, useEffect, useCallback } from 'react'
import { FiMapPin, FiMessageSquare, FiCompass, FiEdit3, FiMic, FiHeadphones, FiCopy, FiExternalLink } from 'react-icons/fi'
import { type AvatarStateService, type AvatarState } from '../services/avatar-state'
import { profileCache } from '../services/profile-cache'
import { DEFAULT_RELAYS } from '../config'

interface ClientListPanelProps {
  isOpen: boolean
  statusService: AvatarStateService
  onOpenProfile?: (pubkey: string) => void
  isEditMode?: boolean
}

interface RelayConfig {
  url: string
  enabledForProfile: boolean
  enabledForWorld: boolean
  status: 'connected' | 'connecting' | 'error' | 'disconnected'
}

export function ClientListPanel({ isEditMode = false, statusService }: ClientListPanelProps) {
  const [clients, setClients] = useState<Map<string, AvatarState>>(new Map())
  const [enabledRelays, setEnabledRelays] = useState<string[]>([])
  const toast = useToast()

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
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [statusService, enabledRelays])

  const clientList = Array.from(clients.values()).sort((a, b) => {
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

  const copyToClipboard = (text: string, label: string) => {
    navigator.clipboard.writeText(text)
    toast({
      title: `${label} copied`,
      status: 'success',
      duration: 2000,
      isClosable: true,
    })
  }

  const openNpubLink = (npub: string) => {
    window.open(`https://njump.me/${npub}`, '_blank')
  }

  const getActivityBadges = (client: AvatarState) => {
    const badges: Array<{ label: string; icon: React.ReactNode; color: string }> = []

    if (client.voiceConnected) {
      badges.push({ label: 'listening', icon: <FiHeadphones size={10} />, color: 'purple.500' })
    }
    if (client.micEnabled) {
      badges.push({ label: 'speaking', icon: <FiMic size={10} />, color: 'red.500' })
    }
    if (client.activities.includes('chatting')) {
      badges.push({ label: 'chatting', icon: <FiMessageSquare size={10} />, color: 'blue.500' })
    }
    if (client.activities.includes('exploring')) {
      badges.push({ label: 'exploring', icon: <FiCompass size={10} />, color: 'green.500' })
    }
    if (client.activities.includes('editing')) {
      badges.push({ label: 'editing', icon: <FiEdit3 size={10} />, color: 'orange.500' })
    }
    if (client.position) {
      const pos = `(${Math.round(client.position.x)}, ${Math.round(client.position.y)}, ${Math.round(client.position.z)})`
      badges.push({ label: pos, icon: <FiMapPin size={10} />, color: 'gray.600' })
    }

    return badges
  }

  // Hide in edit mode
  if (isEditMode) return null

  return (
    <Box
      position="fixed"
      right="20px"
      top="80px"
      width="auto"
      bg="transparent"
      zIndex={1000}
    >
      {/* Client Icons - Vertical Stack */}
      <VStack spacing={2} align="end">
        {clientList.map((client) => (
          <Popover key={client.pubkey} placement="left" trigger="hover">
            <PopoverTrigger>
              <Box>
                <Avatar
                  size="sm"
                  name={getDisplayName(client)}
                  src={getProfilePicture(client)}
                  cursor="pointer"
                  bg="rgba(0, 0, 0, 0.3)"
                  border="2px solid rgba(255, 255, 255, 0.2)"
                  _hover={{
                    border: '2px solid rgba(255, 255, 255, 0.5)',
                    transform: 'scale(1.1)',
                  }}
                  transition="all 0.2s"
                />
              </Box>
            </PopoverTrigger>
            <PopoverContent
              bg="rgba(0, 0, 0, 0.95)"
              backdropFilter="blur(10px)"
              border="1px solid rgba(255, 255, 255, 0.1)"
              boxShadow="0 8px 32px rgba(0, 0, 0, 0.4)"
              color="white"
              width="300px"
            >
              <PopoverBody p={4}>
                <VStack align="start" spacing={3}>
                  {/* Name */}
                  <Text fontSize="md" fontWeight="bold">
                    {getDisplayName(client)}
                  </Text>

                  {/* Short pubkey with copy and link buttons */}
                  <HStack spacing={2} w="full">
                    <Text
                      fontSize="xs"
                      fontFamily="mono"
                      color="whiteAlpha.700"
                      flex={1}
                      noOfLines={1}
                    >
                      {client.npub.slice(0, 16)}...
                    </Text>
                    <IconButton
                      aria-label="Copy npub"
                      icon={<FiCopy />}
                      size="xs"
                      variant="ghost"
                      colorScheme="whiteAlpha"
                      onClick={() => copyToClipboard(client.npub, 'Npub')}
                    />
                    <IconButton
                      aria-label="Open profile"
                      icon={<FiExternalLink />}
                      size="xs"
                      variant="ghost"
                      colorScheme="whiteAlpha"
                      onClick={() => openNpubLink(client.npub)}
                    />
                  </HStack>

                  {/* Activity badges */}
                  <Wrap spacing={1} w="full">
                    {getActivityBadges(client).map((badge, idx) => (
                      <Badge
                        key={idx}
                        fontSize="2xs"
                        px={1.5}
                        py={0.5}
                        borderRadius="md"
                        bg={badge.color}
                        color="white"
                        display="flex"
                        alignItems="center"
                        gap={1}
                      >
                        {badge.icon}
                        {badge.label}
                      </Badge>
                    ))}
                  </Wrap>
                </VStack>
              </PopoverBody>
            </PopoverContent>
          </Popover>
        ))}
      </VStack>
    </Box>
  )
}
