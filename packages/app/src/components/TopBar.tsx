import { Box, Flex } from '@chakra-ui/react'
import { ProfileButton } from './ProfileButton'

interface TopBarProps {
  pubkey: string | null
  onLogin: (pubkey: string) => void
}

export function TopBar({ pubkey, onLogin }: TopBarProps) {
  return (
    <Box
      as="header"
      position="fixed"
      top={0}
      left={0}
      right={0}
      zIndex={1000}
      bg="rgba(0, 0, 0, 0.5)"
      backdropFilter="blur(8px)"
      borderBottom="1px solid rgba(255, 255, 255, 0.1)"
      px={3}
      py={2}
    >
      <Flex justify="space-between" align="center">
        <ProfileButton pubkey={pubkey} onLogin={onLogin} />
      </Flex>
    </Box>
  )
}
