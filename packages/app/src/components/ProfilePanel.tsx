import { useState, useEffect } from 'react'
import { Box, VStack, Text, Avatar, HStack, Divider, Flex } from '@chakra-ui/react'
import { FiUser } from 'react-icons/fi'
import { npubEncode } from 'nostr-tools/nip19'
import { Relay } from 'applesauce-relay'
import { DEFAULT_RELAYS } from '../config'

interface ProfileMetadata {
  name?: string
  picture?: string
  display_name?: string
  about?: string
}

interface ProfilePanelProps {
  pubkey: string | null
  onClose: () => void
}

export function ProfilePanel({ pubkey, onClose }: ProfilePanelProps) {
  const [profile, setProfile] = useState<ProfileMetadata | null>(null)
  const npub = pubkey ? npubEncode(pubkey) : ''
  const displayNpub = npub ? `${npub.slice(0, 12)}...${npub.slice(-8)}` : ''

  useEffect(() => {
    if (pubkey) {
      fetchProfile(pubkey)
    } else {
      setProfile(null)
    }
  }, [pubkey])

  const fetchProfile = async (pubkey: string) => {
    for (const relayUrl of DEFAULT_RELAYS) {
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

            <VStack align="stretch" gap={2}>
              <HStack>
                <Text fontSize="sm" color="whiteAlpha.600" minW="100px">Public Key:</Text>
                <Text fontSize="sm" color="white" fontFamily="monospace">{displayNpub}</Text>
              </HStack>

              <Divider borderColor="whiteAlpha.200" />

              {profile?.about && (
                <>
                  <VStack align="stretch" gap={1}>
                    <Text fontSize="sm" color="whiteAlpha.600">About:</Text>
                    <Text fontSize="sm" color="white">{profile.about}</Text>
                  </VStack>
                  <Divider borderColor="whiteAlpha.200" />
                </>
              )}

              <VStack align="stretch" gap={1}>
                <Text fontSize="sm" color="whiteAlpha.600">Avatar Model:</Text>
                <Text fontSize="sm" color="white">Voxel-based procedural avatar</Text>
              </VStack>
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
