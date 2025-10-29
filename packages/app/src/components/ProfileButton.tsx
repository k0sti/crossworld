import * as logger from '../utils/logger';
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
import { ExtensionAccount, SimpleAccount, NostrConnectAccount } from 'applesauce-accounts/accounts'
import { ExtensionSigner, NostrConnectSigner } from 'applesauce-signers'
import { useAccountManager } from 'applesauce-react/hooks'
import { Relay } from 'applesauce-relay'
import { DEFAULT_RELAYS, DEFAULT_RELAY_STATES } from '../config'
import { NostrExtensionInfoModal } from './NostrExtensionInfoModal'
import { LoginSettingsService } from '../services/login-settings'

// Check if we're on Android
const IS_WEB_ANDROID = /android/i.test(navigator.userAgent)

// Get world relays from configuration
function getWorldRelays(): string[] {
  try {
    const savedRelays = localStorage.getItem('crossworld_relays')
    if (savedRelays) {
      const relays = JSON.parse(savedRelays)
      return relays.filter((r: any) => r.enabledForWorld).map((r: any) => r.url)
    }
  } catch (error) {
    logger.error('ui', '[ProfileButton] Failed to load relay config:', error)
  }

  // Fallback to default world relays
  return DEFAULT_RELAYS.filter(url => {
    const state = DEFAULT_RELAY_STATES[url as keyof typeof DEFAULT_RELAY_STATES]
    return state?.enabledForWorld
  })
}

interface ProfileMetadata {
  name?: string
  picture?: string
  display_name?: string
}

interface ProfileButtonProps {
  pubkey: string | null
  onLogin: (pubkey: string) => void
  onOpenProfile?: () => void
}

