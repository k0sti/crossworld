import { useState } from 'react'
import {
  Modal,
  ModalOverlay,
  ModalContent,
  ModalHeader,
  ModalBody,
  ModalCloseButton,
  VStack,
  Text,
  Link,
  Box,
  Input,
  Button,
  HStack,
  Badge,
} from '@chakra-ui/react'

// Check if we're on Android
const IS_WEB_ANDROID = /android/i.test(navigator.userAgent)

interface NostrExtensionInfoModalProps {
  isOpen: boolean
  onClose: () => void
  onGuestLogin: (name: string) => void
}

export function NostrExtensionInfoModal({ isOpen, onClose, onGuestLogin }: NostrExtensionInfoModalProps) {
  const [guestName, setGuestName] = useState('')
  const [isLoading, setIsLoading] = useState(false)

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

  return (
    <Modal isOpen={isOpen} onClose={onClose} isCentered>
      <ModalOverlay />
      <ModalContent>
        <ModalHeader>Login</ModalHeader>
        <ModalCloseButton />
        <ModalBody pb={6}>
          <VStack spacing={4} align="stretch">
            <Box>
              <Text fontSize="md" fontWeight="semibold" mb={2}>
                Login as guest
              </Text>
              <HStack>
                <Input
                  placeholder="Enter your name"
                  value={guestName}
                  onChange={(e) => setGuestName(e.target.value)}
                  onKeyPress={(e) => e.key === 'Enter' && handleGuestLogin()}
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
              <Text fontSize="md" fontWeight="semibold" mb={2}>
                What is Nostr
              </Text>
              <Text fontSize="md" mb={2}>
                Nostr is a simple, open protocol for decentralized social media.
              </Text>
              <Link
                href="https://start.nostr.net/"
                isExternal
                color="blue.500"
                fontWeight="medium"
                _hover={{ textDecoration: 'underline' }}
              >
                start.nostr.net
              </Link>
            </Box>

            {IS_WEB_ANDROID && (
              <Box>
                <HStack mb={2}>
                  <Text fontSize="md" fontWeight="semibold">
                    Amber Signer
                  </Text>
                  <Badge colorScheme="yellow">Android</Badge>
                </HStack>
                <Text fontSize="md" mb={2}>
                  Amber is a secure key management app for Android. If you have Amber installed,
                  the app should have opened automatically.
                </Text>
                <Link
                  href="https://github.com/greenart7c3/Amber"
                  isExternal
                  color="blue.500"
                  fontWeight="medium"
                  _hover={{ textDecoration: 'underline' }}
                >
                  Get Amber from GitHub
                </Link>
              </Box>
            )}

            <Box>
              <Text fontSize="md" fontWeight="semibold" mb={2}>
                Web extension
              </Text>
              <Text fontSize="md">
                Requires web extension to login with existing user
              </Text>
            </Box>
          </VStack>
        </ModalBody>
      </ModalContent>
    </Modal>
  )
}
