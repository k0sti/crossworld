import { useState, useEffect, useRef } from 'react'
import { VStack, Text, Avatar, HStack, Divider, Flex, IconButton, Tooltip, useToast, SimpleGrid, Button, AlertDialog, AlertDialogBody, AlertDialogFooter, AlertDialogHeader, AlertDialogContent, AlertDialogOverlay, useDisclosure } from '@chakra-ui/react'
import { FiUser, FiCopy, FiExternalLink, FiRefreshCw, FiLogOut } from 'react-icons/fi'
import { npubEncode } from 'nostr-tools/nip19'
import { Relay } from 'applesauce-relay'
import { pubkey_to_emoji } from '@workspace/wasm'
import { DEFAULT_RELAYS } from '../config'
import { ResponsivePanel } from './ResponsivePanel'

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

const SHOW_EMOJI_HASH = false

interface ProfilePanelProps {
  pubkey: string | null
  isOpen: boolean
  onClose: () => void
  local_user?: boolean
  onLogout?: () => void
  onOpenAvatarSelection?: () => void
  onRestart?: () => void
}

export function ProfilePanel({ pubkey, isOpen, onClose, local_user = false, onLogout, onOpenAvatarSelection, onRestart }: ProfilePanelProps) {
  const [profile, setProfile] = useState<ProfileMetadata | null>(null)
  const [enabledRelays, setEnabledRelays] = useState<string[]>([])
  const toast = useToast()
  const cancelRef = useRef<HTMLButtonElement>(null)
  const { isOpen: isLogoutOpen, onOpen: onLogoutOpen, onClose: onLogoutClose } = useDisclosure()
  const { isOpen: isRestartOpen, onOpen: onRestartOpen, onClose: onRestartClose } = useDisclosure()
  const npub = pubkey ? npubEncode(pubkey) : ''
  const displayNpub = npub ? `${npub.slice(0, 12)}...${npub.slice(-8)}` : ''
  const emojiHash = pubkey ? pubkey_to_emoji(pubkey) : ''
  const emojiArray = Array.from(emojiHash)

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

  const handleLogoutConfirm = () => {
    onLogoutClose()
    if (onLogout) {
      onLogout()
    }
    onClose()
  }

  const handleRestartConfirm = () => {
    onRestartClose()

    // Clear avatar settings from localStorage
    localStorage.removeItem('avatarSelection')

    toast({
      title: 'Avatar settings cleared',
      description: 'Opening avatar selection...',
      status: 'info',
      duration: 2000,
      isClosable: true,
    })

    // Close profile panel
    onClose()

    // Call restart handler to reset avatar config
    if (onRestart) {
      onRestart()
    }

    // Open avatar selection
    if (onOpenAvatarSelection) {
      onOpenAvatarSelection()
    }
  }

  return (
    <>
      <ResponsivePanel
        isOpen={isOpen}
        onClose={onClose}
        title="Profile"
        forceFullscreen={true}
        closeOnClickOutside={!isLogoutOpen && !isRestartOpen}
        actions={
          <HStack spacing={3}>
            {local_user && (
              <>
                <Button
                  leftIcon={<FiRefreshCw />}
                  onClick={onRestartOpen}
                  size="sm"
                  colorScheme="blue"
                  variant="outline"
                >
                  Restart
                </Button>
                <Button
                  leftIcon={<FiLogOut />}
                  onClick={onLogoutOpen}
                  size="sm"
                  colorScheme="red"
                  variant="outline"
                >
                  Logout
                </Button>
              </>
            )}
            <Button
              onClick={onClose}
              size="sm"
              colorScheme="blue"
            >
              Close
            </Button>
          </HStack>
        }
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

              <HStack justify="center">
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

              {SHOW_EMOJI_HASH && (
                <Flex justify="center" py={2}>
                  <SimpleGrid columns={9} gap={1}>
                    {emojiArray.map((emoji, index) => (
                      <Text key={index} fontSize="2xl" lineHeight="1">
                        {String(emoji)}
                      </Text>
                    ))}
                  </SimpleGrid>
                </Flex>
              )}

              {profile?.about && (
                <>
                  <Divider borderColor="whiteAlpha.200" />
                  <VStack align="stretch" gap={1}>
                    <Text fontSize="sm" color="whiteAlpha.600">About:</Text>
                    <Text fontSize="sm" color="white">{profile.about}</Text>
                  </VStack>
                </>
              )}
            </>
          ) : (
            <Text fontSize="sm" color="whiteAlpha.700" textAlign="center" py={8}>
              Please log in to view your profile
            </Text>
          )}
        </VStack>
      </ResponsivePanel>

      {/* Logout Confirmation Dialog */}
      {isLogoutOpen && (
        <AlertDialog
          isOpen={isLogoutOpen}
          leastDestructiveRef={cancelRef}
          onClose={onLogoutClose}
        >
          <AlertDialogOverlay zIndex={2000}>
            <AlertDialogContent bg="gray.800" color="white">
              <AlertDialogHeader fontSize="lg" fontWeight="bold">
                Logout
              </AlertDialogHeader>

              <AlertDialogBody>
                Are you sure you want to logout? You will need to log in again to access your account.
              </AlertDialogBody>

              <AlertDialogFooter>
                <Button ref={cancelRef} onClick={onLogoutClose}>
                  Cancel
                </Button>
                <Button colorScheme="red" onClick={handleLogoutConfirm} ml={3}>
                  Logout
                </Button>
              </AlertDialogFooter>
            </AlertDialogContent>
          </AlertDialogOverlay>
        </AlertDialog>
      )}

      {/* Restart Confirmation Dialog */}
      {isRestartOpen && (
        <AlertDialog
          isOpen={isRestartOpen}
          leastDestructiveRef={cancelRef}
          onClose={onRestartClose}
        >
          <AlertDialogOverlay zIndex={2000}>
            <AlertDialogContent bg="gray.800" color="white">
              <AlertDialogHeader fontSize="lg" fontWeight="bold">
                Restart Avatar Selection
              </AlertDialogHeader>

              <AlertDialogBody>
                Are you sure you want to restart? This will clear your current avatar settings.
              </AlertDialogBody>

              <AlertDialogFooter>
                <Button ref={cancelRef} onClick={onRestartClose}>
                  Cancel
                </Button>
                <Button colorScheme="blue" onClick={handleRestartConfirm} ml={3}>
                  Restart
                </Button>
              </AlertDialogFooter>
            </AlertDialogContent>
          </AlertDialogOverlay>
        </AlertDialog>
      )}
    </>
  )
}
