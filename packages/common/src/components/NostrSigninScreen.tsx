import { useState, useEffect } from 'react'
import {
  VStack,
  Text,
  Link,
  Box,
  Input,
  Button,
  HStack,
  Badge,
  useToast,
} from '@chakra-ui/react'
import { SimpleAccount } from 'applesauce-accounts/accounts'
import { useAccountManager } from 'applesauce-react/hooks'
import { LoginSettingsService } from '../services/login-settings'
import { Screen } from './Screen'

// Check if we're on Android
const IS_WEB_ANDROID = /android/i.test(navigator.userAgent)

interface NostrSigninScreenProps {
  isOpen: boolean
  onClose: () => void
  onGuestLogin: (name: string) => void
  onExtensionLogin: () => void
  onLogin: (pubkey: string) => void
}

export function NostrSigninScreen({ isOpen, onClose, onGuestLogin, onExtensionLogin, onLogin }: NostrSigninScreenProps) {
  const [guestName, setGuestName] = useState('')
  const [isLoading, setIsLoading] = useState(false)
  const [savedGuest, setSavedGuest] = useState<{ account: any; name: string } | null>(null)
  const manager = useAccountManager()
  const toast = useToast()

  useEffect(() => {
    // Check for saved guest account
    try {
      const guestData = LoginSettingsService.loadGuestAccount()
      if (guestData) {
        setSavedGuest(guestData)
      }
    } catch (error) {
      console.error('Failed to load guest account:', error)
    }
  }, [])

  const handleGuestLogin = async () => {
    if (!guestName.trim()) return

    setIsLoading(true)
    try {
      await onGuestLogin(guestName.trim())
      onClose()
    } finally {
      setIsLoading(false)
      setGuestName('')
    }
  }

  const handleQuickLogin = async () => {
    if (!savedGuest) return

    setIsLoading(true)
    try {
      const account = SimpleAccount.fromJSON(savedGuest.account)

      const existingAccount = manager.accounts.find(
        (a) => a.type === SimpleAccount.type && a.pubkey === account.pubkey
      )

      if (!existingAccount) {
        manager.addAccount(account)
        manager.setActive(account)
      } else {
        manager.setActive(existingAccount)
      }

      // Save login settings
      LoginSettingsService.save({
        method: 'guest',
        pubkey: account.pubkey,
        lastLogin: Date.now(),
      })

      toast({
        title: 'Guest login successful',
        description: `Welcome back, ${savedGuest.name}!`,
        status: 'success',
        duration: 3000,
        isClosable: true,
      })

      onLogin(account.pubkey)
      onClose()
    } catch (error) {
      console.error('Quick login error:', error)
      toast({
        title: 'Login failed',
        description: error instanceof Error ? error.message : 'Failed to restore guest account',
        status: 'error',
        duration: 5000,
        isClosable: true,
      })
    } finally {
      setIsLoading(false)
    }
  }

  return (
    <Screen
      isOpen={isOpen}
      onClose={onClose}
      title="Login"
      actions={
        <Button
          size="sm"
          variant="ghost"
          onClick={onClose}
          color="whiteAlpha.700"
          _hover={{ color: 'white' }}
        >
          Close
        </Button>
      }
    >
      <VStack spacing={4} align="stretch" maxW="800px" mx="auto">
        <Box>
          <Button
            onClick={() => {
              onExtensionLogin()
              onClose()
            }}
            colorScheme="purple"
            width="100%"
          >
            Login with Extension
          </Button>
        </Box>

        {savedGuest && (
          <Box>
            <Button
              onClick={handleQuickLogin}
              isLoading={isLoading}
              colorScheme="green"
              width="100%"
              size="lg"
            >
              Guest login: {savedGuest.name}
            </Button>
          </Box>
        )}

        <Box>
          <Text fontSize="md" fontWeight="semibold" mb={2} color="white">
            Login as guest
          </Text>
          <HStack>
            <Input
              placeholder="Enter your name"
              value={guestName}
              onChange={(e) => setGuestName(e.target.value)}
              onKeyPress={(e) => e.key === 'Enter' && handleGuestLogin()}
              bg="rgba(255, 255, 255, 0.05)"
              border="1px solid rgba(255, 255, 255, 0.1)"
              color="white"
              _placeholder={{ color: 'whiteAlpha.500' }}
            />
            <Button
              onClick={handleGuestLogin}
              isLoading={isLoading}
              isDisabled={!guestName.trim()}
              colorScheme="blue"
            >
              Login
            </Button>
          </HStack>
        </Box>

        <Box>
          <Text fontSize="md" fontWeight="semibold" mb={2} color="white">
            What is Nostr
          </Text>
          <Text fontSize="md" mb={2} color="whiteAlpha.800">
            Nostr is a simple, open protocol for decentralized social media.
          </Text>
          <Link
            href="https://start.nostr.net/"
            isExternal
            color="blue.300"
            fontWeight="medium"
            _hover={{ textDecoration: 'underline' }}
          >
            start.nostr.net
          </Link>
        </Box>

        {IS_WEB_ANDROID && (
          <Box>
            <HStack mb={2}>
              <Text fontSize="md" fontWeight="semibold" color="white">
                Amber Signer
              </Text>
              <Badge colorScheme="yellow">Android</Badge>
            </HStack>
            <Text fontSize="md" mb={2} color="whiteAlpha.800">
              Amber is a secure key management app for Android. If you have Amber installed,
              the app should have opened automatically.
            </Text>
            <Link
              href="https://github.com/greenart7c3/Amber"
              isExternal
              color="blue.300"
              fontWeight="medium"
              _hover={{ textDecoration: 'underline' }}
            >
              Get Amber from GitHub
            </Link>
          </Box>
        )}

      </VStack>
    </Screen>
  )
}
