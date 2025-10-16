import { useState, useEffect } from 'react'
import {
  HStack,
  Avatar,
  Text,
  IconButton,
  useToast,
  useDisclosure,
} from '@chakra-ui/react'
import { FiLogIn, FiUser } from 'react-icons/fi'
import { ExtensionAccount, SimpleAccount } from 'applesauce-accounts/accounts'
import { ExtensionSigner } from 'applesauce-signers'
import { useAccountManager } from 'applesauce-react/hooks'
import { Relay } from 'applesauce-relay'
import { DEFAULT_RELAYS } from '../config'
import { NostrExtensionInfoModal } from './NostrExtensionInfoModal'

interface ProfileMetadata {
  name?: string
  picture?: string
  display_name?: string
}

interface ProfileButtonProps {
  pubkey: string | null
  onLogin: (pubkey: string) => void
}

export function ProfileButton({ pubkey, onLogin }: ProfileButtonProps) {
  const [isLoading, setIsLoading] = useState(false)
  const [profile, setProfile] = useState<ProfileMetadata | null>(null)
  const toast = useToast()
  const manager = useAccountManager()
  const { isOpen, onOpen, onClose } = useDisclosure()

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
            kinds: [0], // Profile metadata
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

  const handleGuestLogin = async (name: string) => {
    setIsLoading(true)
    try {
      const account = SimpleAccount.generateNew()
      manager.addAccount(account)

      const metadata: ProfileMetadata = {
        name,
        display_name: name,
      }

      const metadataEvent = {
        kind: 0,
        tags: [],
        content: JSON.stringify(metadata),
        created_at: Math.floor(Date.now() / 1000),
      }

      const signedEvent = await account.signer.signEvent(metadataEvent)

      for (const relayUrl of DEFAULT_RELAYS) {
        try {
          const relay = new Relay(relayUrl)
          relay.publish(signedEvent)
          setTimeout(() => {
            try { relay.close() } catch (e) {}
          }, 1000)
        } catch (error) {
          console.error(`Failed to publish to ${relayUrl}:`, error)
        }
      }

      toast({
        title: 'Guest login successful',
        description: `Welcome, ${name}!`,
        status: 'success',
        duration: 3000,
        isClosable: true,
      })

      onLogin(account.pubkey)
    } catch (error) {
      console.error('Guest login error:', error)
      toast({
        title: 'Login failed',
        description: error instanceof Error ? error.message : 'Failed to create guest account',
        status: 'error',
        duration: 5000,
        isClosable: true,
      })
    } finally {
      setIsLoading(false)
    }
  }

  const handleExtensionLogin = async () => {
    setIsLoading(true)
    try {
      if (!window.nostr) {
        setIsLoading(false)
        onOpen()
        return
      }

      const signer = new ExtensionSigner()
      const publicKey = await signer.getPublicKey()

      const existingAccount = manager.accounts.find(
        (a) => a.type === ExtensionAccount.type && a.pubkey === publicKey
      )

      if (!existingAccount) {
        const account = new ExtensionAccount(publicKey, signer)
        manager.addAccount(account)
      }

      toast({
        title: 'Connected',
        description: 'Successfully connected to extension',
        status: 'success',
        duration: 3000,
        isClosable: true,
      })

      onLogin(publicKey)
    } catch (error) {
      console.error('Extension login error:', error)
      toast({
        title: 'Connection failed',
        description: error instanceof Error ? error.message : 'Failed to connect to extension',
        status: 'error',
        duration: 5000,
        isClosable: true,
      })
    } finally {
      setIsLoading(false)
    }
  }

  if (!pubkey) {
    return (
      <>
        <IconButton
          aria-label="Login"
          icon={<FiLogIn />}
          onClick={handleExtensionLogin}
          isLoading={isLoading}
        />
        <NostrExtensionInfoModal
          isOpen={isOpen}
          onClose={onClose}
          onGuestLogin={handleGuestLogin}
        />
      </>
    )
  }

  const displayName = profile?.display_name || profile?.name || null

  return (
    <HStack gap={2}>
      <Avatar src={profile?.picture} icon={<FiUser />} name={displayName || pubkey} size="sm" />
      {displayName && (
        <Text fontSize="sm" fontWeight="medium" color="white" lineHeight="1">
          {displayName}
        </Text>
      )}
    </HStack>
  )
}
