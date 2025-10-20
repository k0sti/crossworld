import { useState, useEffect, useRef } from 'react'
import { Box, VStack, Text, Avatar, HStack, Divider, Flex, IconButton, Tooltip, useToast, SimpleGrid } from '@chakra-ui/react'
import { FiUser, FiCopy, FiExternalLink } from 'react-icons/fi'
import { npubEncode } from 'nostr-tools/nip19'
import { Relay } from 'applesauce-relay'
import { pubkey_to_emoji } from '@workspace/wasm'
import { DEFAULT_RELAYS } from '../config'

interface ProfileMetadata {
  name?: string
  picture?: string
  display_name?: string
  about?: string
}

interface RelayConfig {
  url: string
  enabledForProfile: boolean
  enabledForWorld: boolean
  status: 'connected' | 'connecting' | 'error' | 'disconnected'
}

interface ProfilePanelProps {
  pubkey: string | null
  onClose?: () => void
}

export function ProfilePanel({ pubkey, onClose }: ProfilePanelProps) {
  const [profile, setProfile] = useState<ProfileMetadata | null>(null)
  const [enabledRelays, setEnabledRelays] = useState<string[]>([])
  const toast = useToast()
  const panelRef = useRef<HTMLDivElement>(null)
  const npub = pubkey ? npubEncode(pubkey) : ''
  const displayNpub = npub ? `${npub.slice(0, 12)}...${npub.slice(-8)}` : ''
  const emojiHash = pubkey ? pubkey_to_emoji(pubkey) : ''
  const emojiArray = Array.from(emojiHash)

  // Handle click outside to close panel
  useEffect(() => {
    if (!onClose) return

    const handleClickOutside = (event: MouseEvent) => {
      if (panelRef.current && !panelRef.current.contains(event.target as Node)) {
        onClose()
      }
    }

    // Add listener after a small delay to prevent immediate closing
    const timeoutId = setTimeout(() => {
      document.addEventListener('mousedown', handleClickOutside)
    }, 100)

    return () => {
      clearTimeout(timeoutId)
      document.removeEventListener('mousedown', handleClickOutside)
    }
  }, [onClose])

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
    }

    window.addEventListener('relayConfigChanged', handleRelayConfigChanged)

    return () => {
      window.removeEventListener('relayConfigChanged', handleRelayConfigChanged)
    }
  }, [])

  const copyNpub = () => {
    navigator.clipboard.writeText(npub)
    toast({
      title: 'npub copied',
      status: 'success',
      duration: 2000,
      isClosable: true,
    })
  }

  const openExternal = () => {
    window.open(`https://nostr.eu/${npub}`, '_blank', 'noopener,noreferrer')
  }

  useEffect(() => {
    if (pubkey && enabledRelays.length > 0) {
      fetchProfile(pubkey)
    } else {
      setProfile(null)
    }
  }, [pubkey, enabledRelays])

  const fetchProfile = async (pubkey: string) => {
    for (const relayUrl of enabledRelays) {
      try {
        const relay = new Relay(relayUrl)
        const events = await new Promise<any[]>((resolve) => {
          const collectedEvents: any[] = []
          let isResolved = false

          const cleanup = () => {
            if (!isResolved) {
              isResolved = true
              try { relay.close() } catch (e) {}
              resolve(collectedEvents)
            }
          }

          relay.request({
            kinds: [0],
            authors: [pubkey],
            limit: 1
          }).subscribe({
            next: (event: any) => {
              if (event === 'EOSE') {
                cleanup()
              } else if (event && event.kind === 0) {
                collectedEvents.push(event)
              }
            },
            error: () => cleanup(),
            complete: () => cleanup()
          })

          setTimeout(cleanup, 3000)
        })

        if (events.length > 0) {
          const latestEvent = events.sort((a, b) => b.created_at - a.created_at)[0]
          try {
            const metadata = JSON.parse(latestEvent.content)
            setProfile(metadata)
            return
          } catch (e) {
            console.error('Failed to parse profile metadata:', e)
          }
        }
      } catch (error) {
        console.error(`Failed to fetch profile from ${relayUrl}:`, error)
      }
    }
  }

  const displayName = profile?.display_name || profile?.name || 'Anonymous'

  return (
    <Box
      ref={panelRef}
      position="fixed"
      top="60px"
      left="68px"
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

      <VStack align="stretch" gap={4}>
        {pubkey ? (
          <>
            <Flex justify="center" py={4}>
              <Avatar
                size="2xl"
                src={profile?.picture}
                icon={<FiUser />}
                name={displayName}
              />
            </Flex>

            <Text fontSize="xl" color="white" fontWeight="semibold" textAlign="center">
              {displayName}
            </Text>

            <Flex justify="center" py={2}>
              <SimpleGrid columns={9} gap={1}>
                {emojiArray.map((emoji, index) => (
                  <Text key={index} fontSize="2xl" lineHeight="1">
                    {String(emoji)}
                  </Text>
                ))}
              </SimpleGrid>
            </Flex>

            <VStack align="stretch" gap={2}>
              <HStack justify="space-between">
                <Text fontSize="sm" color="whiteAlpha.600">Public Key:</Text>
                <HStack>
                  <Text fontSize="xs" color="white" fontFamily="monospace">{displayNpub}</Text>
                  <Tooltip label="Copy npub">
                    <IconButton
                      aria-label="Copy npub"
                      icon={<FiCopy />}
                      size="xs"
                      variant="ghost"
                      onClick={copyNpub}
                      color="whiteAlpha.700"
                      _hover={{ color: 'white' }}
                    />
                  </Tooltip>
                  <Tooltip label="Open on nostr.eu">
                    <IconButton
                      aria-label="Open profile"
                      icon={<FiExternalLink />}
                      size="xs"
                      variant="ghost"
                      onClick={openExternal}
                      color="whiteAlpha.700"
                      _hover={{ color: 'white' }}
                    />
                  </Tooltip>
                </HStack>
              </HStack>

              {profile?.about && (
                <>
                  <Divider borderColor="whiteAlpha.200" />
                  <VStack align="stretch" gap={1}>
                    <Text fontSize="sm" color="whiteAlpha.600">About:</Text>
                    <Text fontSize="sm" color="white">{profile.about}</Text>
                  </VStack>
                </>
              )}
            </VStack>
          </>
        ) : (
          <Text fontSize="sm" color="whiteAlpha.700" textAlign="center" py={8}>
            Please log in to view your profile
          </Text>
        )}
      </VStack>
    </Box>
  )
}