export function ProfileButton({ pubkey, onLogin, onOpenProfile }: ProfileButtonProps) {
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
            logger.error('ui', 'Failed to parse profile metadata:', e)
          }
        }
      } catch (error) {
        logger.error('ui', `Failed to fetch profile from ${relayUrl}:`, error)
      }
    }
  }

  const handleGuestLogin = async (name: string) => {
    setIsLoading(true)
    try {
      const account = SimpleAccount.generateNew()
      manager.addAccount(account)
      manager.setActive(account)

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

      // Publish guest account profile metadata only to world relays
      // (Never publish to profile relays - they are read-only for querying existing profiles)
      const worldRelays = getWorldRelays()
      logger.log('ui', '[ProfileButton] Publishing guest profile to world relays:', worldRelays)

      for (const relayUrl of worldRelays) {
        try {
          const relay = new Relay(relayUrl)
          relay.publish(signedEvent)
          setTimeout(() => {
            try { relay.close() } catch (e) {}
          }, 1000)
        } catch (error) {
          logger.error('ui', `Failed to publish to ${relayUrl}:`, error)
        }
      }

      // Save guest account data (persistent)
      try {
        const serializedAccount = account.toJSON()
        LoginSettingsService.saveGuestAccount({
          account: serializedAccount,
          name,
        })
      } catch (error) {
        logger.error('ui', 'Failed to save guest account:', error)
      }

      // Save login settings
      LoginSettingsService.save({
        method: 'guest',
        pubkey: account.pubkey,
        lastLogin: Date.now(),
      })

      toast({
        title: 'Guest login successful',
        description: `Welcome, ${name}!`,
        status: 'success',
        duration: 3000,
        isClosable: true,
      })

      onLogin(account.pubkey)
    } catch (error) {
      logger.error('ui', 'Guest login error:', error)
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

  const tryAmberLogin = async () => {
    try {
      // Create NostrConnect signer for Amber
      const signer = new NostrConnectSigner({
        relays: ['wss://relay.nsec.app', 'wss://relay.damus.io']
      })

      // Generate connection URI for Amber
      const connectionURI = signer.getNostrConnectURI({
        name: 'Crossworld',
        url: window.location.origin,
        image: new URL('/favicon.ico', window.location.origin).toString(),
      })

      // Try to open Amber app directly
      window.location.href = connectionURI

      // Wait a moment for Amber to connect
      toast({
        title: 'Opening Amber',
        description: 'Waiting for Amber connection...',
        status: 'info',
        duration: 3000,
        isClosable: true,
      })

      // Start listening for connection (with timeout)
      const timeout_ms = 30000
      const connectionPromise = signer.waitForSigner()
      const timeoutPromise = new Promise((_, reject) =>
        setTimeout(() => reject(new Error('Amber connection timeout')), timeout_ms)
      )

      await Promise.race([connectionPromise, timeoutPromise])

      const publicKey = await signer.getPublicKey()

      const existingAccount = manager.accounts.find(
        (a) => a.type === NostrConnectAccount.type && a.pubkey === publicKey
      )

      if (!existingAccount) {
        const account = new NostrConnectAccount(publicKey, signer)
        account.metadata = { _isAmber: true }
        manager.addAccount(account)
        manager.setActive(account)
      } else {
        manager.setActive(existingAccount)
      }

      // Save login settings
      LoginSettingsService.save({
        method: 'amber',
        pubkey: publicKey,
        lastLogin: Date.now(),
      })

      toast({
        title: 'Amber connected',
        description: 'Successfully connected to Amber',
        status: 'success',
        duration: 3000,
        isClosable: true,
      })

      onLogin(publicKey)
      return true
    } catch (error) {
      logger.error('ui', 'Amber connection error:', error)
      // Clear any stale login settings
      LoginSettingsService.clear()
      return false
    }
  }

  const handleLoginButtonClick = () => {
    // Always show the login modal when user clicks login button
    // This allows user to choose login method
    onOpen()
  }

  const handleExtensionLogin = async () => {
    setIsLoading(true)
    try {
      if (!window.nostr) {
        // If on Android, try Amber first
        if (IS_WEB_ANDROID) {
          const amberSuccess = await tryAmberLogin()
          if (amberSuccess) {
            setIsLoading(false)
            return
          }
        }

        // If not Android or Amber failed, show modal
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
        manager.setActive(account)
      } else {
        manager.setActive(existingAccount)
      }

      // Save login settings
      LoginSettingsService.save({
        method: 'extension',
        pubkey: publicKey,
        lastLogin: Date.now(),
      })

      toast({
        title: 'Connected',
        description: 'Successfully connected to extension',
        status: 'success',
        duration: 3000,
        isClosable: true,
      })

      onLogin(publicKey)
    } catch (error) {
      logger.error('ui', 'Extension login error:', error)

      // Clear any stale login settings that might be causing issues
      LoginSettingsService.clear()

      // Provide user-friendly error message
      const errorMessage = error instanceof Error ? error.message : 'Failed to connect to extension'
      const isExtensionError = errorMessage.includes('bounds') || errorMessage.includes('extension')

      toast({
        title: 'Connection failed',
        description: isExtensionError
          ? 'Extension connection failed. Please try again or use a different login method.'
          : errorMessage,
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
          onClick={handleLoginButtonClick}
          isLoading={isLoading}
        />
        <NostrExtensionInfoModal
          isOpen={isOpen}
          onClose={onClose}
          onGuestLogin={handleGuestLogin}
          onExtensionLogin={handleExtensionLogin}
          onLogin={onLogin}
        />
      </>
    )
  }

  const displayName = profile?.display_name || profile?.name || null

  return (
    <HStack
      gap={2}
      cursor={onOpenProfile ? "pointer" : "default"}
      onClick={onOpenProfile}
      _hover={onOpenProfile ? { opacity: 0.8 } : undefined}
      transition="opacity 0.2s"
    >
      <Avatar src={profile?.picture} icon={<FiUser />} name={displayName || pubkey} size="sm" />
      {displayName && (
        <Text fontSize="sm" fontWeight="medium" color="white" lineHeight="1">
          {displayName}
        </Text>
      )}
    </HStack>
  )
}
